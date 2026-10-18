//! Leafview: a small terminal PDF viewer with latest-wins rendering and tmux graphics support.
mod document;
mod render;
mod view;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use document::Document;
use notify::Watcher;
use ratatui::{layout::{Constraint, Layout, Rect}, style::{Color, Style}, widgets::{Block, Paragraph}, DefaultTerminal, Frame};
use ratatui_image::{picker::Picker, thread::{ResizeRequest, ResizeResponse, ThreadProtocol}, Resize, StatefulImage};
use render::{Completed, Renderer, Request};
use std::{env, path::{Path, PathBuf}, sync::{mpsc::{self, Receiver}, Arc}, thread, time::{Duration, Instant}};
use view::{raster_width, View, Viewport};

struct App {
    doc: Arc<Document>,
    view: View,
    source: PathBuf,
    font_width: u32,
    font_height: u32,
    viewport: Viewport,
    renderer: Renderer,
    image: ThreadProtocol,
    completed: Receiver<Completed>,
    resized: Receiver<Result<ResizeResponse, ratatui_image::errors::Errors>>,
    picker: Picker,
    jump: String,
    error: Option<String>,
    dirty: bool,
    loading: bool,
}

impl App {
    fn schedule(&mut self) {
        if self.viewport.width == 0 || self.viewport.height == 0 { return; }
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
            width: u32::from(screen.width).saturating_mul(self.font_width).max(1),
            height: u32::from(screen.height.saturating_sub(1)).saturating_mul(self.font_height).max(1),
        };
        if area != self.viewport { self.viewport = area; self.schedule(); self.dirty = true; }
    }
    fn on_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat { return false; }
        let mut request = false;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc if self.jump.is_empty() => return true,
            KeyCode::Esc => { self.jump.clear(); self.dirty = true; }
            KeyCode::Char(c @ '0'..='9') => { if self.jump.len() < 8 { self.jump.push(c); self.dirty = true; } }
            KeyCode::Backspace => { self.jump.pop(); self.dirty = true; }
            KeyCode::Enter if !self.jump.is_empty() => {
                if let Ok(page) = self.jump.parse::<u32>() { request = self.view.go_to(page); }
                self.jump.clear(); self.dirty = true;
            }
            KeyCode::Char('l') | KeyCode::Char('n') | KeyCode::Right | KeyCode::PageDown | KeyCode::Char(' ') => request = self.view.step_page(1),
            KeyCode::Char('h') | KeyCode::Char('p') | KeyCode::Left | KeyCode::PageUp => request = self.view.step_page(-1),
            KeyCode::Char('g') | KeyCode::Home => request = self.view.go_to(1),
            KeyCode::Char('G') | KeyCode::End => request = self.view.go_to(self.view.pages),
            KeyCode::Char('j') | KeyCode::Down => { self.view.scroll(0, 110); request = true; }
            KeyCode::Char('k') | KeyCode::Up => { self.view.scroll(0, -110); request = true; }
            KeyCode::Char('H') => { self.view.scroll(-100, 0); request = true; }
            KeyCode::Char('L') => { self.view.scroll(100, 0); request = true; }
            KeyCode::Char('+') | KeyCode::Char('=') => request = self.view.adjust_zoom(0.25),
            KeyCode::Char('-') | KeyCode::Char('_') => request = self.view.adjust_zoom(-0.25),
            KeyCode::Char('z') => { self.view.zoom = 1.0; self.view.x = 0; self.view.y = 0; request = true; }
            KeyCode::Char('r') => {
                self.reload();
                self.dirty = true;
            }
            _ => {}
        }
        if request { self.schedule(); self.dirty = true; }
        false
    }
    fn reload(&mut self) {
        match Document::open(&self.source, self.doc.revision + 1) {
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
        while let Ok(completed) = self.completed.try_recv() {
            if completed.generation != self.renderer.current_generation() { continue; }
            self.loading = false;
            self.view.x = completed.x;
            self.view.y = completed.y;
            match completed.image {
                Ok(image) => { self.image.replace_protocol(self.picker.new_resize_protocol(image)); self.error = None; }
                Err(err) => self.error = Some(err),
            }
            self.dirty = true;
        }
        while let Ok(resized) = self.resized.try_recv() {
            match resized {
                Ok(proto) => { let _ = self.image.update_resized_protocol(proto); }
                Err(err) => self.error = Some(format!("Image encoding: {err}")),
            }
            self.dirty = true;
        }
    }
    fn status(&self) -> String {
        let name = self.source.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_else(|| "document.pdf".into());
        if !self.jump.is_empty() { return format!(" :{}  Enter: go to page  |  Esc: cancel", self.jump); }
        if let Some(err) = &self.error {
            return format!(" {}  {}/{}  ERROR: {}", name, self.view.page + 1, self.view.pages, abbreviate(err, 110));
        }
        let state = if self.loading { "Rendering…  " } else { "" };
        format!(" {}  {}/{}   {:.0}%   {}j/k scroll · h/l pages · +/- zoom · r reload · q quit", name, self.view.page + 1, self.view.pages, self.view.zoom * 100.0, state)
    }
}

fn abbreviate(text: &str, max_chars: usize) -> String {
    let mut text = text.chars();
    let mut out: String = text.by_ref().take(max_chars).collect();
    if text.next().is_some() { out.push('…'); }
    out
}

fn ui(f: &mut Frame<'_>, app: &mut App) {
    let parts = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(f.area());
    let screen = parts[0];
    f.render_widget(Block::new().style(Style::default().bg(Color::Rgb(20, 22, 26))), screen);
    if app.loading {
        f.render_widget(Paragraph::new("  Rendering page…").style(Style::default().fg(Color::DarkGray)), screen);
    } else if screen.width > 0 && screen.height > 0 {
        f.render_stateful_widget(StatefulImage::new().resize(Resize::Crop(None)), screen, &mut app.image);
    }
    f.render_widget(Paragraph::new(app.status()).style(Style::default().fg(Color::Rgb(195, 199, 208)).bg(Color::Rgb(35, 38, 44))), parts[1]);
}

fn watch(source: &Path) -> Result<(notify::RecommendedWatcher, Receiver<notify::Result<notify::Event>>)> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| { let _ = tx.send(res); })?;
    let parent = source.parent().context("PDF has no parent directory")?;
    watcher.watch(parent, notify::RecursiveMode::NonRecursive)?;
    Ok((watcher,rx))
}

fn viewer_loop(path: PathBuf, mut terminal: DefaultTerminal) -> Result<()> {
    let doc = Arc::new(Document::open(&path, 1)?);
    // Picker queries must occur after alternate-screen entry, before event polling.
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let font = picker.font_size();
    let (tx_completed, completed) = mpsc::channel();
    let renderer = Renderer::new(tx_completed)?;
    let (tx_resize, rec_resize) = mpsc::channel::<ResizeRequest>();
    let (tx_resized, resized) = mpsc::channel();
    // Graphics encoding is performed away from the input/event loop.
    thread::spawn(move || {
        while let Ok(request) = rec_resize.recv() {
            if tx_resized.send(request.resize_encode()).is_err() { break; }
        }
    });
    let (watcher, watched) = watch(&path)?;
    let _watcher = watcher; // Keep the OS watcher alive for the duration of the viewer.
    let mut app = App {
        source: path,
        view: View::new(doc.pages),
        doc,
        font_width: u32::from(font.width.max(1)),
        font_height: u32::from(font.height.max(1)),
        viewport: Viewport { width: 0, height: 0 },
        renderer,
        image: ThreadProtocol::new(tx_resize, None),
        completed,
        resized,
        picker,
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
                    if e.paths.iter().any(|p| p.file_name() == app.source.file_name()) {
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
                Event::Key(key) => if app.on_key(key) { break; },
                Event::Resize(_,_) => app.geometry(terminal.size()?),
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
        (Some(path),None) => PathBuf::from(path),
        _ => {
            eprintln!("Usage: leafview <file.pdf>\n\nRequires Poppler: sudo apt install poppler-utils");
            std::process::exit(2)
        }
    };
    // Check dependencies and PDF *before* switching to alternate-screen mode.
    let path = path.canonicalize().with_context(|| format!("cannot open {}",path.display()))?;
    if !path.is_file() { anyhow::bail!("not a file: {}",path.display()); }
    for program in ["pdfinfo", "pdftoppm", "timeout"] {
        if CommandCheck::missing(program) { anyhow::bail!("required command not found: {program} (install poppler-utils/coreutils)"); }
    }
    let _ = Document::open(&path,0)?;
    let terminal = ratatui::init();
    let result = viewer_loop(path, terminal);
    ratatui::restore();
    result
}

struct CommandCheck;
impl CommandCheck {
    fn missing(command: &str) -> bool {
        std::process::Command::new(command).arg("--help").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_err()
    }
}
