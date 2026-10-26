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
        cancelled: &dyn Fn() -> bool,
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
            if cancelled() || start.elapsed() > Duration::from_secs(60) {
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
    worker: Worker<(u64, crate::typst_options::Options), (u64, Result<Finished>)>,
    publication: Arc<std::sync::Mutex<()>>,
    options: crate::typst_options::Options,
    input: PathBuf,
    pub auto_compile: bool,
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
    pub fn new(
        input: &Path,
        options: crate::typst_options::Options,
        revision: u64,
    ) -> Result<Self> {
        crate::compile_config::validate(input, &options)?;
        let generation_counter = Arc::new(AtomicU64::new(revision + 1));
        let desired_generation = generation_counter.clone();
        let publication = Arc::new(std::sync::Mutex::new(()));
        let publication_worker = publication.clone();
        let initial_input = input.to_owned();
        let input = input.to_owned();
        let current_options = options.clone();
        let process = std::sync::Mutex::new((
            None::<CompilerProcess>,
            None::<crate::typst_options::Options>,
        ));
        let worker = Worker::new(
            move |(generation, options): (u64, crate::typst_options::Options), stop| {
                let result = (|| -> Result<Finished> {
                    let cancelled = || {
                        stop.load(Ordering::SeqCst)
                            || desired_generation.load(Ordering::SeqCst) != generation
                    };
                    anyhow::ensure!(!cancelled(), "superseded compilation");
                    let root = crate::compile_config::validate(&input, &options)?;
                    let mut state = process.lock().unwrap();
                    if state.1.as_ref() != Some(&options) {
                        state.0.take();
                        state.1 = Some(options.clone());
                    }
                    let process = &mut state.0;
                    if process.is_none() {
                        *process = Some(CompilerProcess::new(&input, &root, &options)?);
                    }
                    let result = process.as_mut().unwrap().compile(generation, &cancelled);
                    let (mut reply, output) = match result {
                        Ok(v) => v,
                        Err(e) => {
                            process.take();
                            return Err(e);
                        }
                    };
                    let document = if reply.error.is_none() {
                        Some(Document::open_cancel(&output, generation, cancelled)?)
                    } else {
                        None
                    };
                    let stale: Vec<_> = reply
                        .sources
                        .iter()
                        .filter(|(p, s)| std::fs::read_to_string(p).ok().as_ref() != Some(s))
                        .map(|(p, _)| p.clone())
                        .collect();
                    if !options.no_pdf
                        && reply.error.is_none()
                        && stale.is_empty()
                        && desired_generation.load(Ordering::SeqCst) == generation
                    {
                        if let Err(error) = publish_pdf(
                            &output,
                            &input.with_extension("pdf"),
                            &publication_worker,
                            || {
                                !stop.load(Ordering::SeqCst)
                                    && desired_generation.load(Ordering::SeqCst) == generation
                            },
                        ) {
                            reply.error = Some(format!("Cannot save PDF: {error:#}"));
                        }
                    }
                    Ok(Finished {
                        stale,
                        reply,
                        document,
                    })
                })();
                if result.is_err() {
                    process.lock().unwrap().0.take();
                }
                (generation, result)
            },
        );
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
            publication,
            options: current_options,
            input: initial_input.clone(),
            auto_compile: true,
            generation: generation_counter,
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
    pub fn configure(&mut self, settings: &crate::compile_config::Effective) -> Result<()> {
        crate::compile_config::validate(&self.input, &settings.options)?;
        let recompile =
            self.options != settings.options || self.auto_compile != settings.auto_compile;
        self.options = settings.options.clone();
        self.auto_sync = settings.auto_sync;
        self.auto_compile = settings.auto_compile;
        // Invalidate in-flight results immediately; the owned worker reaps its
        // old compiler before creating a new one, never on the UI thread.
        if recompile {
            self.refresh();
        }
        Ok(())
    }
    pub fn refresh(&mut self) {
        let _publication = self.publication.lock().unwrap();
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.due = Some(Instant::now());
        self.compiling = true;
    }
    pub fn poll(&mut self) -> Option<Update> {
        if self.overflow.swap(0, Ordering::Relaxed) > 0 && self.auto_compile {
            self.refresh();
            self.changed.extend(self.dependencies.iter().cloned());
        }
        while let Ok(path) = self.events.try_recv() {
            if self.dependencies.contains(&path) && self.auto_compile {
                self.changed.retain(|p| p != &path);
                self.changed.push(path);
                let _publication = self.publication.lock().unwrap();
                self.generation.fetch_add(1, Ordering::SeqCst);
                self.due = Some(Instant::now() + Duration::from_millis(200));
                self.compiling = true;
            }
        }
        let mut update = None;
        if let Ok((generation, result)) = self.worker.output.try_recv() {
            self.pending = false;
            match result {
                _ if generation != self.generation.load(Ordering::SeqCst) => {}
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
                        .chain(self.dependencies.iter().filter(|_| reply.error.is_some()))
                        .chain(std::iter::once(&self.input))
                        .filter_map(|p| p.parent().map(Path::to_owned))
                        .collect();
                    let removed: Vec<_> = self.directories.difference(&wanted).cloned().collect();
                    for path in removed {
                        let _ = self.watcher.unwatch(&path);
                        self.directories.remove(&path);
                    }
                    if reply.error.is_none() {
                        self.dependencies.clear();
                    }
                    self.dependencies.extend(reply.dependencies.iter().cloned());
                    self.dependencies.insert(self.input.clone());
                    for path in &self.dependencies {
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
                .try_send((self.generation.load(Ordering::SeqCst), self.options.clone()))
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
/// Stage in the destination directory: rename never exposes a partial PDF.
fn publish_pdf(
    source: &Path,
    destination: &Path,
    publication: &std::sync::Mutex<()>,
    current: impl Fn() -> bool,
) -> Result<()> {
    let mut staged =
        tempfile::NamedTempFile::new_in(destination.parent().context("PDF has no parent")?)?;
    let mut input = std::fs::File::open(source)?;
    std::io::copy(&mut input, &mut staged)?;
    staged.flush()?;
    // Serialize only the final generation check + rename, never the file copy.
    let _publication = publication.lock().unwrap();
    if current() {
        staged.persist(destination).map_err(|e| e.error)?;
    }
    Ok(())
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
    fn atomic_pdf_publication_rejects_stale_result() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("staged.pdf");
        let target = dir.path().join("main.pdf");
        std::fs::write(&source, b"new PDF").unwrap();
        std::fs::write(&target, b"old PDF").unwrap();
        publish_pdf(&source, &target, &std::sync::Mutex::new(()), || false).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"old PDF");
        publish_pdf(&source, &target, &std::sync::Mutex::new(()), || true).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new PDF");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
    #[test]
    fn utf8_and_last_change() {
        assert_eq!(changed_range("абв", "абXв"), Some(4..5));
        assert_eq!(changed_range("abc", "ac"), Some(1..1));
        assert_eq!(changed_range("abc def", "Abc deF"), Some(6..7));
        assert_eq!(changed_range("same", "same"), None);
    }
}
