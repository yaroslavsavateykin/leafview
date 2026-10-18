//! Latest-wins rendering mailbox and bounded raster cache.
//! One coordinator owns PDF rendering requests; Rayon (two workers) prefetches neighbors.
use crate::{document::Document, view::{crop_bounds, Viewport}};
use anyhow::{bail, Context, Result};
use image::{DynamicImage, ImageFormat};
use lru::LruCache;
use rayon::ThreadPool;
use std::{num::NonZeroUsize, path::Path, process::Command, sync::{atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering}, mpsc::Sender, Arc, Condvar, Mutex}, thread};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct Key { revision: u64, page: u32, width: u32 }
type RasterCache = Arc<Mutex<LruCache<Key, Arc<DynamicImage>>>>;

#[derive(Clone)]
pub struct Request {
    pub generation: u64,
    pub doc: Arc<Document>,
    pub page: u32,
    pub width: u32,
    pub viewport: Viewport,
    pub x: u32,
    pub y: u32,
}

pub struct Completed {
    pub generation: u64,
    pub image: Result<DynamicImage, String>,
    pub x: u32,
    pub y: u32,
}

#[derive(Default)]
struct Mailbox { latest: Option<Request>, closed: bool }

pub struct Renderer {
    inbox: Arc<(Mutex<Mailbox>, Condvar)>,
    generation: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Renderer {
    pub fn new(tx: Sender<Completed>) -> Result<Self> {
        let inbox = Arc::new((Mutex::new(Mailbox::default()), Condvar::new()));
        let generation = Arc::new(AtomicU64::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let cache: RasterCache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(4).unwrap())));
        let prefetch_pending = Arc::new(AtomicUsize::new(0));
        let pool = rayon::ThreadPoolBuilder::new().num_threads(2)
            .thread_name(|index| format!("leafview-prefetch-{index}"))
            .build().context("create Rayon pool")?;
        let (worker_inbox, worker_generation, worker_stopped) = (inbox.clone(), generation.clone(), stopped.clone());
        let worker = thread::spawn(move || {
            coordinate(worker_inbox, worker_generation, worker_stopped, cache, pool, prefetch_pending, tx);
        });
        Ok(Self { inbox, generation, stopped, worker: Some(worker) })
    }

    pub fn request(&self, mut request: Request) {
        // Atomic generation number ensures stale completions never override a newer page.
        request.generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (mutex, cv) = &*self.inbox;
        if let Ok(mut slot) = mutex.lock() {
            slot.latest = Some(request); // replace, don't enqueue unbounded work
            cv.notify_one();
        }
    }

    pub fn current_generation(&self) -> u64 { self.generation.load(Ordering::SeqCst) }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let (mutex, cv) = &*self.inbox;
        if let Ok(mut slot) = mutex.lock() { slot.closed = true; slot.latest = None; cv.notify_all(); }
        // A renderer can be inside a native subprocess. Do not freeze quit waiting for it:
        // the detached coordinator and bounded-time child finish without touching the TTY.
        let _ = self.worker.take();
    }
}

fn coordinate(inbox: Arc<(Mutex<Mailbox>, Condvar)>, generation: Arc<AtomicU64>, stopped: Arc<AtomicBool>, cache: RasterCache, pool: ThreadPool, prefetch_pending: Arc<AtomicUsize>, tx: Sender<Completed>) {
    loop {
        let req = {
            let (mutex, cv) = &*inbox;
            let mut slot = match mutex.lock() { Ok(s) => s, Err(_) => break };
            while slot.latest.is_none() && !slot.closed {
                slot = match cv.wait(slot) { Ok(s) => s, Err(_) => return };
            }
            if slot.closed { break; }
            slot.latest.take().expect("mailbox has a request")
        };
        if stopped.load(Ordering::Relaxed) { break; }
        if req.generation != generation.load(Ordering::SeqCst) { continue; }
        let key = Key { revision: req.doc.revision, page: req.page, width: req.width };
        let result = raster_cached(&cache, key, req.doc.snapshot_path()).map(|raster| {
            let (x,y,w,h) = crop_bounds(raster.width(),raster.height(),req.viewport,req.x,req.y);
            (raster.crop_imm(x,y,w,h), x, y)
        });
        if req.generation != generation.load(Ordering::SeqCst) { continue; }
        let success = result.is_ok();
        let (image, x, y) = match result {
            Ok((image,x,y)) => (Ok(image),x,y),
            Err(error) => (Err(format!("{error:#}")),req.x,req.y),
        };
        if tx.send(Completed { generation: req.generation, image, x, y }).is_err() { break; }
        if !success { continue; }

        // Prefetch only for the current generation. The two Rayon workers do independent
        // PDF renders; no native PDF context is shared between threads.
        for neighbor in [req.page.checked_sub(1), req.page.checked_add(1).filter(|&p| p < req.doc.pages)].into_iter().flatten() {
            if prefetch_pending.load(Ordering::SeqCst) >= 2 { continue; }
            prefetch_pending.fetch_add(1, Ordering::SeqCst);
            let prefetch_pending = prefetch_pending.clone();
            let (cache, generation, stopped, doc, width, request_generation) =
                (cache.clone(), generation.clone(), stopped.clone(), req.doc.clone(), req.width, req.generation);
            pool.spawn(move || {
                if !stopped.load(Ordering::Relaxed) && generation.load(Ordering::SeqCst) == request_generation {
                    let key = Key { revision: doc.revision, page: neighbor, width };
                    let _ = raster_cached(&cache, key, doc.snapshot_path());
                }
                prefetch_pending.fetch_sub(1, Ordering::SeqCst);
            });
        }
    }
}

fn raster_cached(cache: &RasterCache, key: Key, path: &Path) -> Result<Arc<DynamicImage>> {
    if let Some(image) = cache.lock().expect("cache mutex poisoned").get(&key).cloned() { return Ok(image); }
    let image = Arc::new(rasterize(path,key.page,key.width)?);
    cache.lock().expect("cache mutex poisoned").put(key,image.clone());
    Ok(image)
}

fn rasterize(path: &Path, page_zero_based: u32, width: u32) -> Result<DynamicImage> {
    let page = page_zero_based.checked_add(1).context("page index overflow")?;
    // GNU timeout bounds worst-case PDF rendering; the worker never writes to the terminal.
    let output = Command::new("timeout")
        .args(["--signal=KILL", "12s", "pdftoppm", "-f"]).arg(page.to_string())
        .args(["-l"]).arg(page.to_string())
        .args(["-singlefile", "-scale-to-x"]).arg(width.clamp(100,3400).to_string())
        .args(["-scale-to-y", "-1", "-png"])
        .arg(path).output().context("run Poppler (install poppler-utils and coreutils)")?;
    if !output.status.success() {
        bail!("pdftoppm failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    if output.stdout.len() > 80 * 1024 * 1024 { bail!("rendered page exceeds 80 MiB PNG limit"); }
    let image = image::load_from_memory_with_format(&output.stdout,ImageFormat::Png)
        .context("decode Poppler PNG")?;
    if image.width() as u64 * image.height() as u64 > 22_000_000 {
        bail!("PDF image too large (over 22 million pixels)");
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn independent_cache_keys() {
        assert_ne!(Key { revision: 1,page: 0,width: 800 },Key { revision: 1,page: 0,width: 1000 });
        assert_ne!(Key { revision: 1,page: 0,width: 800 },Key { revision: 2,page: 0,width: 800 });
    }
}

#[cfg(test)]
mod pdf_smoke_test {
    use super::*;
    #[test]
    fn rasterizes_fixture_as_image() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
        let img = rasterize(&path, 1, 800).unwrap();
        assert_eq!(img.width(), 800);
        assert!(img.height() > 800);
    }
}
