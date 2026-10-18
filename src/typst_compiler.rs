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
    Library, LibraryExt, World,
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
    fn new(input: PathBuf, root: PathBuf) -> Result<Self> {
        let main = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::virtualize(&root, &input).map_err(|e| anyhow::anyhow!("{e:?}"))?,
        )
        .intern();
        let mut fonts = FontStore::new();
        if let Some(paths) = std::env::var_os("TYPST_FONT_PATHS") {
            for path in std::env::split_paths(&paths) {
                fonts.extend(typst_kit::fonts::scan(&path));
            }
        }
        fonts.extend(typst_kit::fonts::system());
        fonts.extend(typst_kit::fonts::embedded());
        Ok(Self {
            library: LazyHash::new(Library::builder().build()),
            fonts,
            files: FileStore::new(SystemFiles::new(
                FsRoot::new(root),
                SystemPackages::new(SystemDownloader::new("leafview")),
            )),
            main,
            now: Time::system(),
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
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                        .join("; "),
                )
            }
            Ok(document) => {
                let result = (|| -> Result<()> {
                    // Gather source spans from the exact frames exported below.
                    for (page, item) in document.pages().iter().enumerate() {
                        collect(
                            self,
                            &item.frame,
                            typst::layout::Transform::identity(),
                            page as u32,
                            (item.frame.width().to_pt(), item.frame.height().to_pt()),
                            &mut reply.points,
                        );
                    }
                    anyhow::ensure!(
                        reply.points.len() <= 500_000,
                        "source map exceeds 500000 points"
                    );
                    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
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
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build_global()
        .ok();
    let mut compiler = Compiler::new(input.into(), root.into())?;
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
