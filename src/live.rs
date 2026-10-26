//! Owned incremental compiler, dependency watcher and deterministic source diff.
use crate::{document::Document, workers::Worker};
use anyhow::{Context, Result};
use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

#[cfg(feature = "typst")]
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
    #[serde(default)]
    pub stale: Vec<PathBuf>,
    #[serde(default)]
    pub synctex_coordinates: bool,
}
#[cfg(feature = "typst")]
pub fn reply_path(output: &Path) -> PathBuf {
    output.with_extension("json")
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
    worker: Worker<(u64, BTreeMap<PathBuf, String>), Result<Finished>>,
    label: &'static str,
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
        no_pdf: bool,
        label: &'static str,
        mut compile: impl FnMut(
                u64,
                &dyn Fn() -> bool,
                &BTreeMap<PathBuf, String>,
            ) -> Result<(CompileReply, PathBuf)>
            + Send
            + 'static,
    ) -> Result<Self> {
        let generation_counter = Arc::new(AtomicU64::new(1));
        let desired_generation = generation_counter.clone();
        let initial_input = input.to_owned();
        let input = input.to_owned();
        let compile = std::sync::Mutex::new(
            move |generation, cancelled: &dyn Fn() -> bool, sources: &BTreeMap<PathBuf, String>| {
                compile(generation, cancelled, sources)
            },
        );
        let worker = Worker::new(move |(generation, sources), stop| {
            let cancelled = || {
                stop.load(Ordering::SeqCst)
                    || desired_generation.load(Ordering::SeqCst) != generation
            };
            let (mut reply, output) = compile.lock().unwrap()(generation, &cancelled, &sources)?;
            let document = if reply.error.is_none() {
                Some(Document::open_cancel(&output, generation, || {
                    stop.load(Ordering::SeqCst)
                })?)
            } else {
                None
            };
            if reply.synctex_coordinates {
                reply.points.retain_mut(|p| {
                    let Some(space) = document
                        .as_ref()
                        .and_then(|d| d.links.pages.get(p.page as usize))
                        .and_then(|p| p.as_ref())
                        .map(|p| p.space)
                    else {
                        return false;
                    };
                    let (x, y) = space.to_display(space.media.x0 + p.x, space.media.y1 - p.y);
                    if !(0. ..=1.).contains(&x) || !(0. ..=1.).contains(&y) {
                        return false;
                    }
                    p.x = x;
                    p.y = y;
                    true
                });
            }
            let mut stale: Vec<_> = reply
                .sources
                .iter()
                .filter(|(p, s)| std::fs::read_to_string(p).ok().as_ref() != Some(s))
                .map(|(p, _)| p.clone())
                .collect();
            stale.append(&mut reply.stale);
            if !no_pdf
                && reply.error.is_none()
                && stale.is_empty()
                && desired_generation.load(Ordering::SeqCst) == generation
            {
                if let Err(error) = publish_pdf(&output, &input.with_extension("pdf"), || {
                    !stop.load(Ordering::SeqCst)
                        && desired_generation.load(Ordering::SeqCst) == generation
                }) {
                    reply.error = Some(format!("Cannot save PDF: {error:#}"));
                }
            }
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
            label,
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
                    self.compiling = self.due.is_some();
                    if self.due.is_none() {
                        update = Some(Update {
                            generation: self.generation.load(Ordering::SeqCst),
                            document: None,
                            location: None,
                            error: Some(format!("{}: {e:#}", self.label)),
                            millis: 0,
                        });
                    }
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
                            error: reply.error.map(|e| format!("{}: {e}", self.label)),
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
                .try_send((self.generation.load(Ordering::SeqCst), self.sources.clone()))
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
fn publish_pdf(source: &Path, destination: &Path, current: impl Fn() -> bool) -> Result<()> {
    let mut staged =
        tempfile::NamedTempFile::new_in(destination.parent().context("PDF has no parent")?)?;
    let mut input = std::fs::File::open(source)?;
    std::io::copy(&mut input, &mut staged)?;
    staged.flush()?;
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
        publish_pdf(&source, &target, || false).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"old PDF");
        publish_pdf(&source, &target, || true).unwrap();
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
