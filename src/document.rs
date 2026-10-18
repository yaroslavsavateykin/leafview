//! Immutable on-disk PDF snapshots: a partially-written Typst PDF never replaces the live one.
use anyhow::{bail, Context, Result};
use std::{path::Path, process::Command};
use tempfile::NamedTempFile;

pub struct Document {
    snapshot: NamedTempFile,
    pub pages: u32,
    pub revision: u64,
}

impl Document {
    pub fn open(source: &Path, revision: u64) -> Result<Self> {
        let source = source.canonicalize().context("PDF file not found")?;
        let snapshot = NamedTempFile::new().context("create PDF snapshot")?;
        std::fs::copy(&source, snapshot.path()).context("copy PDF snapshot")?;
        let output = Command::new("pdfinfo")
            .arg(snapshot.path()).output().context("pdfinfo not found: install poppler-utils")?;
        if !output.status.success() {
            bail!("cannot open PDF: {}", String::from_utf8_lossy(&output.stderr).trim());
        }
        let info = String::from_utf8_lossy(&output.stdout);
        let pages: u32 = info.lines().find_map(|line| {
            line.strip_prefix("Pages:").and_then(|count| count.trim().parse().ok())
        }).context("pdfinfo did not report the page count")?;
        if pages == 0 { bail!("PDF has no pages"); }
        Ok(Self { snapshot, pages, revision })
    }

    pub fn snapshot_path(&self) -> &Path { self.snapshot.path() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_three_page_fixture() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let doc = Document::open(&root.join("tests/fixtures/sample.pdf"), 1).unwrap();
        assert_eq!(doc.pages, 3);
        assert!(doc.snapshot_path().exists());
    }
}
