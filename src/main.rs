//! Leafview: a small terminal PDF viewer with latest-wins rendering and tmux graphics support.
mod document;
#[cfg(test)]
mod link_tests;
mod links;
mod process;
mod render;
mod view;
mod workers;

use anyhow::{Context, Result};
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use document::Document;
use links::{Destination, LinkAction, RenderGeometry};
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
    displayed: Option<RenderGeometry>,
    image_area: ratatui::layout::Rect,
    hovered: Option<usize>,
    hover_deadline: Option<Instant>,
    pointer: Option<(u16, u16)>,
    destination: Option<Destination>,
    history: std::collections::VecDeque<View>,
    browser: Worker<String, Result<()>>,
    browser_pending: bool,
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
    backend: ratatui_image::picker::ProtocolType,
    graphics_safe: bool,
    last_render_at: Option<Instant>,
    render_due: Option<Instant>,
}

// Only targeted Kitty deletion: never erase images belonging to adjacent panes.
struct GraphicsCleanup {
    id: u32,
    enabled: bool,
    tmux: bool,
}
impl GraphicsCleanup {
    fn new(enabled: bool, tmux: bool) -> Self {
        Self {
            id: std::process::id(),
            enabled,
            tmux,
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
        let seq = if self.tmux {
            format!("\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b"))
        } else {
            seq
        };
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(seq.as_bytes());
        let _ = out.flush();
    }
}

type EncodeInput = (RenderGeometry, image::DynamicImage, Size);
type EncodeOutput = (
    RenderGeometry,
    Result<Protocol, ratatui_image::errors::Errors>,
);

impl App {
    // Bound request churn when the user holds a scroll/navigation key. Requests
    // still use latest-wins semantics, but we don't restart Poppler on every
    // auto-repeat and overwhelm Kitty/tmux with obsolete images.
    fn schedule(&mut self) {
        if self.viewport.width == 0 || self.viewport.height == 0 {
            return;
        }
        self.loading = true;
        self.hovered = None;
        self.hover_deadline = None;
        const MIN_INTERVAL: Duration = Duration::from_millis(75);
        if let Some(last) = self.last_render_at {
            let earliest = last + MIN_INTERVAL;
            if Instant::now() < earliest {
                self.render_due = Some(earliest);
                return;
            }
        }
        self.dispatch_render();
    }
    fn flush_render(&mut self) {
        if self.render_due.is_some_and(|due| Instant::now() >= due) {
            self.dispatch_render();
        }
    }
    fn dispatch_render(&mut self) {
        self.render_due = None;
        self.last_render_at = Some(Instant::now());
        self.renderer.request(Request {
            generation: 0,
            destination: self.destination.clone(),
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
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
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
            KeyCode::Char('b') => {
                if let Some(view) = self.history.pop_back() {
                    self.view = view;
                    request = true;
                }
            }
            KeyCode::Char('r') => {
                self.reload();
                self.dirty = true;
            }
            _ => {}
        }
        if request {
            self.destination = None;
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
        if let Err(err) = self
            .reloader
            .input
            .as_ref()
            .unwrap()
            .try_send((self.source.clone(), self.doc.revision + 1))
        {
            if matches!(err, mpsc::TrySendError::Disconnected(_)) {
                self.reload_pending = false;
                self.error = Some("PDF reload worker stopped unexpectedly".into());
            } else {
                // A pending reload is retried when the current worker completes.
                self.reload_pending = false;
                self.reload_again = true;
            }
        }
    }
    fn finish_reload(&mut self, result: Result<Document>) {
        self.reload_pending = false;
        match result {
            Ok(doc) => {
                self.view.set_pages(doc.pages);
                self.history.clear();
                self.destination = None;
                self.error = doc.links.error.clone();
                self.doc = Arc::new(doc);
                self.schedule();
            }
            Err(err) => self.error = Some(format!("Reload: {err:#}")),
        }
    }
    fn hover_at(&mut self, column: u16, row: u16) {
        let hovered = self
            .displayed
            .as_ref()
            .filter(|g| {
                g.is_current(
                    self.doc.revision,
                    self.view.page,
                    self.renderer.current_generation(),
                ) && !self.loading
            })
            .and_then(|g| {
                g.hit(
                    column,
                    row,
                    self.image_area,
                    (self.font_width, self.font_height),
                )
            });
        if hovered != self.hovered {
            self.hovered = hovered;
            self.hover_deadline = hovered.map(|_| Instant::now() + Duration::from_millis(150));
            self.dirty = true;
        }
    }
    fn maybe_hint(&mut self) {
        let Some(deadline) = self.hover_deadline else {
            return;
        };
        if Instant::now() < deadline {
            return;
        }
        self.hover_deadline = None;
        let Some(id) = self.hovered else {
            return;
        };
        let destination = match self
            .displayed
            .as_ref()
            .and_then(|g| g.links.as_ref())
            .and_then(|l| l.links.get(id))
        {
            Some(link) => match &link.action {
                LinkAction::Internal(destination) => destination.clone(),
                _ => return,
            },
            None => return,
        };
        if destination.page == self.view.page {
            return;
        }
        let zoom =
            destination.view_zoom((self.viewport.width, self.viewport.height), self.view.zoom);
        let width = ((self.viewport.width.max(1) as f32 * zoom).round() as u32).clamp(100, 3400);
        self.renderer
            .hint(self.doc.clone(), destination.page, width);
    }
    fn on_mouse(&mut self, event: MouseEvent) {
        self.pointer = Some((event.column, event.row));
        self.hover_at(event.column, event.row);
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }
        let action = self.hovered.and_then(|id| {
            self.displayed
                .as_ref()?
                .links
                .as_ref()?
                .links
                .get(id)
                .map(|l| l.action.clone())
        });
        match action {
            Some(LinkAction::Internal(dest)) => {
                if self.history.len() == 64 {
                    self.history.pop_front();
                }
                self.history.push_back(self.view.clone());
                self.view.page = dest.page;
                self.view.zoom =
                    dest.view_zoom((self.viewport.width, self.viewport.height), self.view.zoom);
                self.destination = Some(dest);
                self.schedule();
                self.dirty = true;
            }
            Some(LinkAction::External(url)) if !self.browser_pending => {
                match self.browser.input.as_ref().unwrap().try_send(url) {
                    Ok(()) => self.browser_pending = true,
                    Err(_) => self.error = Some("Browser launcher busy".into()),
                }
                self.dirty = true;
            }
            Some(LinkAction::Unsupported(reason)) => {
                self.error = Some(links::sanitize(&reason));
                self.dirty = true;
            }
            _ => {}
        }
    }
    fn consume(&mut self) {
        while let Ok(result) = self.browser.output.try_recv() {
            self.browser_pending = false;
            if let Err(e) = result {
                self.error = Some(format!("Browser: {e}"));
            }
            self.dirty = true;
        }
        while let Ok(result) = self.reloader.output.try_recv() {
            self.finish_reload(result);
            self.dirty = true;
            if self.reload_again {
                self.reload_again = false;
                self.reload();
            }
        }
        while let Ok(completed) = self.completed.try_recv() {
            if completed.generation != self.renderer.current_generation()
                || self.render_due.is_some()
            {
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
                    if let Some(geometry) = completed.geometry {
                        self.encode_latest = Some((geometry, image, size));
                    }
                    self.error = self.doc.links.error.clone();
                }
                Err(err) => {
                    self.error = Some(err);
                    self.loading = false;
                }
            }
            self.dirty = true;
        }
        while let Ok((geometry, result)) = self.encoder.output.try_recv() {
            self.encoding_pending = false;
            if geometry.generation != self.renderer.current_generation()
                || self.render_due.is_some()
            {
                continue;
            }
            self.loading = false;
            match result {
                Ok(proto) => {
                    self.image = Some(proto);
                    self.displayed = Some(geometry);
                    self.hovered = None;
                    self.hover_deadline = None;
                    self.destination = None;
                }
                Err(err) => self.error = Some(format!("Image encoding: {err}")),
            }
            self.dirty = true;
        }
        if !self.encoding_pending {
            if let Some(input) = self.encode_latest.take() {
                if input.0.generation == self.renderer.current_generation() {
                    match self.encoder.input.as_ref().unwrap().try_send(input) {
                        Ok(()) => self.encoding_pending = true,
                        Err(mpsc::TrySendError::Full(input)) => {
                            // Keep the newest image; do not lose it when the
                            // bounded worker queue is momentarily full.
                            self.encode_latest = Some(input);
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => {
                            self.loading = false;
                            self.error = Some("Image encoding worker stopped unexpectedly".into());
                            self.dirty = true;
                        }
                    }
                }
            }
        }
    }
    fn status(&self) -> String {
        if let Some(action) = self.hovered.and_then(|i| {
            self.displayed
                .as_ref()?
                .links
                .as_ref()?
                .links
                .get(i)
                .map(|l| &l.action)
        }) {
            return match action {
                LinkAction::External(url) => {
                    format!(" ↗ {}  [click to open]", links::sanitize(url))
                }
                LinkAction::Internal(d) => {
                    format!(" → Page {}  [click to follow · b back]", d.page + 1)
                }
                LinkAction::Unsupported(reason) => {
                    format!(" Unsupported link: {}", links::sanitize(reason))
                }
            };
        }
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
        let state = if self.graphics_safe {
            "Halfblocks (Ghostty/tmux safety mode) · "
        } else if self.backend == ratatui_image::picker::ProtocolType::Halfblocks {
            "Halfblocks (low resolution) · "
        } else if self.loading {
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
        let area = links::image_area(screen, size);
        app.image_area = area;
        f.render_widget(Image::new(image), area);
    }
    if let Some((x, y)) = app.pointer {
        app.hover_at(x, y);
    }
    f.render_widget(
        Paragraph::new(links::sanitize(&app.status())).style(
            Style::default()
                .fg(if app.hovered.is_some() {
                    Color::LightCyan
                } else {
                    Color::Rgb(195, 199, 208)
                })
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

// Ghostty has documented process-wide crashes when Kitty graphics are emitted
// through tmux (especially with mouse forwarding). A child process cannot
// recover a terminal emulator which itself crashes. Default to the bounded
// text-cell backend for this combination, with explicit native opt-in.
fn use_safe_graphics(tmux: bool, ghostty: bool, mode: &str) -> bool {
    match mode {
        "safe" | "halfblocks" => true,
        "native" => false,
        _ => tmux && ghostty,
    }
}

fn viewer_loop(path: PathBuf, doc: Arc<Document>, mut terminal: DefaultTerminal) -> Result<()> {
    let tmux =
        env::var_os("TMUX").is_some() || env::var("TERM").is_ok_and(|v| v.starts_with("tmux"));
    let ghostty = env::var("TERM_PROGRAM").is_ok_and(|v| v.eq_ignore_ascii_case("ghostty"))
        || env::var_os("GHOSTTY_RESOURCES_DIR").is_some();
    let graphics_safe = use_safe_graphics(
        tmux,
        ghostty,
        &env::var("LEAFVIEW_GRAPHICS")
            .unwrap_or_default()
            .to_ascii_lowercase(),
    );
    // Picker queries must occur after alternate-screen entry, before event polling.
    // In safe mode skip the graphics query entirely.
    let picker = if graphics_safe {
        Picker::halfblocks()
    } else {
        Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
    };
    if env::var("LEAFVIEW_DEBUG").is_ok() {
        eprintln!(
            "leafview: protocol={:?} font={:?} tmux_env={:?} term={:?} term_program={:?} graphics_safe={graphics_safe}",
            picker.protocol_type(),
            picker.font_size(),
            env::var("TMUX").is_ok(),
            env::var("TERM").ok(),
            env::var("TERM_PROGRAM").ok(),
        );
    }
    // Do not feed unsolicited mouse/focus reports into the capability parser.
    // Enable reporting only after its terminal query has finished.
    crossterm::execute!(
        std::io::stdout(),
        event::EnableFocusChange,
        event::EnableMouseCapture
    )?;
    let backend = picker.protocol_type();
    let font = picker.font_size();
    let (tx_completed, completed) = mpsc::sync_channel(1);
    let renderer = Renderer::new(tx_completed)?;
    let graphics = GraphicsCleanup::new(
        picker.protocol_type() == ratatui_image::picker::ProtocolType::Kitty,
        tmux,
    );
    let image_id = graphics.id;
    let encoder = Worker::new(move |(geometry, image, _size): EncodeInput, _| {
        // Pad the last terminal cell instead of stretching the PDF pixels into
        // ceil-sized Kitty placements. All backends now have identical pixel scale.
        let cells = Size::new(
            image.width().div_ceil(u32::from(font.width.max(1))) as u16,
            image.height().div_ceil(u32::from(font.height.max(1))) as u16,
        );
        let mut padded = image::DynamicImage::new_rgba8(
            u32::from(cells.width) * u32::from(font.width.max(1)),
            u32::from(cells.height) * u32::from(font.height.max(1)),
        );
        image::imageops::overlay(&mut padded, &image, 0, 0);
        let image = padded;
        let size = cells;
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
        (geometry, result)
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
        doc: doc.clone(),
        font_width: u32::from(font.width.max(1)),
        font_height: u32::from(font.height.max(1)),
        viewport: Viewport {
            width: 0,
            height: 0,
        },
        renderer,
        image: None,
        displayed: None,
        image_area: ratatui::layout::Rect::default(),
        hovered: None,
        hover_deadline: None,
        pointer: None,
        destination: None,
        history: std::collections::VecDeque::new(),
        browser: Worker::new(links::open_url),
        browser_pending: false,
        completed,
        encoder,
        reloader,
        reload_pending: false,
        reload_again: false,
        encoding_pending: false,
        encode_latest: None,
        jump: String::new(),
        error: doc.links.error.clone(),
        dirty: true,
        loading: false,
        backend,
        graphics_safe,
        last_render_at: None,
        render_due: None,
    };
    app.geometry(terminal.size()?);
    let mut reload_due: Option<Instant> = None;
    loop {
        app.flush_render();
        app.consume();
        app.maybe_hint();
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
                Event::Mouse(mouse) => app.on_mouse(mouse),
                Event::Resize(width, height) => {
                    app.hovered = None;
                    app.hover_deadline = None;
                    app.image_area = ratatui::layout::Rect::default();
                    app.geometry(Size::new(width, height));
                    app.dirty = true;
                }
                Event::FocusLost => {
                    app.pointer = None;
                    app.hovered = None;
                    app.hover_deadline = None;
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
    // Private bounded helper: parsing cannot stall the UI, leak threads, or make
    // shutdown wait for an uninterruptible in-process PDF parser.
    if env::args_os()
        .nth(1)
        .is_some_and(|s| s == "--extract-links")
    {
        let path = env::args_os().nth(2).context("missing snapshot")?;
        let index = links::LinkIndex::extract(Path::new(&path))?;
        serde_json::to_writer(std::io::stdout().lock(), &index)?;
        return Ok(());
    }
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
    viewer_loop(path, doc, terminal)
}

struct TerminalRestore;
impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            event::DisableMouseCapture,
            event::DisableFocusChange
        );
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

#[cfg(test)]
mod graphics_safety_tests {
    use super::use_safe_graphics;

    #[test]
    fn ghostty_tmux_defaults_to_safe_backend() {
        assert!(use_safe_graphics(true, true, "auto"));
        assert!(use_safe_graphics(true, true, ""));
        assert!(!use_safe_graphics(false, true, "auto"));
        assert!(!use_safe_graphics(true, false, "auto"));
        assert!(use_safe_graphics(false, false, "safe"));
        assert!(!use_safe_graphics(true, true, "native"));
    }
}
