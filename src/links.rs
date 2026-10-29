//! PDF annotations and explicit coordinate transforms. No terminal IO or PDF actions
//! are executed here. Metadata is extracted from the immutable renderer snapshot.
use anyhow::{bail, Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId};
use ratatui::layout::{Rect, Size};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PdfRect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
impl PdfRect {
    fn new(a: [f64; 4]) -> Result<Self> {
        if !a.iter().all(|n| n.is_finite()) {
            bail!("nonfinite PDF rectangle");
        }
        Ok(Self {
            x0: a[0].min(a[2]),
            y0: a[1].min(a[3]),
            x1: a[0].max(a[2]),
            y1: a[1].max(a[3]),
        })
    }
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
    fn area(self) -> f64 {
        (self.x1 - self.x0).max(0.) * (self.y1 - self.y0).max(0.)
    }
    fn intersection(self, b: Self) -> f64 {
        (self.x1.min(b.x1) - self.x0.max(b.x0)).max(0.)
            * (self.y1.min(b.y1) - self.y0.max(b.y0)).max(0.)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PageSpace {
    pub media: PdfRect,
    pub crop: PdfRect,
    pub rotation: u16,
}
impl PageSpace {
    pub fn size(self) -> (f64, f64) {
        let (w, h) = (self.media.x1 - self.media.x0, self.media.y1 - self.media.y0);
        if self.rotation == 90 || self.rotation == 270 {
            (h, w)
        } else {
            (w, h)
        }
    }
    /// Normalized top-left raster coordinates, with inherited PDF rotation applied.
    pub fn to_display(self, x: f64, y: f64) -> (f64, f64) {
        let u = (x - self.media.x0) / (self.media.x1 - self.media.x0);
        let v = (self.media.y1 - y) / (self.media.y1 - self.media.y0);
        match self.rotation {
            90 => (1. - v, u),
            180 => (1. - u, 1. - v),
            270 => (v, 1. - u),
            _ => (u, v),
        }
    }
    pub fn to_pdf(self, u: f64, v: f64) -> (f64, f64) {
        let (u, v) = match self.rotation {
            90 => (v, 1. - u),
            180 => (1. - u, 1. - v),
            270 => (1. - v, u),
            _ => (u, v),
        };
        (
            self.media.x0 + u * (self.media.x1 - self.media.x0),
            self.media.y1 - v * (self.media.y1 - self.media.y0),
        )
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DestinationKind {
    Xyz {
        left: Option<f64>,
        top: Option<f64>,
        zoom: Option<f64>,
    },
    Fit,
    FitH(Option<f64>),
    FitV(Option<f64>),
    FitR(PdfRect),
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Destination {
    pub page: u32,
    pub space: PageSpace,
    pub kind: DestinationKind,
}
impl Destination {
    /// Convert PDF magnification/fit semantics into Leafview's pane-width-relative zoom.
    pub fn view_zoom(&self, viewport: (u32, u32), current: f32) -> f32 {
        let (w, h) = self.space.size();
        let (vw, vh) = (viewport.0.max(1) as f64, viewport.1.max(1) as f64);
        let scale = match self.kind {
            DestinationKind::Xyz { zoom: Some(z), .. } => z,
            DestinationKind::Xyz { .. } => return current,
            DestinationKind::Fit => (vw / w).min(vh / h),
            DestinationKind::FitH(_) => {
                if matches!(self.space.rotation, 90 | 270) {
                    vh / h
                } else {
                    vw / w
                }
            }
            DestinationKind::FitV(_) => {
                if matches!(self.space.rotation, 90 | 270) {
                    vw / w
                } else {
                    vh / h
                }
            }
            DestinationKind::FitR(r) => {
                let (a, b) = self.space.to_display(r.x0, r.y0);
                let (c, d) = self.space.to_display(r.x1, r.y1);
                (vw / ((a - c).abs() * w)).min(vh / ((b - d).abs() * h))
            }
        };
        ((scale * w / vw) as f32).clamp(0.5, 3.)
    }
    pub fn offsets(&self, raster: (u32, u32), old: (u32, u32)) -> (u32, u32) {
        let old_pdf = self.space.to_pdf(
            old.0 as f64 / raster.0.max(1) as f64,
            old.1 as f64 / raster.1.max(1) as f64,
        );
        let (x, y) = match self.kind {
            DestinationKind::Xyz { left, top, .. } => self
                .space
                .to_display(left.unwrap_or(old_pdf.0), top.unwrap_or(old_pdf.1)),
            DestinationKind::Fit => (0., 0.),
            DestinationKind::FitH(top) => self
                .space
                .to_display(self.space.media.x0, top.unwrap_or(old_pdf.1)),
            DestinationKind::FitV(left) => self
                .space
                .to_display(left.unwrap_or(old_pdf.0), self.space.media.y1),
            DestinationKind::FitR(r) => {
                let (a, b) = self.space.to_display(r.x0, r.y0);
                let (c, d) = self.space.to_display(r.x1, r.y1);
                (a.min(c), b.min(d))
            }
        };
        (
            (x.max(0.) * raster.0 as f64).round() as u32,
            (y.max(0.) * raster.1 as f64).round() as u32,
        )
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LinkAction {
    Internal(Destination),
    External(String),
    Unsupported(String),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PdfLink {
    pub bounds: PdfRect,
    pub action: LinkAction,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PageLinks {
    pub space: PageSpace,
    pub links: Vec<PdfLink>,
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LinkIndex {
    pub pages: Vec<Option<Arc<PageLinks>>>,
    pub error: Option<String>,
}

fn object<'a>(d: &'a Document, o: &'a Object) -> Result<&'a Object> {
    Ok(d.dereference(o)?.1)
}
fn dict<'a>(d: &'a Document, o: &'a Object) -> Result<&'a Dictionary> {
    Ok(object(d, o)?.as_dict()?)
}
fn num(d: &Document, o: &Object) -> Result<f64> {
    let v = object(d, o)?.as_float()? as f64;
    if !v.is_finite() {
        bail!("nonfinite PDF number")
    }
    Ok(v)
}
fn rectangle(d: &Document, o: &Object) -> Result<PdfRect> {
    let a = object(d, o)?.as_array()?;
    if a.len() != 4 {
        bail!("invalid rectangle")
    }
    PdfRect::new([
        num(d, &a[0])?,
        num(d, &a[1])?,
        num(d, &a[2])?,
        num(d, &a[3])?,
    ])
}
fn inherited<'a>(d: &'a Document, mut id: ObjectId, key: &[u8]) -> Result<&'a Object> {
    let mut seen = HashSet::new();
    for _ in 0..64 {
        if !seen.insert(id) {
            bail!("cyclic page tree")
        }
        let page = d.get_object(id)?.as_dict()?;
        if let Ok(value) = page.get(key) {
            return object(d, value);
        }
        id = page.get(b"Parent")?.as_reference()?;
    }
    bail!("page tree too deep")
}
fn space(d: &Document, id: ObjectId) -> Result<PageSpace> {
    let media = rectangle(d, inherited(d, id, b"MediaBox")?)?;
    if media.area() <= 0. {
        bail!("empty MediaBox")
    }
    let crop = inherited(d, id, b"CropBox")
        .and_then(|o| rectangle(d, o))
        .unwrap_or(media);
    let rotation = inherited(d, id, b"Rotate")
        .and_then(|o| Ok(o.as_i64()?))
        .unwrap_or(0)
        .rem_euclid(360) as u16;
    if ![0, 90, 180, 270].contains(&rotation) {
        bail!("unsupported page rotation")
    }
    Ok(PageSpace {
        media,
        crop,
        rotation,
    })
}
fn name(o: &Object) -> Result<&[u8]> {
    Ok(match o {
        Object::Name(n) | Object::String(n, _) => n,
        _ => bail!("invalid destination name"),
    })
}
fn find_named<'a>(d: &'a Document, key: &[u8]) -> Result<&'a Object> {
    let catalog = d.catalog()?;
    if let Ok(legacy) = catalog.get(b"Dests") {
        if let Ok(value) = dict(d, legacy)?.get(key) {
            return Ok(value);
        }
    }
    let names = dict(d, catalog.get(b"Names")?)?;
    let mut stack = vec![names.get(b"Dests")?];
    let mut seen = HashSet::new();
    let mut count = 0;
    while let Some(node) = stack.pop() {
        count += 1;
        if count > 10000 {
            bail!("destination tree limit")
        }
        if let Object::Reference(id) = node {
            if !seen.insert(*id) {
                continue;
            }
        }
        let node = dict(d, node)?;
        if let Ok(entries) = node.get(b"Names") {
            for pair in object(d, entries)?.as_array()?.as_chunks::<2>().0 {
                if name(object(d, &pair[0])?)? == key {
                    return Ok(&pair[1]);
                }
            }
        }
        if let Ok(kids) = node.get(b"Kids") {
            stack.extend(object(d, kids)?.as_array()?)
        }
    }
    bail!("unresolved named destination")
}
fn destination(
    d: &Document,
    o: &Object,
    pages: &HashMap<ObjectId, u32>,
    spaces: &[Option<PageSpace>],
) -> Result<Destination> {
    let mut o = o;
    let mut visited = HashSet::new();
    for _ in 0..32 {
        o = object(d, o)?;
        match o {
            Object::Name(_) | Object::String(..) => {
                let key = name(o)?;
                if !visited.insert(key.to_vec()) {
                    bail!("cyclic named destination")
                };
                o = find_named(d, key)?
            }
            Object::Dictionary(v) => o = v.get(b"D")?,
            Object::Array(a) => {
                if a.len() < 2 {
                    bail!("short destination")
                }
                // Local GoTo requires a page object reference; integer page numbers
                // belong to remote destinations, which are deliberately unsupported.
                let page = *pages
                    .get(&a[0].as_reference()?)
                    .context("unknown destination page")?;
                let space =
                    spaces[page as usize].context("unsupported destination page geometry")?;
                let optional = |i: usize| -> Result<Option<f64>> {
                    let Some(value) = a.get(i) else {
                        return Ok(None);
                    };
                    let value = object(d, value)?;
                    if matches!(value, Object::Null) {
                        Ok(None)
                    } else {
                        Ok(Some(num(d, value)?))
                    }
                };
                let kind = match object(d, &a[1])?.as_name()? {
                    b"XYZ" => {
                        let zoom = optional(4)?.filter(|v| *v != 0.);
                        if zoom.is_some_and(|v| v <= 0.) {
                            bail!("invalid destination zoom")
                        };
                        DestinationKind::Xyz {
                            left: optional(2)?,
                            top: optional(3)?,
                            zoom,
                        }
                    }
                    b"Fit" | b"FitB" => DestinationKind::Fit,
                    b"FitH" | b"FitBH" => DestinationKind::FitH(optional(2)?),
                    b"FitV" | b"FitBV" => DestinationKind::FitV(optional(2)?),
                    b"FitR" => {
                        if a.len() != 6 {
                            bail!("invalid FitR")
                        };
                        let r = PdfRect::new([
                            num(d, &a[2])?,
                            num(d, &a[3])?,
                            num(d, &a[4])?,
                            num(d, &a[5])?,
                        ])?;
                        if r.area() <= 0. {
                            bail!("empty FitR")
                        };
                        DestinationKind::FitR(r)
                    }
                    _ => bail!("unsupported destination type"),
                };
                return Ok(Destination { page, space, kind });
            }
            _ => bail!("invalid destination"),
        }
    }
    bail!("destination recursion limit")
}
fn action(
    d: &Document,
    a: &Dictionary,
    pages: &HashMap<ObjectId, u32>,
    spaces: &[Option<PageSpace>],
) -> Result<LinkAction> {
    if let Ok(dest) = a.get(b"Dest") {
        return Ok(LinkAction::Internal(destination(d, dest, pages, spaces)?));
    }
    let a = dict(d, a.get(b"A")?)?;
    match object(d, a.get(b"S")?)?.as_name()? {
        b"URI" => {
            let url = String::from_utf8(object(d, a.get(b"URI")?)?.as_str()?.to_vec())
                .context("non UTF-8 URI")?;
            validate_url(&url)?;
            Ok(LinkAction::External(url))
        }
        b"GoTo" => Ok(LinkAction::Internal(destination(
            d,
            a.get(b"D")?,
            pages,
            spaces,
        )?)),
        _ => bail!("unsupported PDF action"),
    }
}
impl LinkIndex {
    pub fn load(path: &Path, cancelled: impl Fn() -> bool) -> Self {
        let result = (|| -> Result<Self> {
            use std::io::{Read, Seek, SeekFrom};
            let mut output = crate::process::run(
                std::process::Command::new(std::env::current_exe()?)
                    .arg("--extract-links")
                    .arg(path),
                32 * 1024 * 1024,
                cancelled,
            )?;
            output.seek(SeekFrom::Start(0))?;
            let mut bytes = Vec::new();
            output.take(32 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
            Ok(serde_json::from_slice(&bytes)?)
        })();
        match result {
            Ok(v) => v,
            Err(e) => Self {
                pages: vec![],
                error: Some(format!("Links unavailable: {e}")),
            },
        }
    }
    pub fn extract(path: &Path) -> Result<Self> {
        let d = Document::load(path)?;
        if d.is_encrypted() {
            bail!("encrypted PDF links unsupported")
        }
        let pages = d.get_pages();
        if pages.len() > 100000 {
            bail!("link page limit")
        }
        let ids: HashMap<_, _> = pages
            .values()
            .enumerate()
            .map(|(i, id)| (*id, i as u32))
            .collect();
        let spaces: Vec<_> = pages.values().map(|id| space(&d, *id).ok()).collect();
        let mut out = Vec::with_capacity(pages.len());
        let mut total = 0;
        for (i, id) in pages.values().enumerate() {
            let Some(space) = spaces[i] else {
                out.push(None);
                continue;
            };
            let mut links = vec![];
            if let Ok(annotations) = d.get_object(*id)?.as_dict()?.get(b"Annots") {
                for a in object(&d, annotations)?.as_array()?.iter().take(4096) {
                    let Ok(a) = dict(&d, a) else { continue };
                    if a.get(b"Subtype")
                        .ok()
                        .and_then(|o| object(&d, o).ok())
                        .and_then(|o| o.as_name().ok())
                        != Some(b"Link".as_slice())
                    {
                        continue;
                    }
                    let flags = a.get(b"F").ok().and_then(|o| o.as_i64().ok()).unwrap_or(0);
                    if flags & (1 | 2 | 32) != 0 {
                        continue;
                    }
                    let Ok(bounds) = a
                        .get(b"Rect")
                        .map_err(anyhow::Error::from)
                        .and_then(|o| rectangle(&d, o))
                    else {
                        continue;
                    };
                    if total >= 100000 {
                        bail!("link annotation limit")
                    };
                    total += 1;
                    if a.get(b"QuadPoints").is_ok() {
                        continue;
                    } // Nonrectangular links need polygon hit testing.
                    links.push(PdfLink {
                        bounds,
                        action: action(&d, a, &ids, &spaces)
                            .unwrap_or_else(|e| LinkAction::Unsupported(e.to_string())),
                    });
                }
            }
            out.push(Some(Arc::new(PageLinks { space, links })));
        }
        Ok(Self {
            pages: out,
            error: None,
        })
    }
}

/// Snapshot identity and exact decoded pixel dimensions travel with the encoded image.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RenderGeometry {
    pub revision: u64,
    pub page: u32,
    pub generation: u64,
    pub raster: (u32, u32),
    pub crop: (u32, u32, u32, u32),
    pub links: Option<Arc<PageLinks>>,
}
pub fn image_area(screen: Rect, size: Size) -> Rect {
    Rect {
        x: screen.x + screen.width.saturating_sub(size.width) / 2,
        y: screen.y + screen.height.saturating_sub(size.height) / 2,
        width: screen.width.min(size.width),
        height: screen.height.min(size.height),
    }
}
impl RenderGeometry {
    pub fn is_current(&self, revision: u64, page: u32, generation: u64) -> bool {
        self.revision == revision && self.page == page && self.generation == generation
    }
    pub fn hit(&self, column: u16, row: u16, area: Rect, font: (u32, u32)) -> Option<usize> {
        if !area.contains((column, row).into()) || font.0 == 0 || font.1 == 0 {
            return None;
        }
        let links = self.links.as_ref()?;
        let px = f64::from(column - area.x) * font.0 as f64;
        let py = f64::from(row - area.y) * font.1 as f64;
        let (cx, cy, cw, ch) = self.crop;
        // Last cell can be partially padded. Never click its empty portion.
        if px + font.0 as f64 > cw as f64 || py + font.1 as f64 > ch as f64 {
            return None;
        }
        let to_pdf = |x: f64, y: f64| {
            links.space.to_pdf(
                (cx as f64 + x) / self.raster.0 as f64,
                (cy as f64 + y) / self.raster.1 as f64,
            )
        };
        let (a, b) = to_pdf(px, py);
        let (c, d) = to_pdf(px + font.0 as f64, py + font.1 as f64);
        let cell = PdfRect::new([a, b, c, d]).ok()?;
        let center = to_pdf(px + font.0 as f64 / 2., py + font.1 as f64 / 2.);
        let mut hit: Option<usize> = None;
        for (i, link) in links.links.iter().enumerate() {
            if link.bounds.intersection(cell) > 0. {
                // Text links are often shorter than a terminal cell. Accept an
                // unambiguous overlap, but never guess between distinct targets.
                if let Some(previous) = hit {
                    if links.links[previous].action != link.action {
                        return None;
                    }
                }
                if hit.is_none() || link.bounds.contains(center.0, center.1) {
                    hit = Some(i);
                }
            }
        }
        hit
    }
}
#[cfg(test)]
mod hit_tests {
    use super::*;
    #[test]
    fn short_links_and_duplicate_annotations_are_clickable_but_ambiguous_targets_are_not() {
        let space = PageSpace {
            media: PdfRect::new([0., 0., 100., 100.]).unwrap(),
            crop: PdfRect::new([0., 0., 100., 100.]).unwrap(),
            rotation: 0,
        };
        let link = PdfLink {
            bounds: PdfRect::new([1., 94., 9., 96.]).unwrap(),
            action: LinkAction::External("https://example.org".into()),
        };
        let mut page = PageLinks {
            space,
            links: vec![link.clone(), link],
        };
        let mut geometry = RenderGeometry {
            revision: 1,
            page: 0,
            generation: 1,
            raster: (100, 100),
            crop: (0, 0, 100, 100),
            links: Some(Arc::new(page.clone())),
        };
        let area = Rect::new(0, 0, 10, 10);
        assert_eq!(geometry.hit(0, 0, area, (10, 10)), Some(1));
        page.links[1].action = LinkAction::External("https://other.example".into());
        geometry.links = Some(Arc::new(page));
        assert_eq!(geometry.hit(0, 0, area, (10, 10)), None);
    }
}

pub fn sanitize(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
        })
        .take(180)
        .collect()
}
pub fn validate_url(url: &str) -> Result<()> {
    if url.len() > 8192 || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        bail!("invalid URL")
    }
    let (_, rest) = url
        .split_once("://")
        .filter(|(s, _)| s.eq_ignore_ascii_case("https") || s.eq_ignore_ascii_case("http"))
        .context("only http/https links supported")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if host.is_empty() || host.contains(['\\', '@']) {
        bail!("invalid URL host")
    }
    Ok(())
}
pub fn browser_command(url: &str) -> Result<std::process::Command> {
    validate_url(url)?;
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(url);
    Ok(cmd)
}
pub fn open_url(url: String, stop: &std::sync::atomic::AtomicBool) -> Result<()> {
    use std::{
        process::Stdio,
        sync::atomic::Ordering,
        time::{Duration, Instant},
    };
    let child = browser_command(&url)?
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("cannot start xdg-open")?;
    struct Launcher(std::process::Child);
    impl Drop for Launcher {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut guard = Launcher(child);
    let child = &mut guard.0;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                bail!("xdg-open failed ({status})")
            };
            return Ok(());
        }
        if stop.load(Ordering::SeqCst) || start.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            let _ = child.wait();
            bail!("browser launcher timed out")
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
