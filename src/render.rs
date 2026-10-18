//! Single-slot latest-wins mailbox, byte-bounded cache and cancellable Poppler jobs.
use crate::{
    document::Document,
    view::{crop_bounds, Viewport},
};
use anyhow::{Context, Result};
use image::{DynamicImage, ImageReader, Limits};
use lru::LruCache;
use std::{
    io::{BufReader, Seek, SeekFrom},
    num::NonZeroUsize,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::SyncSender,
        Arc, Condvar, Mutex,
    },
    thread,
};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct Key {
    revision: u64,
    page: u32,
    width: u32,
}
struct Cache {
    images: LruCache<Key, Arc<DynamicImage>>,
    bytes: usize,
    limit: usize,
}
impl Cache {
    fn new(limit: usize) -> Self {
        Self {
            images: LruCache::new(NonZeroUsize::new(8).unwrap()),
            bytes: 0,
            limit,
        }
    }
    // Speculation must never evict a useful raster, especially the visible page.
    fn insert_prefetched(&mut self, key: Key, image: Arc<DynamicImage>) {
        if !self.images.contains(&key)
            && self.images.len() < 8
            && self.bytes.saturating_add(image.as_bytes().len()) <= self.limit
        {
            self.insert(key, image);
        }
    }
    fn insert(&mut self, key: Key, image: Arc<DynamicImage>) {
        if let Some(old) = self.images.pop(&key) {
            self.bytes -= old.as_bytes().len();
        }
        let size = image.as_bytes().len();
        if size > self.limit {
            return;
        }
        while self.bytes + size > self.limit || self.images.len() == 8 {
            if let Some((_, old)) = self.images.pop_lru() {
                self.bytes -= old.as_bytes().len();
            } else {
                break;
            }
        }
        self.bytes += size;
        self.images.put(key, image);
    }
}
#[derive(Clone)]
pub struct Request {
    pub generation: u64,
    pub doc: Arc<Document>,
    pub page: u32,
    pub width: u32,
    pub viewport: Viewport,
    pub x: u32,
    pub y: u32,
    pub destination: Option<crate::links::Destination>,
}
pub struct Completed {
    pub geometry: Option<crate::links::RenderGeometry>,
    pub generation: u64,
    pub image: Result<DynamicImage, String>,
    pub x: u32,
    pub y: u32,
}
#[derive(Default)]
struct Mailbox {
    latest: Option<Request>,
    closed: bool,
}
pub struct Renderer {
    inbox: Arc<(Mutex<Mailbox>, Condvar)>,
    generation: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Renderer {
    pub fn new(tx: SyncSender<Completed>) -> Result<Self> {
        let inbox = Arc::new((Mutex::new(Mailbox::default()), Condvar::new()));
        let generation = Arc::new(AtomicU64::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .thread_name(|i| format!("leafview-raster-{i}"))
            .build()?;
        let (wi, wg, ws) = (inbox.clone(), generation.clone(), stopped.clone());
        let worker = thread::spawn(move || {
            let cache = Mutex::new(Cache::new(96 * 1024 * 1024));
            loop {
                let req = {
                    let (lock, cv) = &*wi;
                    let mut slot = lock.lock().unwrap();
                    while slot.latest.is_none() && !slot.closed {
                        slot = cv.wait(slot).unwrap();
                    }
                    if slot.closed {
                        break;
                    }
                    slot.latest.take().unwrap()
                };
                let cancelled =
                    || ws.load(Ordering::SeqCst) || wg.load(Ordering::SeqCst) != req.generation;
                if cancelled() {
                    continue;
                }
                let key = Key {
                    revision: req.doc.revision,
                    page: req.page,
                    width: req.width,
                };
                let result = cached(&cache, key, req.doc.snapshot_path(), &cancelled).map(|r| {
                    let (sx, sy) = req.destination.as_ref().map_or((req.x, req.y), |d| {
                        d.offsets((r.width(), r.height()), (req.x, req.y))
                    });
                    let (x, y, w, h) = crop_bounds(r.width(), r.height(), req.viewport, sx, sy);
                    let geometry = crate::links::RenderGeometry {
                        revision: req.doc.revision,
                        page: req.page,
                        generation: req.generation,
                        raster: (r.width(), r.height()),
                        crop: (x, y, w, h),
                        links: req
                            .doc
                            .links
                            .pages
                            .get(req.page as usize)
                            .cloned()
                            .flatten(),
                    };
                    (r.crop_imm(x, y, w, h), x, y, geometry)
                });
                if cancelled() {
                    continue;
                }
                let success = result.is_ok();
                let (image, x, y, geometry) = match result {
                    Ok((img, x, y, geometry)) => (Ok(img), x, y, Some(geometry)),
                    Err(e) => (Err(format!("{e:#}")), req.x, req.y, None),
                };
                // A full result slot is harmless: the UI will drain it and reschedule.
                // Retrying is cancellable and cannot block shutdown.
                let mut completed = Completed {
                    geometry,
                    generation: req.generation,
                    image,
                    x,
                    y,
                };
                loop {
                    match tx.try_send(completed) {
                        Ok(()) => break,
                        Err(std::sync::mpsc::TrySendError::Full(value)) => {
                            completed = value;
                            if cancelled() {
                                break;
                            }
                            thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(_) => return,
                    }
                }
                if !success || cancelled() {
                    continue;
                }
                // Release obsolete speculative rasters to make room as the window moves.
                {
                    let mut cache = cache.lock().unwrap();
                    let obsolete: Vec<_> = cache
                        .images
                        .iter()
                        .map(|(&k, _)| k)
                        .filter(|k| {
                            k.revision != key.revision
                                || k.width != key.width
                                || k.page.abs_diff(key.page) > 3
                        })
                        .collect();
                    for old in obsolete {
                        if let Some(image) = cache.images.pop(&old) {
                            cache.bytes -= image.as_bytes().len();
                        }
                    }
                }
                // Complete each distance before starting the next: Rayon scopes alone
                // do not guarantee FIFO priority. At most two renders run concurrently.
                for distance in 1..=3 {
                    if cancelled() {
                        break;
                    }
                    pool.scope(|scope| {
                        for page in neighbors(req.page, req.doc.pages, distance)
                            .into_iter()
                            .flatten()
                        {
                            let cache = &cache;
                            let cancelled = &cancelled;
                            let path = req.doc.snapshot_path();
                            scope.spawn(move |_| {
                                let _ = prefetch(cache, Key { page, ..key }, key, path, cancelled);
                            });
                        }
                    });
                }
            }
        });
        Ok(Self {
            inbox,
            generation,
            stopped,
            worker: Some(worker),
        })
    }
    pub fn request(&self, mut request: Request) {
        request.generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (lock, cv) = &*self.inbox;
        let mut slot = lock.lock().unwrap();
        slot.latest = Some(request);
        cv.notify_one();
    }
    pub fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let (lock, cv) = &*self.inbox;
        {
            let mut slot = lock.lock().unwrap();
            slot.closed = true;
            slot.latest = None;
            cv.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn neighbors(page: u32, pages: u32, distance: u32) -> [Option<u32>; 2] {
    [
        page.checked_add(distance).filter(|&p| p < pages),
        page.checked_sub(distance),
    ]
}

fn prefetch(
    cache: &Mutex<Cache>,
    key: Key,
    current: Key,
    path: &Path,
    cancelled: &(impl Fn() -> bool + Sync),
) -> Result<()> {
    if cancelled() {
        return Ok(());
    }
    {
        let cache = cache.lock().unwrap();
        if cache.images.contains(&key) {
            return Ok(());
        }
        // Use the current raster as a conservative estimate before doing work.
        // Exact admission below also handles mixed-size/orientation pages.
        let estimate = cache
            .images
            .peek(&current)
            .map_or(0, |img| img.as_bytes().len());
        if cache.images.len() == 8 || cache.bytes.saturating_add(estimate) > cache.limit {
            return Ok(());
        }
    }
    let image = Arc::new(rasterize(path, key.page, key.width, cancelled)?);
    if !cancelled() {
        cache.lock().unwrap().insert_prefetched(key, image);
    }
    Ok(())
}

fn cached(
    cache: &Mutex<Cache>,
    key: Key,
    path: &Path,
    cancelled: &(impl Fn() -> bool + Sync),
) -> Result<Arc<DynamicImage>> {
    if let Some(image) = cache.lock().unwrap().images.get(&key).cloned() {
        return Ok(image);
    }
    let image = Arc::new(rasterize(path, key.page, key.width, cancelled)?);
    if !cancelled() {
        cache.lock().unwrap().insert(key, image.clone());
    }
    Ok(image)
}
fn rasterize(
    path: &Path,
    page: u32,
    width: u32,
    cancelled: impl Fn() -> bool,
) -> Result<DynamicImage> {
    let page = page.checked_add(1).context("page overflow")?.to_string();
    let mut command = Command::new("pdftoppm");
    command
        .args([
            "-f",
            &page,
            "-l",
            &page,
            "-singlefile",
            "-scale-to-x",
            &width.clamp(100, 3400).to_string(),
            "-scale-to-y",
            "-1",
            "-png",
        ])
        .arg(path);
    let mut output = crate::process::run(&mut command, 80 * 1024 * 1024, &cancelled)?;
    output.seek(SeekFrom::Start(0))?;
    let mut reader = ImageReader::new(BufReader::new(output)).with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(3400);
    limits.max_image_height = Some(12000);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().context("decode bounded Poppler PNG")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefetch_bounds_and_budget() {
        assert_eq!(neighbors(0, 10, 1), [Some(1), None]);
        assert_eq!(neighbors(9, 10, 3), [None, Some(6)]);
        assert_eq!(neighbors(4, 10, 3), [Some(7), Some(1)]);
        assert_eq!(neighbors(u32::MAX, u32::MAX, 1), [None, Some(u32::MAX - 1)]);
        let img = Arc::new(DynamicImage::new_rgb8(10, 10));
        let size = img.as_bytes().len();
        let key = Key {
            revision: 1,
            page: 0,
            width: 10,
        };
        let mut cache = Cache::new(size * 7);
        cache.insert(key, img.clone());
        for page in 1..10 {
            cache.insert_prefetched(Key { page, ..key }, img.clone());
        }
        assert_eq!(cache.images.len(), 7);
        assert_eq!(cache.bytes, size * 7);
        assert!(cache.images.contains(&key));
        assert!(cache.images.contains(&Key { page: 1, ..key }));
        assert!(!cache.images.contains(&Key { page: 7, ..key }));
    }
    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf")
    }
    #[test]
    #[ignore = "generate /tmp/leafview-bench using scripts/make-bench-fixtures.py first"]
    fn profile_synthetic_pdfs() {
        use std::time::Instant;
        for name in ["small", "long", "vector", "raster", "math"] {
            let path = std::path::PathBuf::from(format!("/tmp/leafview-bench/{name}.pdf"));
            let start = Instant::now();
            let doc = Document::open(&path, 1).unwrap();
            let open = start.elapsed();
            let cache = Mutex::new(Cache::new(96 * 1024 * 1024));
            let key = Key {
                revision: 1,
                page: 0,
                width: 1000,
            };
            let start = Instant::now();
            let raster = cached(&cache, key, doc.snapshot_path(), &|| false).unwrap();
            let render_decode = start.elapsed();
            let start = Instant::now();
            for _ in 0..1000 {
                assert!(Arc::ptr_eq(
                    &raster,
                    &cached(&cache, key, doc.snapshot_path(), &|| false).unwrap()
                ));
            }
            let hits = start.elapsed();
            let start = Instant::now();
            let crop = raster.crop_imm(0, 0, 1000, raster.height().min(700));
            let crop_time = start.elapsed();
            let start = Instant::now();
            let _ = ratatui_image::protocol::kitty::Kitty::new(
                crop,
                ratatui::layout::Size::new(100, 35),
                123,
                false,
            )
            .unwrap();
            println!("{name}: pages={} open={open:?} render+decode={render_decode:?} 1000 cache hits={hits:?} crop={crop_time:?} kitty encode={:?}",doc.pages,start.elapsed());
        }
    }
    #[test]
    fn annotation_geometry_matches_poppler_pixels() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/links.pdf");
        let index = crate::links::LinkIndex::extract(&path).unwrap();
        for page in [0, 2, 3, 4] {
            let raster = rasterize(&path, page, 600, || false).unwrap().to_rgb8();
            let links = index.pages[page as usize].as_ref().unwrap();
            let r = links.links[0].bounds;
            let (u, v) = links
                .space
                .to_display((r.x0 + r.x1) / 2., (r.y0 + r.y1) / 2.);
            let p = raster.get_pixel(
                (u * raster.width() as f64) as u32,
                (v * raster.height() as f64) as u32,
            );
            assert!(
                p[0] > 240 && p[1] < 20 && p[2] < 20,
                "rotation {}: {p:?}",
                links.space.rotation
            );
        }
    }
    #[test]
    fn raster_and_cache() {
        let img = Arc::new(rasterize(&fixture(), 1, 800, || false).unwrap());
        assert_eq!(img.width(), 800);
        assert!(img.height() > 800);
        let size = img.as_bytes().len();
        let mut cache = Cache::new(size);
        let key = Key {
            revision: 1,
            page: 0,
            width: 800,
        };
        cache.insert(key, img.clone());
        cache.insert(Key { page: 1, ..key }, img);
        assert_eq!(cache.bytes, size);
        assert!(!cache.images.contains(&key));
    }
    #[test]
    fn latest_wins_and_shutdown() {
        let doc = Arc::new(Document::open(&fixture(), 1).unwrap());
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let renderer = Renderer::new(tx).unwrap();
        for i in 0..1000 {
            renderer.request(Request {
                generation: 0,
                destination: None,
                doc: doc.clone(),
                page: i % 3,
                width: 400,
                viewport: Viewport {
                    width: 400,
                    height: 600,
                },
                x: 0,
                y: 0,
            });
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let result = rx
                .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .unwrap();
            if result.generation == renderer.current_generation() {
                assert!(result.image.is_ok());
                break;
            }
        }
        let start = std::time::Instant::now();
        drop(renderer);
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }
}
