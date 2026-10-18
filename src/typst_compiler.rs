//! Private compiler process: PDF and source coordinates come from one layout.
use crate::typst_live::{reply_path, CompileReply, CompileRequest, SourcePoint};
use anyhow::{Context, Result};
use std::{collections::BTreeMap, io::BufRead, path::PathBuf};
use typst::{
    diag::FileResult,
    foundations::{Bytes, Datetime, Duration},
    syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot},
    text::{Font, FontBook},
    utils::LazyHash,
    Library, LibraryExt, World, WorldExt,
};
use typst_kit::{
    datetime::Time,
    downloader::SystemDownloader,
    files::{FileStore, FsRoot, SystemFiles},
    fonts::FontStore,
    packages::SystemPackages,
};
use typst_layout::PagedDocument;

struct Compiler {
    library: LazyHash<Library>,
    fonts: FontStore,
    files: FileStore<SystemFiles>,
    main: FileId,
    now: Time,
    pdf_options: typst_pdf::PdfOptions,
}
impl World for Compiler {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        self.files.source(id)
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.file(id)
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }
    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.now.today(offset)
    }
}
impl Compiler {
    fn new(input: PathBuf, root: PathBuf, options: crate::typst_options::Options) -> Result<Self> {
        let main = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::virtualize(&root, &input).map_err(|e| anyhow::anyhow!("{e:?}"))?,
        )
        .intern();
        let mut fonts = FontStore::new();
        for path in &options.fonts {
            fonts.extend(typst_kit::fonts::scan(path));
        }
        if !options.ignore_system {
            fonts.extend(typst_kit::fonts::system());
        }
        if !options.ignore_embedded {
            fonts.extend(typst_kit::fonts::embedded());
        }
        use typst::foundations::IntoValue;
        let inputs = options
            .inputs
            .iter()
            .map(|(k, v)| (k.as_str().into(), v.as_str().into_value()))
            .collect();
        let features = options
            .features
            .iter()
            .map(|name| {
                typst::Feature::all()
                    .find(|f| match f { typst::Feature::Html => "html", typst::Feature::Bundle => "bundle", typst::Feature::A11yExtras => "a11y-extras", _ => "" } == name)
                    .with_context(|| format!("unknown Typst feature: {name}"))
            })
            .collect::<Result<Vec<_>>>()?;
        let downloader = if let Some(cert) = options.cert {
            SystemDownloader::with_cert_path("leafview", cert)
        } else {
            SystemDownloader::new("leafview")
        };
        use typst_kit::packages::{FsPackages, UniversePackages};
        let packages = SystemPackages::from_parts(
            options
                .package
                .map(FsPackages::new)
                .or_else(FsPackages::system_data),
            options
                .cache
                .map(FsPackages::new)
                .or_else(FsPackages::system_cache),
            UniversePackages::new(downloader),
        );
        let now = options
            .timestamp
            .map(Time::fixed_timestamp)
            .transpose()
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .unwrap_or_else(Time::system);
        let standards = options
            .standards
            .iter()
            .map(|s| serde_json::from_value(serde_json::Value::String(s.clone())))
            .collect::<std::result::Result<Vec<typst_pdf::PdfStandard>, _>>()?;
        let pdf_options = typst_pdf::PdfOptions {
            pretty: options.pretty,
            tagged: !options.no_tags,
            standards: typst_pdf::PdfStandards::new(&standards)
                .map_err(|e| anyhow::anyhow!("{e:?}"))?,
            page_ranges: options.pages.as_deref().map(parse_pages).transpose()?,
            timestamp: if options.timestamp.is_some() {
                now.today(None).map(typst_pdf::Timestamp::new_utc)
            } else {
                None
            },
            ..Default::default()
        };
        Ok(Self {
            library: LazyHash::new(
                Library::builder()
                    .with_inputs(inputs)
                    .with_features(features.into_iter().collect())
                    .build(),
            ),
            fonts,
            files: FileStore::new(SystemFiles::new(FsRoot::new(root), packages)),
            main,
            now,
            pdf_options,
        })
    }
    fn compile(&mut self, request: CompileRequest) -> CompileReply {
        let start = std::time::Instant::now();
        self.files.reset();
        self.now.reset();
        let output = typst::compile::<PagedDocument>(self);
        let (loader, ids) = self.files.dependencies();
        let deps: Vec<_> = ids
            .filter_map(|id| loader.resolve(id).ok().map(|p| (id, p)))
            .collect();
        let mut sources = BTreeMap::new();
        let mut source_bytes = 0usize;
        for (id, path) in &deps {
            if path.extension().is_some_and(|e| e == "typ") {
                if let Ok(source) = self.source(*id) {
                    source_bytes = source_bytes.saturating_add(source.text().len());
                    if source_bytes <= 16 * 1024 * 1024 {
                        sources.insert(path.clone(), source.text().to_owned());
                    }
                }
            }
        }
        let mut reply = CompileReply {
            generation: request.generation,
            dependencies: deps.into_iter().map(|(_, p)| p).collect(),
            sources,
            points: Vec::new(),
            error: None,
            millis: 0,
        };
        if source_bytes > 16 * 1024 * 1024 {
            reply.error = Some("Typst source snapshots exceed 16 MiB".into());
            return reply;
        }
        match output.output {
            Err(errors) => {
                reply.error = Some(
                    errors
                        .iter()
                        .take(3)
                        .map(|e| {
                            let mut diagnostic = format!("error: {}", e.message);
                            if let Some(id) = e.span.id() {
                                if let (Ok(source), Some(range), Ok(path)) = (
                                    self.source(id),
                                    self.range(e.span),
                                    self.files.loader().resolve(id),
                                ) {
                                    let offset = range.start.min(source.text().len());
                                    let prefix = &source.text()[..offset];
                                    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
                                    let column =
                                        prefix.rsplit('\n').next().unwrap_or("").chars().count()
                                            + 1;
                                    let text = source.text().lines().nth(line - 1).unwrap_or("");
                                    diagnostic.push_str(&format!(
                                    "\n  --> {}:{line}:{column}\n   |\n{line:>3} | {}\n   | {}^",
                                    path.display(),
                                    text.chars().take(180).collect::<String>(),
                                    " ".repeat(column.saturating_sub(1).min(179))
                                ));
                                }
                            }
                            for hint in e.hints.iter().take(3) {
                                diagnostic.push_str(&format!("\n hint: {}", hint.v));
                            }
                            diagnostic
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                )
            }
            Ok(document) => {
                let result = (|| -> Result<()> {
                    // Gather source spans from the exact frames exported below.
                    let mut exported_page = 0;
                    for (page, item) in document.pages().iter().enumerate() {
                        if self
                            .pdf_options
                            .page_ranges
                            .as_ref()
                            .is_some_and(|ranges| !ranges.includes_page_index(page))
                        {
                            continue;
                        }
                        collect(
                            self,
                            &item.frame,
                            typst::layout::Transform::identity(),
                            exported_page,
                            (item.frame.width().to_pt(), item.frame.height().to_pt()),
                            &mut reply.points,
                        );
                        exported_page += 1;
                    }
                    anyhow::ensure!(
                        reply.points.len() <= 500_000,
                        "source map exceeds 500000 points"
                    );
                    let pdf = typst_pdf::pdf(&document, &self.pdf_options)
                        .map_err(|e| anyhow::anyhow!("PDF export: {e:?}"))?;
                    anyhow::ensure!(
                        pdf.len() <= 512 * 1024 * 1024,
                        "compiled PDF exceeds 512 MiB"
                    );
                    std::fs::write(&request.output, pdf)?;
                    Ok(())
                })();
                if let Err(e) = result {
                    reply.error = Some(format!("{e:#}"));
                    reply.points.clear();
                }
            }
        }
        reply.millis = start.elapsed().as_millis() as u64;
        typst::comemo::evict(5);
        reply
    }
}
fn collect(
    world: &Compiler,
    frame: &typst::layout::Frame,
    transform: typst::layout::Transform,
    page: u32,
    size: (f64, f64),
    points: &mut Vec<SourcePoint>,
) {
    use typst::layout::{FrameItem, Transform};
    for (mut pos, item) in frame.items().cloned() {
        match item {
            FrameItem::Group(group) => {
                collect(
                    world,
                    &group.frame,
                    group
                        .transform
                        .post_concat(Transform::translate(pos.x, pos.y))
                        .post_concat(transform),
                    page,
                    size,
                    points,
                );
            }
            FrameItem::Text(text) => {
                for glyph in text.glyphs {
                    if points.len() > 500_000 {
                        return;
                    }
                    if let Some(id) = glyph.span.0.id() {
                        if let (Ok(source), Ok(path)) =
                            (world.source(id), world.files.loader().resolve(id))
                        {
                            if let Some(range) = source.find(glyph.span.0).map(|node| node.range())
                            {
                                let point = pos.transform(transform);
                                // Math text runs retain one span for a multi-digit
                                // token; unlike markup their glyph offsets live
                                // in TextItem::range rather than span.1.
                                let math = source.find(glyph.span.0).is_some_and(|node| {
                                    node.kind() == typst::syntax::SyntaxKind::MathText
                                });
                                let candidate = SourcePoint {
                                    file: path,
                                    start: if math {
                                        range.start
                                    } else {
                                        (range.start + usize::from(glyph.span.1)).min(range.end)
                                    },
                                    end: if math {
                                        range.end
                                    } else {
                                        (range.start
                                            + usize::from(glyph.span.1)
                                            + glyph.range.len())
                                        .min(range.end)
                                    },
                                    page,
                                    x: point.x.to_pt() / size.0,
                                    y: point.y.to_pt() / size.1,
                                };
                                if points.last() != Some(&candidate) {
                                    points.push(candidate);
                                }
                            }
                        }
                    }
                    pos.x += glyph.x_advance.at(text.size);
                }
            }
            _ => {}
        }
    }
}
pub fn serve() -> Result<()> {
    let input = std::env::args_os().nth(2).context("missing Typst input")?;
    let root = std::env::args_os().nth(3).context("missing Typst root")?;
    let options: crate::typst_options::Options = std::env::args()
        .nth(4)
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default();
    rayon::ThreadPoolBuilder::new()
        .num_threads(options.jobs.unwrap_or(2))
        .build_global()
        .ok();
    let mut compiler = Compiler::new(input.into(), root.into(), options)?;
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        anyhow::ensure!(line.len() < 65536, "compiler request too large");
        let request: CompileRequest = serde_json::from_str(&line)?;
        let request_output = request.output.clone();
        let reply = compiler.compile(request);
        let path = reply_path(&request_output);
        let temporary = path.with_extension("pending");
        serde_json::to_writer(std::fs::File::create(&temporary)?, &reply)?;
        std::fs::rename(temporary, path)?;
    }
    Ok(())
}

fn parse_pages(text: &str) -> Result<typst::layout::PageRanges> {
    use std::num::NonZeroUsize;
    let mut ranges = Vec::new();
    for part in text.split(',') {
        let parse = |s: &str| -> Result<Option<NonZeroUsize>> {
            if s.is_empty() {
                return Ok(None);
            }
            Ok(Some(
                NonZeroUsize::new(s.parse()?).context("page numbers start at 1")?,
            ))
        };
        let (start, end) = if let Some((a, b)) = part.split_once('-') {
            (parse(a)?, parse(b)?)
        } else {
            let n = parse(part)?.context("empty page range")?;
            (Some(n), Some(n))
        };
        anyhow::ensure!(
            start.zip(end).is_none_or(|(a, b)| a <= b),
            "reversed page range"
        );
        ranges.push(start..=end);
    }
    Ok(typst::layout::PageRanges::new(ranges))
}
