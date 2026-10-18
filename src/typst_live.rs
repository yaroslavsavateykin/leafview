//! Owned incremental compiler, dependency watcher and deterministic source diff.
use crate::{document::Document, workers::Worker};
use anyhow::{Context, Result};
use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
pub struct CompileRequest {
    pub generation: u64,
    pub output: PathBuf,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SourcePoint {
    pub file: PathBuf,
    pub start: usize,
    pub end: usize,
    pub page: u32,
    pub x: f64,
    pub y: f64,
}
#[derive(Serialize, Deserialize)]
pub struct CompileReply {
    pub generation: u64,
    pub dependencies: Vec<PathBuf>,
    pub sources: BTreeMap<PathBuf, String>,
    pub points: Vec<SourcePoint>,
    pub error: Option<String>,
    pub millis: u64,
}
pub fn reply_path(output: &Path) -> PathBuf {
    output.with_extension("json")
}
struct CompilerProcess {
    child: Child,
    input: ChildStdin,
    directory: tempfile::TempDir,
}
impl Drop for CompilerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl CompilerProcess {
    fn new(input: &Path, root: &Path, options: &crate::typst_options::Options) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let mut child = Command::new("prlimit")
            .args(["--as=3221225472", "--"])
            .arg(std::env::current_exe()?)
            .arg("--typst-compiler")
            .arg(input)
            .arg(root)
            .arg(serde_json::to_string(options)?)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child.stdin.take().context("compiler stdin")?;
        Ok(Self {
            child,
            input,
            directory,
        })
    }
    fn compile(
        &mut self,
        generation: u64,
        stop: &std::sync::atomic::AtomicBool,
    ) -> Result<(CompileReply, PathBuf)> {
        let output = self.directory.path().join("document.pdf");
        let reply = reply_path(&output);
        if reply.exists() {
            std::fs::remove_file(&reply)?;
        }
        serde_json::to_writer(
            &mut self.input,
            &CompileRequest {
                generation,
                output: output.clone(),
            },
        )?;
        self.input.write_all(b"\n")?;
        self.input.flush()?;
        let start = Instant::now();
        while !reply.exists() {
            if stop.load(Ordering::SeqCst) || start.elapsed() > Duration::from_secs(60) {
                let _ = self.child.kill();
                anyhow::bail!("Typst compilation cancelled or timed out");
            }
            anyhow::ensure!(
                self.child.try_wait()?.is_none(),
                "Typst compiler stopped (resource limit or internal error)"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        anyhow::ensure!(
            std::fs::metadata(&reply)?.len() <= 96 * 1024 * 1024,
            "source map exceeds 96 MiB"
        );
        let result = serde_json::from_reader(std::fs::File::open(reply)?)?;
        Ok((result, output))
    }
}
pub struct Update {
    pub generation: u64,
    pub document: Option<Document>,
    pub location: Option<SourcePoint>,
    pub error: Option<String>,
    pub millis: u64,
}
struct Finished {
    stale: Vec<PathBuf>,
    reply: CompileReply,
    document: Option<Document>,
}
pub struct Live {
    worker: Worker<u64, Result<Finished>>,
    generation: Arc<AtomicU64>,
    pending: bool,
    due: Option<Instant>,
    watcher: notify::RecommendedWatcher,
    events: mpsc::Receiver<PathBuf>,
    directories: BTreeSet<PathBuf>,
    dependencies: BTreeSet<PathBuf>,
    sources: BTreeMap<PathBuf, String>,
    changed: Vec<PathBuf>,
    overflow: Arc<AtomicU64>,
    pub auto_sync: bool,
    pub last: Option<SourcePoint>,
    pub compiling: bool,
}
impl Live {
    pub fn new(input: &Path, options: crate::typst_options::Options) -> Result<Self> {
        let root = options
            .root
            .clone()
            .unwrap_or_else(|| input.parent().unwrap().to_owned())
            .canonicalize()?;
        anyhow::ensure!(
            input.starts_with(&root),
            "Typst input must be within TYPST_ROOT"
        );
        let initial_input = input.to_owned();
        let input = input.to_owned();
        let process = std::sync::Mutex::new(None::<CompilerProcess>);
        let worker = Worker::new(move |generation, stop| {
            let mut process = process.lock().unwrap();
            if process.is_none() {
                *process = Some(CompilerProcess::new(&input, &root, &options)?);
            }
            let result = process.as_mut().unwrap().compile(generation, stop);
            let (reply, output) = match result {
                Ok(v) => v,
                Err(e) => {
                    process.take();
                    return Err(e);
                }
            };
            let document = if reply.error.is_none() {
                Some(Document::open_cancel(&output, generation, || {
                    stop.load(Ordering::SeqCst)
                })?)
            } else {
                None
            };
            let stale = reply
                .sources
                .iter()
                .filter(|(p, s)| std::fs::read_to_string(p).ok().as_ref() != Some(s))
                .map(|(p, _)| p.clone())
                .collect();
            Ok(Finished {
                stale,
                reply,
                document,
            })
        });
        let (tx, events) = mpsc::sync_channel(256);
        let overflow = Arc::new(AtomicU64::new(0));
        let overflow_events = overflow.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if let Ok(event) = event {
                    if !matches!(event.kind, notify::EventKind::Access(_)) {
                        for path in event.paths {
                            if tx.try_send(path).is_err() {
                                overflow_events.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            })?;
        watcher.watch(
            initial_input.parent().unwrap(),
            notify::RecursiveMode::NonRecursive,
        )?;
        Ok(Self {
            worker,
            generation: Arc::new(AtomicU64::new(1)),
            pending: false,
            due: Some(Instant::now()),
            watcher,
            events,
            directories: BTreeSet::from([initial_input.parent().unwrap().to_owned()]),
            dependencies: BTreeSet::from([initial_input]),
            sources: BTreeMap::new(),
            changed: Vec::new(),
            overflow,
            auto_sync: true,
            last: None,
            compiling: true,
        })
    }
    pub fn refresh(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.due = Some(Instant::now());
        self.compiling = true;
    }
    pub fn poll(&mut self) -> Option<Update> {
        if self.overflow.swap(0, Ordering::Relaxed) > 0 {
            self.refresh();
            self.changed.extend(self.dependencies.iter().cloned());
        }
        while let Ok(path) = self.events.try_recv() {
            if self.dependencies.contains(&path) {
                self.changed.retain(|p| p != &path);
                self.changed.push(path);
                self.generation.fetch_add(1, Ordering::SeqCst);
                self.due = Some(Instant::now() + Duration::from_millis(200));
                self.compiling = true;
            }
        }
        let mut update = None;
        if let Ok(result) = self.worker.output.try_recv() {
            self.pending = false;
            match result {
                Err(e) => {
                    self.compiling = false;
                    update = Some(Update {
                        generation: self.generation.load(Ordering::SeqCst),
                        document: None,
                        location: None,
                        error: Some(format!("Typst: {e:#}")),
                        millis: 0,
                    });
                }
                Ok(finished) => {
                    let reply = finished.reply;
                    // Watch dependency parents (atomic rename safe), never a recursive home tree.
                    for path in &reply.dependencies {
                        if let Some(parent) = path.parent() {
                            if !self.directories.contains(parent)
                                && self
                                    .watcher
                                    .watch(parent, notify::RecursiveMode::NonRecursive)
                                    .is_ok()
                            {
                                self.directories.insert(parent.to_owned());
                            }
                        }
                    }
                    let wanted: BTreeSet<_> = reply
                        .dependencies
                        .iter()
                        .filter_map(|p| p.parent().map(Path::to_owned))
                        .collect();
                    let removed: Vec<_> = self.directories.difference(&wanted).cloned().collect();
                    for path in removed {
                        let _ = self.watcher.unwatch(&path);
                        self.directories.remove(&path);
                    }
                    self.dependencies = reply.dependencies.iter().cloned().collect();
                    // A dependency may have changed before its parent was watched.
                    // Reject snapshots whose compiled source bytes are no longer current.
                    let stale = finished.stale;
                    if !stale.is_empty() {
                        self.changed.extend(stale);
                        self.refresh();
                    }
                    if reply.generation == self.generation.load(Ordering::SeqCst) {
                        self.compiling = false;
                        let location = if reply.error.is_none() {
                            self.resolve(&reply)
                        } else {
                            None
                        };
                        if reply.error.is_none() {
                            self.sources = reply.sources;
                            self.changed.clear();
                            self.last = location.clone();
                        }
                        update = Some(Update {
                            generation: reply.generation,
                            document: finished.document,
                            location,
                            error: reply.error.map(|e| format!("Typst: {e}")),
                            millis: reply.millis,
                        });
                    }
                }
            }
        }
        if !self.pending
            && self.due.is_some_and(|t| Instant::now() >= t)
            && self
                .worker
                .input
                .as_ref()
                .unwrap()
                .try_send(self.generation.load(Ordering::SeqCst))
                .is_ok()
        {
            self.pending = true;
            self.due = None;
        }
        update
    }
    fn resolve(&self, reply: &CompileReply) -> Option<SourcePoint> {
        for path in self.changed.iter().rev() {
            let (Some(old), Some(new)) = (self.sources.get(path), reply.sources.get(path)) else {
                return None;
            };
            let Some(range) = changed_range(old, new) else {
                continue;
            };
            let candidates: Vec<_> = reply
                .points
                .iter()
                .filter(|p| &p.file == path && p.start <= range.end && p.end > range.start)
                .collect();
            // Ambiguous repeated content is not a reason to jump to an arbitrary page.
            if let Some(first) = candidates.first() {
                if candidates.iter().all(|p| p.page == first.page) {
                    return Some((*first).clone());
                }
            }
            if range.is_empty() {
                let closest = reply
                    .points
                    .iter()
                    .filter(|p| &p.file == path)
                    .min_by_key(|p| p.start.abs_diff(range.start));
                if let Some(point) = closest {
                    return Some(point.clone());
                }
            }
            return None;
        }
        None
    }
}
/// Linear, UTF-8-safe changed envelope; the last differing suffix is selected.
/// Choosing the last changed character gives deterministic last-hunk semantics
/// even when a save contains several disjoint edits, without quadratic diff RAM.
pub fn changed_range(old: &str, new: &str) -> Option<std::ops::Range<usize>> {
    if old == new {
        return None;
    }
    let prefix = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let suffix = old[prefix..]
        .chars()
        .rev()
        .zip(new[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let end = new.len() - suffix;
    let start = new[..end]
        .char_indices()
        .next_back()
        .map_or(end, |(i, _)| i)
        .max(prefix);
    Some(start..end)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf8_and_last_change() {
        assert_eq!(changed_range("абв", "абXв"), Some(4..5));
        assert_eq!(changed_range("abc", "ac"), Some(1..1));
        assert_eq!(changed_range("abc def", "Abc deF"), Some(6..7));
        assert_eq!(changed_range("same", "same"), None);
    }
}
