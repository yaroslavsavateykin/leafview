//! Immutable snapshots; failed reloads never replace the current document.
use anyhow::{bail, Context, Result};
use std::{
    hash::{Hash, Hasher},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    process::Command,
};
use tempfile::NamedTempFile;

pub struct Document {
    snapshot: NamedTempFile,
    pub pages: u32,
    pub revision: u64,
    /// Process-unique identity of this immutable snapshot (revision is caller-local).
    pub id: u64,
    pub links: crate::links::LinkIndex,
    fallback_aspect: f64,
}
impl Document {
    pub fn open(source: &Path, revision: u64) -> Result<Self> {
        Self::open_cancel(source, revision, || false)
    }
    pub fn open_cancel(source: &Path, revision: u64, cancelled: impl Fn() -> bool) -> Result<Self> {
        let mut input = std::fs::File::open(source).context("PDF file not found")?;
        let before = input.metadata()?;
        if before.len() > 512 * 1024 * 1024 {
            bail!("PDF exceeds 512 MiB snapshot limit");
        }
        let mut snapshot = NamedTempFile::new()?;
        let mut buffer = vec![0; 64 * 1024];
        let mut copied = 0;
        let mut fingerprint = std::collections::hash_map::DefaultHasher::new();
        loop {
            if cancelled() {
                bail!("cancelled");
            }
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            copied += count as u64;
            if copied > 512 * 1024 * 1024 {
                bail!("PDF exceeds snapshot limit");
            }
            fingerprint.write(&buffer[..count]);
            snapshot.write_all(&buffer[..count])?;
        }
        let after = input.metadata()?;
        if before.len() != after.len() || before.modified()? != after.modified()? {
            bail!("PDF changed during snapshot; waiting for next save");
        }
        let mut output = crate::process::run(
            Command::new("pdfinfo").arg(snapshot.path()),
            65536,
            &cancelled,
        )?;
        output.seek(SeekFrom::Start(0))?;
        let mut info = String::new();
        output.read_to_string(&mut info)?;
        let pages = info
            .lines()
            .find_map(|line| {
                line.strip_prefix("Pages:")
                    .and_then(|n| n.trim().parse::<u32>().ok())
            })
            .filter(|&n| n > 0)
            .context("pdfinfo did not report a valid page count")?;
        let fallback_aspect = info
            .lines()
            .find_map(|line| {
                let size = line.strip_prefix("Page size:")?;
                let mut numbers = size.split_whitespace();
                let width = numbers.next()?.parse::<f64>().ok()?;
                if numbers.next()? != "x" {
                    return None;
                }
                let height = numbers.next()?.parse::<f64>().ok()?;
                (width > 0. && height > 0.).then_some(height / width)
            })
            .unwrap_or(297. / 210.);
        copied.hash(&mut fingerprint);
        // Cache by immutable PDF contents, not path/mtime: Typst writes a new
        // temporary path for every compilation, and atomic saves reuse names.
        static CACHE: std::sync::OnceLock<
            std::sync::Mutex<lru::LruCache<u64, crate::links::LinkIndex>>,
        > = std::sync::OnceLock::new();
        let cache = CACHE.get_or_init(|| {
            std::sync::Mutex::new(lru::LruCache::new(std::num::NonZeroUsize::new(8).unwrap()))
        });
        let key = fingerprint.finish();
        let cached = cache.lock().unwrap().get(&key).cloned();
        let links = if let Some(links) = cached {
            links
        } else {
            let links = crate::links::LinkIndex::load(snapshot.path(), &cancelled);
            if links.error.is_none() && !cancelled() {
                cache.lock().unwrap().put(key, links.clone());
            }
            links
        };
        static NEXT_SNAPSHOT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Ok(Self {
            id: NEXT_SNAPSHOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            links,
            snapshot,
            pages,
            revision,
            fallback_aspect,
        })
    }
    pub fn page_aspect(&self, page: u32) -> f64 {
        self.links
            .pages
            .get(page as usize)
            .and_then(Option::as_ref)
            .map(|p| {
                let (w, h) = p.space.size();
                h / w
            })
            .filter(|a| a.is_finite() && *a > 0.)
            .unwrap_or(self.fallback_aspect)
    }
    pub fn snapshot_path(&self) -> &Path {
        self.snapshot.path()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_survives_replacement() {
        let source = NamedTempFile::new().unwrap();
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf"),
            source.path(),
        )
        .unwrap();
        let doc = Document::open(source.path(), 1).unwrap();
        assert_eq!(doc.pages, 3);
        std::fs::write(source.path(), b"broken").unwrap();
        assert!(Document::open(source.path(), 2).is_err());
        assert!(Document::open(doc.snapshot_path(), 3).is_ok());
    }
    #[test]
    fn missing_and_cancelled() {
        assert!(Document::open(Path::new("/nonexistent/leafview.pdf"), 0).is_err());
        assert!(Document::open_cancel(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf"),
            0,
            || true
        )
        .is_err());
    }
}
