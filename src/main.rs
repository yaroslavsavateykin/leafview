//! Leafview: a small terminal PDF viewer with latest-wins rendering and tmux graphics support.
mod document;
mod process;
mod render;
mod view;
mod workers;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use document::Document;
use notify::Watcher;
use ratatui::{
    layout::{Constraint, Layout, Size},
    style::{Color, Style},
    widgets::{Block, Paragraph},
    DefaultTerminal, Frame,
};
use ratatui_image::{picker::Picker, protocol::Protocol, Image, Resize};
use render::{Completed, Renderer, Request};
use std::{
    env,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    time::{Duration, Instant},
};
use view::{raster_width, View, Viewport};
use workers::Worker;

struct App {
    doc: Arc<Document>,
    view: View,
    source: PathBuf,
    font_width: u32,
    font_height: u32,
    viewport: Viewport,
    renderer: Renderer,
    image: Option<Protocol>,
    completed: Receiver<Completed>,
    encoder: Worker<EncodeInput, EncodeOutput>,
    reloader: Worker<(PathBuf, u64), Result<Document>>,
    reload_pending: bool,
    reload_again: bool,
    encoding_pending: bool,
    encode_latest: Option<EncodeInput>,
    jump: String,
    error: Option<String>,
    dirty: bool,
    loading: bool,
}

// Only targeted Kitty deletion: never erase images belonging to adjacent panes.
struct GraphicsCleanup {
    id: u32,
    enabled: bool,
}
impl GraphicsCleanup {
    fn new(enabled: bool) -> Self {
        Self {
            id: std::process::id(),
            enabled,
        }
    }
}
impl Drop for GraphicsCleanup {
    fn drop(&mut self) {
        use std::io::Write;
        if !self.enabled {
            return;
        }
        let seq = format!("\x1b_Ga=d,d=I,i={},q=2;\x1b\\", self.id);
        let seq = if env::var("TERM").is_ok_and(|v| v.starts_with("tmux")) {
            format!("\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b"))
        } else {
            seq
        };
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(seq.as_bytes());
        let _ = out.flush();
    }
}

type EncodeInput = (u64, image::DynamicImage, Size);
type EncodeOutput = (u64, Result<Protocol, ratatui_image::errors::Errors>);

impl App {
    fn schedule(&mut self) {
        if self.viewport.width == 0 || self.viewport.height == 0 {
            return;
        }
        self.loading = true;
        self.renderer.request(Request {
            generation: 0,
            doc: self.doc.clone(),
            page: self.view.page,
            width: raster_width(&self.view, self.viewport),
            viewport: self.viewport,
            x: self.view.x,
            y: self.view.y,
        });
    }
    fn geometry(&mut self, screen: ratatui::layout::Size) {
        let area = Viewport {
            width: u32::from(screen.width)
                .saturating_mul(self.font_width)
                .clamp(1, 3400),
            height: u32::from(screen.height.saturating_sub(1))
                .saturating_mul(self.font_height)
                .clamp(1, 2400),
        };
        if area != self.viewport {
            self.viewport = area;
            self.schedule();
            self.dirty = true;
        }
    }
    fn on_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return false;
        }
        let mut request = false;
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Esc if self.jump.is_empty() => return true,
            KeyCode::Esc => {
                self.jump.clear();
                self.dirty = true;
            }
            KeyCode::Char(c @ '0'..='9') => {
                if self.jump.len() < 8 {
                    self.jump.push(c);
                    self.dirty = true;
                }
            }
            KeyCode::Backspace => {
                self.jump.pop();
                self.dirty = true;
            }
            KeyCode::Enter if !self.jump.is_empty() => {
                if let Ok(page) = self.jump.parse::<u32>() {
                    request = self.view.go_to(page);
                }
                self.jump.clear();
                self.dirty = true;
            }
            KeyCode::Char('l')
            | KeyCode::Char('n')
            | KeyCode::Right
            | KeyCode::PageDown
            | KeyCode::Char(' ') => request = self.view.step_page(1),
            KeyCode::Char('h') | KeyCode::Char('p') | KeyCode::Left | KeyCode::PageUp => {
                request = self.view.step_page(-1)
            }
            KeyCode::Char('g') | KeyCode::Home => request = self.view.go_to(1),
            KeyCode::Char('G') | KeyCode::End => request = self.view.go_to(self.view.pages),
            KeyCode::Char('j') | KeyCode::Down => {
                self.view.scroll(0, 110);
                request = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.view.scroll(0, -110);
                request = true;
            }
            KeyCode::Char('H') => {
                self.view.scroll(-100, 0);
                request = true;
            }
            KeyCode::Char('L') => {
                self.view.scroll(100, 0);
                request = true;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => request = self.view.adjust_zoom(0.25),
            KeyCode::Char('-') | KeyCode::Char('_') => request = self.view.adjust_zoom(-0.25),
            KeyCode::Char('z') => {
                self.view.zoom = 1.0;
                self.view.x = 0;
                self.view.y = 0;
                request = true;
            }
            KeyCode::Char('r') => {
                self.reload();
                self.dirty = true;
            }
            _ => {}
        }
        if request {
            self.schedule();
            self.dirty = true;
        }
        false
    }
    fn reload(&mut self) {
        if self.reload_pending {
            self.reload_again = true;
            return;
        }
        self.reload_pending = true;
        let _ = self
            .reloader
            .input
            .as_ref()
            .unwrap()
            .try_send((self.source.clone(), self.doc.revision + 1));
    }
    fn finish_reload(&mut self, result: Result<Document>) {
        self.reload_pending = false;
        match result {
            Ok(doc) => {
                self.view.set_pages(doc.pages);
                self.doc = Arc::new(doc);
                self.error = None;
                self.schedule();
            }
            Err(err) => self.error = Some(format!("Reload: {err:#}")),
        }
    }
    fn consume(&mut self) {
        while let Ok(result) = self.reloader.output.try_recv() {
            self.finish_reload(result);
            self.dirty = true;
            if self.reload_again {
                self.reload_again = false;
                self.reload();
            }
        }
        while let Ok(completed) = self.completed.try_recv() {
            if completed.generation != self.renderer.current_generation() {
                continue;
            }
            self.view.x = completed.x;
            self.view.y = completed.y;
            match completed.image {
                Ok(image) => {
                    let size = Size::new(
                        (self.viewport.width / self.font_width) as u16,
                        (self.viewport.height / self.font_height) as u16,
                    );
                    self.encode_latest = Some((completed.generation, image, size));
                    self.error = None;
                }
                Err(err) => {
                    self.error = Some(err);
                    self.loading = false;
                }
            }
            self.dirty = true;
        }
        while let Ok((generation, result)) = self.encoder.output.try_recv() {
            self.encoding_pending = false;
            if generation != self.renderer.current_generation() {
                continue;
            }
            self.loading = false;
            match result {
                Ok(proto) => self.image = Some(proto),
                Err(err) => self.error = Some(format!("Image encoding: {err}")),
            }
            self.dirty = true;
        }
        if !self.encoding_pending {
            if let Some(input) = self.encode_latest.take() {
                if input.0 == self.renderer.current_generation() {
                    self.encoding_pending =
                        self.encoder.input.as_ref().unwrap().try_send(input).is_ok();
                }
            }
        }
    }
    fn status(&self) -> String {
        let name = self
            .source
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_else(|| "document.pdf".into());
        if !self.jump.is_empty() {
            return format!(" :{}  Enter: go to page  |  Esc: cancel", self.jump);
        }
        if let Some(err) = &self.error {
            return format!(
                " {}  {}/{}  ERROR: {}",
                name,
                self.view.page + 1,
                self.view.pages,
                abbreviate(err, 110)
            );
        }
        let state = if self.loading {
            "Loading (previous image)…  "
        } else if self.reload_pending {
            "Reloading…  "
        } else {
            ""
        };
        format!(
            " {}  {}/{}   {:.0}%   {}j/k scroll · h/l pages · +/- zoom · r reload · q quit",
            name,
            self.view.page + 1,
            self.view.pages,
            self.view.zoom * 100.0,
            state
        )
    }
}

fn abbreviate(text: &str, max_chars: usize) -> String {
    let mut text = text.chars();
    let mut out: String = text.by_ref().take(max_chars).collect();
    if text.next().is_some() {
        out.push('…');
    }
    out
}

fn ui(f: &mut Frame<'_>, app: &mut App) {
    let parts = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(f.area());
    let screen = parts[0];
    f.render_widget(
        Block::new().style(Style::default().bg(Color::Rgb(20, 22, 26))),
        screen,
    );
    if let Some(image) = &app.image {
        if screen.width == 0 || screen.height == 0 {
            return;
        }
        let size = image.size();
        let mut area = screen;
        area.x += screen.width.saturating_sub(size.width) / 2;
        area.y += screen.height.saturating_sub(size.height) / 2;
        area.width = area.width.min(size.width);
        area.height = area.height.min(size.height);
        f.render_widget(Image::new(image), area);
    }
    f.render_widget(
        Paragraph::new(app.status()).style(
            Style::default()
                .fg(Color::Rgb(195, 199, 208))
                .bg(Color::Rgb(35, 38, 44)),
        ),
        parts[1],
    );
}

fn watch(
    source: &Path,
) -> Result<(
    notify::RecommendedWatcher,
    Receiver<notify::Result<notify::Event>>,
)> {
    let (tx, rx) = mpsc::sync_channel(1);
    let name = source.file_name().unwrap().to_owned();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.as_ref().map_or(true, |e| {
            !matches!(e.kind, notify::EventKind::Access(_))
                && e.paths
                    .iter()
                    .any(|p| p.file_name() == Some(name.as_os_str()))
        }) {
            let _ = tx.try_send(res);
        }
    })?;
    let parent = source.parent().context("PDF has no parent directory")?;
    watcher.watch(parent, notify::RecursiveMode::NonRecursive)?;
    Ok((watcher, rx))
}

fn viewer_loop(path: PathBuf, doc: Arc<Document>, mut terminal: DefaultTerminal) -> Result<()> {
    // Picker queries must occur after alternate-screen entry, before event polling.
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let font = picker.font_size();
    let (tx_completed, completed) = mpsc::sync_channel(1);
    let renderer = Renderer::new(tx_completed)?;
    let graphics =
        GraphicsCleanup::new(picker.protocol_type() == ratatui_image::picker::ProtocolType::Kitty);
    let image_id = graphics.id;
    let tmux = env::var("TERM").is_ok_and(|v| v.starts_with("tmux"));
    let encoder = Worker::new(move |(generation, image, size): EncodeInput, _| {
        let result = if picker.protocol_type() == ratatui_image::picker::ProtocolType::Kitty {
            // Reuse only our own ID: upstream generates a new random ID per protocol
            // and does not release terminal-side image data on Drop.
            let cells = Size::new(
                image.width().div_ceil(u32::from(font.width.max(1))) as u16,
                image.height().div_ceil(u32::from(font.height.max(1))) as u16,
            );
            ratatui_image::protocol::kitty::Kitty::new(image, cells, image_id, tmux)
                .map(Protocol::Kitty)
        } else {
            picker.new_protocol(image, size, Resize::Crop(None))
        };
        (generation, result)
    });
    let reloader = Worker::new(|(path, revision): (PathBuf, u64), stop| {
        Document::open_cancel(&path, revision, || {
            stop.load(std::sync::atomic::Ordering::SeqCst)
        })
    });
    let (watcher, watched) = watch(&path)?;
    let _watcher = watcher; // Keep the OS watcher alive for the duration of the viewer.
    let mut app = App {
        source: path,
        view: View::new(doc.pages),
        doc,
        font_width: u32::from(font.width.max(1)),
        font_height: u32::from(font.height.max(1)),
        viewport: Viewport {
            width: 0,
            height: 0,
        },
        renderer,
        image: None,
        completed,
        encoder,
        reloader,
        reload_pending: false,
        reload_again: false,
        encoding_pending: false,
        encode_latest: None,
        jump: String::new(),
        error: None,
        dirty: true,
        loading: false,
    };
    app.geometry(terminal.size()?);
    let mut reload_due: Option<Instant> = None;
    loop {
        app.consume();
        while let Ok(event) = watched.try_recv() {
            match event {
                Ok(e) => {
                    // Watching the parent (not the inode) also catches atomic rename-save.
                    if e.paths
                        .iter()
                        .any(|p| p.file_name() == app.source.file_name())
                    {
                        reload_due = Some(Instant::now() + Duration::from_millis(350));
                    }
                }
                Err(e) => app.error = Some(format!("Watcher: {e}")),
            }
        }
        if reload_due.is_some_and(|t| Instant::now() >= t) {
            reload_due = None;
            app.reload();
            app.dirty = true;
        }
        if app.dirty {
            terminal.draw(|f| ui(f, &mut app))?;
            app.dirty = false;
        }
        if event::poll(Duration::from_millis(30))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.on_key(key) {
                        break;
                    }
                }
                Event::Resize(width, height) => {
                    app.geometry(Size::new(width, height));
                    app.dirty = true;
                }
                Event::FocusGained => {
                    terminal.clear()?;
                    app.schedule();
                    app.dirty = true;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut args = env::args_os();
    let _exe = args.next();
    let path = match (args.next(), args.next()) {
        (Some(path), None) => PathBuf::from(path),
        _ => {
            eprintln!(
                "Usage: leafview <file.pdf>\n\nRequires Poppler: sudo apt install poppler-utils"
            );
            std::process::exit(2)
        }
    };
    // Check dependencies and PDF *before* switching to alternate-screen mode.
    let path = path
        .canonicalize()
        .with_context(|| format!("cannot open {}", path.display()))?;
    if !path.is_file() {
        anyhow::bail!("not a file: {}", path.display());
    }
    for program in ["pdfinfo", "pdftoppm", "prlimit"] {
        if CommandCheck::missing(program) {
            anyhow::bail!(
                "required command not found: {program} (install poppler-utils and util-linux)"
            );
        }
    }
    let doc = Arc::new(Document::open(&path, 1)?);
    let terminal = ratatui::try_init()?;
    let _restore = TerminalRestore;
    crossterm::execute!(std::io::stdout(), event::EnableFocusChange)?;
    viewer_loop(path, doc, terminal)
}

struct TerminalRestore;
impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), event::DisableFocusChange);
        ratatui::restore();
    }
}

struct CommandCheck;
impl CommandCheck {
    fn missing(command: &str) -> bool {
        std::process::Command::new(command)
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_err()
    }
}
