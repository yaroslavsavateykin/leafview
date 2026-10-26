//! Leafview: a small terminal PDF viewer with latest-wins rendering and tmux graphics support.
#[cfg(feature = "typst")]
mod compile_config;
#[cfg(feature = "typst")]
mod compile_settings;
mod document;
mod files;
mod kitty;
#[cfg(test)]
mod link_tests;
mod links;
mod process;
mod render;
#[cfg(feature = "typst")]
mod typst_compiler;
#[cfg(feature = "typst")]
mod typst_live;
#[cfg(feature = "typst")]
mod typst_options;
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
#[cfg(feature = "typst")]
use ratatui::widgets::{Borders, Clear, Paragraph, Wrap};
use ratatui::{
    layout::{Constraint, Layout, Size},
    style::{Color, Style},
    widgets::Block,
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
    files: files::Files,
    terminal_size: Size,
    opener: Worker<(PathBuf, u64), (PathBuf, Result<Document>)>,
    opening: bool,
    #[cfg(feature = "typst")]
    live: Option<typst_live::Live>,
    #[cfg(feature = "typst")]
    session: Option<compile_config::Session>,
    #[cfg(feature = "typst")]
    settings: Option<compile_settings::Settings>,
    #[cfg(feature = "typst")]
    settings_notice: Option<String>,
    #[cfg(feature = "typst")]
    compile_millis: u64,
    doc: Arc<Document>,
    view: View,
    source: PathBuf,
    font_width: u32,
    font_height: u32,
    viewport: Viewport,
    renderer: Renderer,
    image: Option<DisplayImage>,
    displayed: Option<RenderGeometry>,
    image_area: ratatui::layout::Rect,
    hovered: Option<usize>,
    hover_deadline: Option<Instant>,
    pointer: Option<(u16, u16)>,
    last_swipe_page: Option<Instant>,
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
    #[cfg(feature = "typst")]
    compile_error: Option<String>,
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

enum DisplayImage {
    Kitty(kitty::KittyImage),
    Other(Protocol),
}
impl DisplayImage {
    fn size(&self) -> Size {
        match self {
            Self::Kitty(image) => image.size,
            Self::Other(image) => image.size(),
        }
    }
}

type EncodeInput = (RenderGeometry, image::DynamicImage, Size);
type EncodeOutput = (RenderGeometry, Result<DisplayImage>);

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
        const MIN_INTERVAL: Duration = Duration::from_millis(25);
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
        self.terminal_size = screen;
        let area = Viewport {
            width: u32::from(
                screen
                    .width
                    .saturating_sub(self.files.width(screen.width))
                    .saturating_sub(2),
            )
            .saturating_mul(self.font_width)
            .clamp(1, 3400),
            height: u32::from(screen.height.saturating_sub(2))
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
        #[cfg(feature = "typst")]
        if self.settings.is_some() {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return true;
            }
            let action = self.settings.as_mut().unwrap().key(key, &self.source);
            match action {
                compile_settings::Action::Cancel => self.settings = None,
                compile_settings::Action::Apply(session, value, save) => {
                    let result = (|| -> Result<()> {
                        if save {
                            session.save()?;
                        }
                        let live = self.live.as_mut().context("no live compiler")?;
                        live.configure(&value)?;
                        let mut session = *session;
                        session.warning = None;
                        self.settings_notice = Some(if save {
                            format!(
                                "Saved for {} → {}",
                                self.source
                                    .file_name()
                                    .unwrap_or(self.source.as_os_str())
                                    .to_string_lossy(),
                                session.global_path.display()
                            )
                        } else {
                            "Applied for this session · not saved".into()
                        });
                        self.session = Some(session);
                        Ok(())
                    })();
                    match result {
                        Ok(()) => self.settings = None,
                        Err(e) => self.settings.as_mut().unwrap().error = Some(format!("{e:#}")),
                    }
                }
                compile_settings::Action::None => {}
            }
            self.dirty = true;
            return false;
        }
        #[cfg(feature = "typst")]
        if key.code == KeyCode::Char('o') && self.live.is_some() {
            self.settings_notice = None;
            if let Some(session) = &self.session {
                self.settings = Some(compile_settings::Settings::new(session));
            }
            self.dirty = true;
            return false;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char(direction @ ('h' | 'j' | 'k' | 'l')) => {
                    self.files
                        .focus_direction(direction, self.terminal_size.width);
                }
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                    self.files.resize(
                        key.code,
                        self.terminal_size.width,
                        self.terminal_size.height,
                    );
                    // Moving the tree/Recent divider does not resize the PDF.
                    // Keep its image; geometry schedules a new render only when
                    // the document viewport actually changes.
                    self.geometry(self.terminal_size);
                }
                _ => {
                    return self.on_non_focus_key(key);
                }
            }
            self.dirty = true;
            return false;
        }
        self.on_non_focus_key(key)
    }
    fn on_non_focus_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }
        if key.code == KeyCode::Esc && !self.jump.is_empty() {
            self.jump.clear();
            self.dirty = true;
            return false;
        }
        if matches!(key.code, KeyCode::Esc | KeyCode::F(2)) {
            self.files.visible = !self.files.visible;
            self.files.focused = self.files.visible;
            self.image = None;
            self.geometry(self.terminal_size);
            self.dirty = true;
            return false;
        }
        if key.code == KeyCode::Tab
            && self.files.visible
            && self.files.width(self.terminal_size.width) > 0
        {
            self.files.cycle_focus(self.terminal_size.width);
            self.dirty = true;
            return false;
        }
        if self.files.focused && self.files.width(self.terminal_size.width) > 0 {
            if let Some(path) = self.files.key(key) {
                if !self.opening && !self.reload_pending {
                    if is_typst(&path) {
                        #[cfg(feature = "typst")]
                        if let Err(e) = self.open_typst(&path) {
                            self.files.notice = Some(format!("{e:#}"));
                        }
                        #[cfg(not(feature = "typst"))]
                        {
                            self.files.notice = Some(
                                "Typst support is not enabled; build with --features typst".into(),
                            );
                        }
                    } else if self
                        .opener
                        .input
                        .as_ref()
                        .unwrap()
                        .try_send((path, self.doc.revision + 1))
                        .is_ok()
                    {
                        self.opening = true;
                        self.files.notice = Some("Opening PDF…".into());
                    }
                }
            }
            self.dirty = true;
            return false;
        }
        let mut request = false;
        match key.code {
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
            KeyCode::Right | KeyCode::Down | KeyCode::Char('l' | 'j')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                request = self.view.step_page(1)
            }
            KeyCode::Left | KeyCode::Up | KeyCode::Char('h' | 'k')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                request = self.view.step_page(-1)
            }
            KeyCode::Char('L' | 'J' | 'n' | ' ') | KeyCode::PageDown => {
                request = self.view.step_page(1)
            }
            KeyCode::Char('H' | 'K' | 'p') | KeyCode::PageUp => request = self.view.step_page(-1),
            KeyCode::Char('g') | KeyCode::Home => request = self.view.go_to(1),
            KeyCode::Char('G') | KeyCode::End => request = self.view.go_to(self.view.pages),
            KeyCode::Char('j') | KeyCode::Down => {
                self.view.scroll(0, 30);
                request = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.view.scroll(0, -30);
                request = true;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.view.scroll(-30, 0);
                request = true;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.view.scroll(30, 0);
                request = true;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => request = self.view.adjust_zoom(0.05),
            KeyCode::Char('-') | KeyCode::Char('_') => request = self.view.adjust_zoom(-0.05),
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
            #[cfg(feature = "typst")]
            KeyCode::Char('s') if self.live.is_some() => {
                let live = self.live.as_mut().unwrap();
                live.auto_sync = !live.auto_sync;
                if let Some(session) = &mut self.session {
                    session
                        .runtime
                        .get_or_insert_with(Default::default)
                        .auto_sync = Some(live.auto_sync);
                }
                self.dirty = true;
            }
            #[cfg(feature = "typst")]
            KeyCode::Char('f') if self.live.is_some() => {
                if let Some(location) = self.live.as_ref().and_then(|l| l.last.clone()) {
                    self.follow_source(&location);
                }
            }
            KeyCode::Char('r') => {
                #[cfg(feature = "typst")]
                if let Some(live) = &mut self.live {
                    live.refresh();
                    self.dirty = true;
                    return false;
                }
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
    #[cfg(feature = "typst")]
    fn open_typst(&mut self, path: &Path) -> Result<()> {
        let path = path.canonicalize()?;
        let mut session = compile_config::Session::load(&path, compile_config::Flags::default());
        let settings = session.initial(&path);
        let mut live = typst_live::Live::new(&path, settings.options.clone(), self.doc.revision)?;
        live.configure(&settings)?;
        // Fully validate/create the new lifecycle before dropping the old one.
        self.live.take();
        self.live = Some(live);
        self.error = session.warning.clone();
        self.session = Some(session);
        self.source = path;
        self.compile_error = None;
        self.settings = None;
        self.files.record(&self.source);
        self.files.notice = None;
        self.files.focused = false;
        self.dirty = true;
        Ok(())
    }
    #[cfg(feature = "typst")]
    fn follow_source(&mut self, location: &typst_live::SourcePoint) {
        if location.page >= self.doc.pages {
            return;
        }
        let Some(links) = self
            .doc
            .links
            .pages
            .get(location.page as usize)
            .and_then(|p| p.as_ref())
        else {
            return;
        };
        let space = links.space;
        let width = raster_width(&self.view, self.viewport);
        let (pw, ph) = space.size();
        let height = width as f64 * ph / pw;
        let px = location.x * width as f64;
        let py = location.y * height;
        if self.view.page == location.page
            && px >= self.view.x as f64
            && px < (self.view.x + self.viewport.width) as f64
            && py >= self.view.y as f64
            && py < (self.view.y + self.viewport.height) as f64
        {
            return;
        }
        let nx = (px - self.viewport.width as f64 / 3.).max(0.) / width as f64;
        let ny = (py - self.viewport.height as f64 / 3.).max(0.) / height;
        let (left, top) = space.to_pdf(nx, ny);
        self.view.page = location.page;
        self.destination = Some(Destination {
            page: location.page,
            space,
            kind: links::DestinationKind::Xyz {
                left: Some(left),
                top: Some(top),
                zoom: None,
            },
        });
        self.schedule();
        self.dirty = true;
    }
    #[cfg(feature = "typst")]
    fn consume_live(&mut self) {
        if self.opening {
            return;
        }
        let Some(live) = &mut self.live else {
            return;
        };
        let was_compiling = live.compiling;
        let update = live.poll();
        if was_compiling != live.compiling {
            self.dirty = true;
        }
        let auto_sync = live.auto_sync;
        if let Some(update) = update {
            self.compile_millis = update.millis;
            self.compile_error = update.error.clone();
            self.error = update
                .error
                .or_else(|| self.session.as_ref().and_then(|s| s.warning.clone()));
            if let Some(doc) = update.document {
                debug_assert_eq!(doc.revision, update.generation);
                self.view.set_pages(doc.pages);
                self.history.clear();
                self.destination = None;
                self.doc = Arc::new(doc);
                self.schedule();
                if auto_sync {
                    if let Some(location) = update.location {
                        self.follow_source(&location);
                    }
                }
            }
            self.dirty = true;
        }
    }
    fn reload(&mut self) {
        if self.opening {
            return;
        }
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
        #[cfg(feature = "typst")]
        if self.settings.is_some() {
            return;
        }
        let panel_width = self.files.width(self.terminal_size.width);
        if event.row >= self.terminal_size.height {
            return;
        }
        let click = event.kind == MouseEventKind::Down(MouseButton::Left);
        if panel_width > 0 && event.column < panel_width {
            if click {
                self.files.focused = true;
                let panels = self.files.panels(ratatui::layout::Rect::new(
                    0,
                    0,
                    panel_width,
                    self.terminal_size.height,
                ));
                self.files.recent_focused = event.row >= panels[1].y;
                self.pointer = None;
                self.hovered = None;
                self.hover_deadline = None;
                self.dirty = true;
            }
            return;
        }
        if self.files.focused {
            if !click {
                return;
            }
            self.files.focused = false;
            self.pointer = Some((event.column, event.row));
            self.hover_at(event.column, event.row);
            self.dirty = true;
            // The first click activates the PDF, without following a link.
            return;
        }
        self.pointer = Some((event.column, event.row));
        self.hover_at(event.column, event.row);
        let changed = match event.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let aspect = self
                    .doc
                    .links
                    .pages
                    .get(self.view.page as usize)
                    .and_then(|p| p.as_ref())
                    .map(|p| {
                        let (w, h) = p.space.size();
                        h / w
                    })
                    .or_else(|| {
                        self.displayed
                            .as_ref()
                            .filter(|g| g.page == self.view.page)
                            .map(|g| g.raster.1 as f64 / g.raster.0 as f64)
                    });
                if let Some(aspect) = aspect {
                    let anchor = (
                        f64::from(event.column.saturating_sub(panel_width).saturating_sub(1))
                            * f64::from(self.font_width)
                            + f64::from(self.font_width) / 2.,
                        f64::from(event.row.saturating_sub(1)) * f64::from(self.font_height)
                            + f64::from(self.font_height) / 2.,
                    );
                    // Five percentage points per terminal wheel tick; existing
                    // latest-wins scheduling coalesces trackpad bursts.
                    let delta = if event.kind == MouseEventKind::ScrollUp {
                        0.05
                    } else {
                        -0.05
                    };
                    self.view.zoom_at(delta, self.viewport, aspect, anchor)
                } else {
                    false
                }
            }
            // A swipe produces many ticks: bound page turns during a gesture.
            MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight
                if self
                    .last_swipe_page
                    .is_none_or(|t| t.elapsed() >= Duration::from_millis(250)) =>
            {
                self.last_swipe_page = Some(Instant::now());
                self.view
                    .step_page(if event.kind == MouseEventKind::ScrollRight {
                        1
                    } else {
                        -1
                    })
            }
            _ => false,
        };
        if changed {
            self.destination = None;
            self.schedule();
            self.dirty = true;
        }
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
            // Input can still arrive while encoding runs. Commit clamped offsets
            // only together with the matching displayed image below.
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
                    if let Some(path) = env::var_os("LEAFVIEW_TRACE") {
                        use std::io::Write;
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                        {
                            let _ = writeln!(
                                file,
                                "{}",
                                serde_json::json!({"revision": geometry.revision, "page": geometry.page, "zoom": self.view.zoom, "crop": geometry.crop, "raster": geometry.raster})
                            );
                        }
                    }
                    self.view.x = geometry.crop.0;
                    self.view.y = geometry.crop.1;
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
        #[cfg(not(feature = "typst"))]
        let live_state = "";
        #[cfg(feature = "typst")]
        let live_state = self
            .live
            .as_ref()
            .map(|live| {
                format!(
                    "{} · sync {} · {}ms · ",
                    if live.compiling || self.loading {
                        "⏳"
                    } else {
                        "✓"
                    },
                    if live.auto_sync { "on" } else { "off" },
                    self.compile_millis
                )
            })
            .unwrap_or_default();
        let state = if self.graphics_safe {
            "Halfblocks (explicit compatibility mode) · "
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
            " {}  {}/{}   {:.0}%   {}{}",
            name,
            self.view.page + 1,
            self.view.pages,
            self.view.zoom * 100.0,
            live_state,
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
    let panel_width = app.files.width(f.area().width);
    let columns =
        Layout::horizontal([Constraint::Length(panel_width), Constraint::Min(1)]).split(f.area());
    if panel_width > 0 {
        app.files.render(f, columns[0], &app.source);
    }
    let name = links::sanitize(
        &app.source
            .file_name()
            .unwrap_or(app.source.as_os_str())
            .to_string_lossy(),
    );
    let title = format!(
        "PDF · {name} · {}/{} · {:.0}%",
        app.view.page + 1,
        app.view.pages,
        app.view.zoom * 100.0
    );
    #[cfg(feature = "typst")]
    let title = if let Some(notice) = &app.settings_notice {
        format!("{title} · {}", links::sanitize(notice))
    } else {
        title
    };
    let notice = if app.files.focused {
        app.files.notice.clone().unwrap_or_default()
    } else if app.error.is_some() || app.hovered.is_some() || !app.jump.is_empty() || app.loading {
        app.status()
    } else {
        String::new()
    };
    let title = if notice.is_empty() {
        title
    } else {
        format!("{title} · {}", links::sanitize(&notice))
    };
    let pdf_block = files::panel_block(&title, !app.files.focused || panel_width == 0);
    let screen = pdf_block.inner(columns[1]);
    f.render_widget(pdf_block, columns[1]);
    f.render_widget(
        Block::new().style(Style::default().bg(Color::Rgb(20, 22, 26))),
        screen,
    );
    #[cfg(feature = "typst")]
    let compilation_failed = app.compile_error.is_some();
    #[cfg(not(feature = "typst"))]
    let compilation_failed = false;
    if let Some(image) = app.image.as_ref() {
        if screen.width == 0 || screen.height == 0 {
            return;
        }
        let size = image.size();
        let area = links::image_area(screen, size);
        app.image_area = area;
        match image {
            DisplayImage::Kitty(image) => f.render_widget(image, area),
            DisplayImage::Other(image) => f.render_widget(Image::new(image), area),
        }
    }
    if compilation_failed {
        app.hovered = None;
        app.hover_deadline = None;
        app.image_area = ratatui::layout::Rect::default();
    } else if let Some((x, y)) = app.pointer {
        app.hover_at(x, y);
    }
    #[cfg(feature = "typst")]
    if let Some(error) = &app.compile_error {
        render_compile_error(f, screen, error);
    }
    #[cfg(feature = "typst")]
    if let Some(settings) = &app.settings {
        settings.render(f);
    }
}

#[cfg(feature = "typst")]
fn render_compile_error(f: &mut Frame<'_>, screen: ratatui::layout::Rect, error: &str) {
    let width = screen
        .width
        .saturating_sub(4)
        .min(100)
        .max(screen.width.min(1));
    if width == 0 || screen.height == 0 {
        return;
    }
    let text = error
        .lines()
        .take(30)
        .map(links::sanitize)
        .collect::<Vec<_>>()
        .join("\n");
    let inner = usize::from(width.saturating_sub(4).max(1));
    let lines: usize = text
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(inner))
        .sum();
    let height = (lines.saturating_add(4).min(u16::MAX as usize) as u16).min(screen.height);
    let area = ratatui::layout::Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .style(
                Style::default()
                    .fg(Color::Rgb(255, 210, 210))
                    .bg(Color::Rgb(45, 24, 29)),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Typst — compilation error ")
                    .border_style(Style::default().fg(Color::Rgb(220, 100, 115)))
                    .padding(ratatui::widgets::Padding::uniform(1)),
            ),
        area,
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

// Low-resolution output is an explicit user choice, not an emulator blacklist.
// Automatic mode negotiates graphics equally inside and outside tmux.
fn use_safe_graphics(mode: &str) -> bool {
    matches!(mode, "safe" | "halfblocks")
}

fn viewer_loop(
    path: PathBuf,
    doc: Arc<Document>,
    mut terminal: DefaultTerminal,
    #[cfg(feature = "typst")] live: Option<typst_live::Live>,
    #[cfg(feature = "typst")] session: Option<compile_config::Session>,
) -> Result<()> {
    let tmux =
        env::var_os("TMUX").is_some() || env::var("TERM").is_ok_and(|v| v.starts_with("tmux"));
    let graphics_safe = use_safe_graphics(
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
            kitty::KittyImage::new(image, cells, image_id, tmux).map(DisplayImage::Kitty)
        } else {
            picker
                .new_protocol(image, size, Resize::Crop(None))
                .map(DisplayImage::Other)
                .map_err(anyhow::Error::from)
        };
        (geometry, result)
    });
    let reloader = Worker::new(|(path, revision): (PathBuf, u64), stop| {
        Document::open_cancel(&path, revision, || {
            stop.load(std::sync::atomic::Ordering::SeqCst)
        })
    });
    #[cfg(not(feature = "typst"))]
    let pdf_mode = true;
    #[cfg(feature = "typst")]
    let pdf_mode = live.is_none();
    let (watcher, watched) = if pdf_mode && path.is_file() {
        let (w, r) = watch(&path)?;
        (Some(w), r)
    } else {
        let (_, r) = mpsc::channel();
        (None, r)
    };
    let mut watcher = watcher;
    let mut watched = watched;
    let mut watched_path = path.clone();
    let mut files = files::Files::new(&path, pdf_mode && path.is_dir());
    if pdf_mode {
        files.record(&path);
    }
    let mut app = App {
        files,
        terminal_size: Size::default(),
        opener: Worker::new(|(path, revision): (PathBuf, u64), stop| {
            let result = Document::open_cancel(&path, revision, || {
                stop.load(std::sync::atomic::Ordering::SeqCst)
            });
            (path, result)
        }),
        opening: false,
        #[cfg(feature = "typst")]
        live,
        #[cfg(feature = "typst")]
        session,
        #[cfg(feature = "typst")]
        settings: None,
        #[cfg(feature = "typst")]
        settings_notice: None,
        #[cfg(feature = "typst")]
        compile_millis: 0,
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
        last_swipe_page: None,
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
        #[cfg(feature = "typst")]
        compile_error: None,
        dirty: true,
        loading: false,
        backend,
        graphics_safe,
        last_render_at: None,
        render_due: None,
    };
    app.geometry(terminal.size()?);
    let stopping = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut signal_ids = Vec::new();
    for signal in [
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
        signal_hook::consts::SIGINT,
    ] {
        signal_ids.push(signal_hook::flag::register(signal, stopping.clone())?);
    }
    struct Signals(Vec<signal_hook::SigId>);
    impl Drop for Signals {
        fn drop(&mut self) {
            for id in self.0.drain(..) {
                signal_hook::low_level::unregister(id);
            }
        }
    }
    let _signals = Signals(signal_ids);
    let mut reload_due: Option<Instant> = None;
    loop {
        if stopping.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        #[cfg(feature = "typst")]
        app.consume_live();
        if app.files.poll() {
            app.dirty = true;
        }
        app.flush_render();
        app.consume();
        if let Ok((path, result)) = app.opener.output.try_recv() {
            app.opening = false;
            match result {
                Ok(doc) => {
                    #[cfg(feature = "typst")]
                    {
                        app.live.take();
                        app.session = None;
                        app.settings = None;
                        app.compile_error = None;
                    }
                    app.source = path;
                    app.view = View::new(doc.pages);
                    app.doc = Arc::new(doc);
                    app.history.clear();
                    app.destination = None;
                    app.image = None;
                    app.displayed = None;
                    app.pointer = None;
                    app.error = app.doc.links.error.clone();
                    app.files.notice = None;
                    app.files.record(&app.source);
                    app.files.focused = false;
                    app.jump.clear();
                    app.schedule();
                }
                Err(error) => app.files.notice = Some(format!("Cannot open PDF: {error:#}")),
            }
            app.dirty = true;
        }
        if app.source != watched_path {
            if is_typst(&app.source) {
                watcher = None;
                let (_, receiver) = mpsc::channel();
                watched = receiver;
            } else {
                match watch(&app.source) {
                    Ok((w, r)) => {
                        watcher = Some(w);
                        watched = r;
                    }
                    Err(error) => {
                        watcher = None;
                        app.error = Some(format!("Watcher: {error}"));
                    }
                }
            }
            watched_path = app.source.clone();
            reload_due = None;
        }
        let _keep_watcher_alive = &watcher;
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
        if event::poll(Duration::from_millis(10))? {
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
    #[cfg(feature = "typst")]
    if env::args_os()
        .nth(1)
        .is_some_and(|s| s == "--typst-compiler")
    {
        return typst_compiler::serve();
    }
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
    #[cfg(feature = "typst")]
    let typst_options = if env::args_os()
        .nth(1)
        .is_some_and(|s| s == "typst" || is_typst(Path::new(&s)))
    {
        let explicit = env::args_os().nth(1).is_some_and(|s| s == "typst");
        let args: Vec<_> = env::args_os().skip(if explicit { 2 } else { 1 }).collect();
        if args.iter().any(|s| s == "--help" || s == "-h") {
            println!("{}", typst_options::HELP);
            return Ok(());
        }
        if args.iter().any(|s| s == "--version" || s == "-V") {
            println!("LeafView embedded Typst 0.15.1");
            return Ok(());
        }
        Some(compile_config::parse_cli(args)?)
    } else {
        None
    };
    let mut args = env::args_os();
    let _exe = args.next();
    #[cfg(feature = "typst")]
    let parsed_typst = typst_options.as_ref().map(|(path, _)| (path.clone(), true));
    #[cfg(not(feature = "typst"))]
    let parsed_typst = None;
    let (path, typst_mode) = if let Some(parsed) = parsed_typst {
        parsed
    } else {
        match (args.next(), args.next(), args.next(), args.next()) {
            (Some(mode), Some(path), None, None) if mode == "typst" => (PathBuf::from(path), true),
            (Some(path), None, None, None) if path == "--help" || path == "-h" => {
                println!("Usage: leafview [file.pdf|directory]\n       leafview [typst] <file.typ> [OPTIONS]\n\nWithout arguments, browse PDF/Typst files in the current directory.");
                return Ok(());
            }
            (Some(path), None, None, None) => {
                let typst = is_typst(Path::new(&path));
                (PathBuf::from(path), typst)
            }
            (None, None, None, None) => (env::current_dir()?, false),
            _ => {
                eprintln!(
                "Usage: leafview [file.pdf|directory]\n       leafview [typst] <file.typ> [OPTIONS]\n\nRequires Poppler: sudo apt install poppler-utils"
            );
                std::process::exit(2)
            }
        }
    };
    // Check dependencies and PDF *before* switching to alternate-screen mode.
    let path = path
        .canonicalize()
        .with_context(|| format!("cannot open {}", path.display()))?;
    if !path.is_file() && !(path.is_dir() && !typst_mode) {
        anyhow::bail!("not a file or PDF directory: {}", path.display());
    }
    for program in ["pdfinfo", "pdftoppm", "prlimit"] {
        if CommandCheck::missing(program) {
            anyhow::bail!(
                "required command not found: {program} (install poppler-utils and util-linux)"
            );
        }
    }
    #[cfg(not(feature = "typst"))]
    if typst_mode {
        anyhow::bail!("Typst support is not enabled; install with: cargo install --locked --features typst --path .");
    }
    #[cfg(feature = "typst")]
    let mut session = if typst_mode {
        let (_, cli) = typst_options.unwrap();
        Some(compile_config::Session::load(&path, cli))
    } else {
        None
    };
    #[cfg(feature = "typst")]
    let mut live = if let Some(session) = &mut session {
        let settings = session.initial(&path);
        let mut live = typst_live::Live::new(&path, settings.options.clone(), 1)?;
        live.configure(&settings)?;
        Some(live)
    } else {
        None
    };
    // Initial compilation runs in its owned worker before terminal entry. Errors
    // still enter the viewer with a placeholder, allowing edit/retry and quit.
    #[cfg(not(feature = "typst"))]
    let doc = if path.is_dir() {
        let placeholder = initial_placeholder()?;
        Arc::new(Document::open(placeholder.path(), 0)?)
    } else {
        Arc::new(Document::open(&path, 1)?)
    };
    #[cfg(feature = "typst")]
    let doc = if typst_mode || path.is_dir() {
        let placeholder = initial_placeholder()?;
        Arc::new(Document::open(placeholder.path(), 0)?)
    } else {
        Arc::new(Document::open(&path, 1)?)
    };
    let terminal = ratatui::try_init()?;
    let _restore = TerminalRestore;
    viewer_loop(
        path,
        doc,
        terminal,
        #[cfg(feature = "typst")]
        live.take(),
        #[cfg(feature = "typst")]
        session,
    )
}

fn is_typst(path: &Path) -> bool {
    path.extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("typ"))
}

fn initial_placeholder() -> Result<tempfile::NamedTempFile> {
    // Poppler-valid blank PDF, held only long enough for Document's snapshot.
    let mut pdf = lopdf::Document::with_version("1.5");
    let pages = pdf.new_object_id();
    let page = pdf.add_object(lopdf::dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(),0.into(),595.into(),842.into()] });
    pdf.objects.insert(
        pages,
        lopdf::dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }.into(),
    );
    let catalog = pdf.add_object(lopdf::dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let file = tempfile::NamedTempFile::new()?;
    pdf.save(file.path())?;
    Ok(file)
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
    fn low_resolution_requires_explicit_opt_in() {
        assert!(!use_safe_graphics("auto"));
        assert!(!use_safe_graphics(""));
        assert!(!use_safe_graphics("native"));
        assert!(use_safe_graphics("safe"));
        assert!(use_safe_graphics("halfblocks"));
    }
}
