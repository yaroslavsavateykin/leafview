//! Leafview: a small terminal PDF viewer with latest-wins rendering and tmux graphics support.
mod commands;
#[cfg(feature = "typst")]
mod compile_config;
#[cfg(feature = "typst")]
mod compile_settings;
mod diagnostics;
mod document;
#[cfg(feature = "editor")]
mod editor;
mod files;
mod keybindings;
mod kitty;
#[cfg(test)]
mod link_tests;
mod links;
mod picker;
mod process;
mod render;
mod terminal;
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
use ratatui::widgets::{Borders, Clear, Wrap};
use ratatui::{
    layout::{Constraint, Layout, Size},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    DefaultTerminal, Frame,
};
use ratatui_image::{protocol::Protocol, Image, Resize};
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

mod welcome;

struct App {
    keybindings: keybindings::System,
    #[cfg(feature = "editor")]
    editor: editor::Editor,
    picker: Option<picker::Picker>,
    #[cfg(feature = "editor")]
    browser_editor: bool,
    #[cfg(feature = "editor")]
    window_prefix: bool,
    workspace_hover: Option<usize>,
    split_drag: Option<u8>,
    workspace_history: Vec<(PathBuf, PathBuf)>,
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
    reload_source: Option<PathBuf>,
    reload_again: bool,
    encoding_pending: bool,
    encode_latest: Option<EncodeInput>,
    jump: String,
    error: Option<String>,
    #[cfg(feature = "typst")]
    compile_error: Option<String>,
    dirty: bool,
    loading: bool,
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
        let seq = (0..32)
            .map(|slot| {
                format!(
                    "\x1b_Ga=d,d=I,i={},q=2;\x1b\\",
                    self.id.wrapping_mul(64).wrapping_add(slot)
                )
            })
            .collect::<String>();
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
        let aspect = self.doc.page_aspect(self.view.page);
        self.view.apply_fit(self.viewport, aspect);
        if self.source.is_dir() || self.viewport.width == 0 || self.viewport.height == 0 {
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
        let panel_width = self.files.width(screen.width);
        let content = ratatui::layout::Rect::new(
            panel_width,
            0,
            screen.width.saturating_sub(panel_width),
            screen.height,
        );
        #[cfg(feature = "editor")]
        let content = {
            let area = editor_regions(
                content,
                self.editor.visible,
                self.editor.horizontal,
                self.editor.ratio,
            )
            .1;
            area
        };
        let framed = panel_width > 0;
        let area = Viewport {
            width: u32::from(content.width.saturating_sub(if framed { 2 } else { 0 }))
                .saturating_mul(self.font_width)
                .clamp(1, 3400),
            height: u32::from(content.height.saturating_sub(2))
                .saturating_mul(self.font_height)
                .clamp(1, 2400),
        };
        if area != self.viewport {
            self.viewport = area;
            self.schedule();
            self.dirty = true;
        }
    }
    #[cfg(feature = "editor")]
    fn focus_pane(&mut self, direction: char) {
        let area =
            ratatui::layout::Rect::new(0, 0, self.terminal_size.width, self.terminal_size.height);
        let width = self.files.width(area.width);
        let side = ratatui::layout::Rect::new(0, 0, width, area.height);
        let [tree, recent] = self.files.panels(side);
        let (source, pdf) = editor_regions(
            ratatui::layout::Rect::new(width, 0, area.width.saturating_sub(width), area.height),
            self.editor.visible,
            self.editor.horizontal,
            self.editor.ratio,
        );
        let panes = [tree, recent, source, pdf];
        let current = if self.editor.visible && self.editor.focused {
            2
        } else if self.files.focused && width > 0 {
            usize::from(self.files.recent_focused)
        } else {
            3
        };
        let next = editor::focus::neighbor(&panes, current, direction);
        if next < 2 {
            self.browser_editor = current == 2 || (current < 2 && self.browser_editor);
        }
        self.editor.focused = next == 2;
        self.files.focused = next < 2;
        if next < 2 {
            self.files.recent_focused = next == 1;
        }
        self.files.clear_hover();
        self.pointer = None;
        self.hovered = None;
        self.hover_deadline = None;
        self.dirty = true;
    }
    fn begin_picker(&mut self, target: picker::Target) {
        let root = self.files.folder().to_owned();
        self.picker = Some(picker::Picker::new(&root, target));
        self.dirty = true;
    }
    fn choose_file(&mut self, path: PathBuf, target: picker::Target) {
        self.picker = None;
        #[cfg(feature = "editor")]
        if target == picker::Target::Editor {
            self.editor.open(&path);
            self.files.focused = false;
            self.geometry(self.terminal_size);
            self.dirty = true;
            return;
        }
        #[cfg(not(feature = "editor"))]
        if target == picker::Target::Editor {
            self.files.notice = Some("Build with --features editor".into());
            self.dirty = true;
            return;
        }
        if is_typst(&path) {
            #[cfg(feature = "typst")]
            if let Err(e) = self.open_typst(&path) {
                self.files.notice = Some(format!("{e:#}"));
            }
            #[cfg(not(feature = "typst"))]
            {
                self.files.notice = Some("Build with --features typst".into());
            }
        } else if !self.opening
            && self
                .opener
                .input
                .as_ref()
                .unwrap()
                .try_send((path, self.doc.revision + 1))
                .is_ok()
        {
            self.opening = true;
        }
        self.dirty = true;
    }
    fn on_key(&mut self, key: KeyEvent) -> bool {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return false;
        }
        if let Some(result) = self.dispatch_bindings(key) {
            return result;
        }
        self.on_key_legacy(key)
    }
    fn on_key_legacy(&mut self, key: KeyEvent) -> bool {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return false;
        }
        if let Some(picker) = &mut self.picker {
            if key.code == KeyCode::Esc {
                self.picker = None;
            } else if key.code == KeyCode::Char('c')
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                return true;
            } else {
                let target = picker.target;
                if let Some(path) = picker.key(key) {
                    self.choose_file(path, target);
                }
            }
            self.dirty = true;
            return false;
        }
        #[cfg(feature = "typst")]
        let modal = self.settings.is_some();
        #[cfg(not(feature = "typst"))]
        let modal = false;
        if !modal
            && (key.code == KeyCode::F(3)
                || (key.code == KeyCode::Char('p')
                    && key.modifiers.contains(KeyModifiers::CONTROL)))
            && {
                #[cfg(feature = "editor")]
                {
                    !(self.editor.visible
                        && self.editor.focused
                        && (self.editor.vim.mode == editor::vim::Mode::Insert
                            || self.editor.owns_completion_key(key)))
                }
                #[cfg(not(feature = "editor"))]
                {
                    true
                }
            }
        {
            #[cfg(feature = "editor")]
            let target = if self.editor.focused || (self.files.focused && self.browser_editor) {
                picker::Target::Editor
            } else {
                picker::Target::Preview
            };
            #[cfg(not(feature = "editor"))]
            let target = picker::Target::Preview;
            self.begin_picker(target);
            return false;
        }
        #[cfg(feature = "editor")]
        if !modal
            && self.editor.visible
            && self.editor.focused
            && self.editor.owns_tool_window_key(key)
        {
            self.editor.key(key);
            self.dirty = true;
            return false;
        }
        #[cfg(feature = "editor")]
        if !modal && self.window_prefix {
            self.window_prefix = false;
            match key.code {
                KeyCode::Char(direction @ ('h' | 'j' | 'k' | 'l')) => self.focus_pane(direction),
                KeyCode::Char('w') => {
                    if self.editor.visible && self.editor.focused {
                        self.editor.focused = false;
                    } else if self.editor.visible {
                        self.editor.focused = true;
                        self.files.focused = false;
                    } else {
                        self.files.cycle_focus(self.terminal_size.width);
                    }
                }
                KeyCode::Char('v') => {
                    self.editor.horizontal = false;
                    if !self.editor.visible && !self.source.is_dir() {
                        self.editor.open(&self.source);
                    }
                }
                KeyCode::Char('s') => {
                    self.editor.horizontal = true;
                    if !self.editor.visible && !self.source.is_dir() {
                        self.editor.open(&self.source);
                    }
                }
                KeyCode::Char('c') => {
                    self.editor.visible = false;
                    self.editor.focused = false;
                }
                KeyCode::Char('=') => self.editor.ratio = 50,
                KeyCode::Char('>' | '+') => self.editor.ratio = (self.editor.ratio + 5).min(80),
                KeyCode::Char('<' | '-') => {
                    self.editor.ratio = self.editor.ratio.saturating_sub(5).max(20)
                }
                _ => {}
            }
            self.geometry(self.terminal_size);
            self.dirty = true;
            return false;
        }
        #[cfg(feature = "editor")]
        if !modal
            && key.code == KeyCode::Char('w')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && (!self.editor.focused || self.editor.vim.mode == editor::vim::Mode::Normal)
        {
            self.window_prefix = true;
            self.editor.message =
                "Ctrl+w: hjkl focus · w cycle · v/s split · <> resize · = equal · c hide".into();
            self.dirty = true;
            return false;
        }
        #[cfg(feature = "editor")]
        if key.code == KeyCode::F(2) {
            if !self.files.visible {
                self.browser_editor = self.editor.focused;
            }
            self.files.visible = !self.files.visible;
            if self.files.visible {
                self.files.focused = true;
                self.editor.focused = false;
            }
            if !self.files.visible {
                self.files.focused = false;
            }
            self.files.clear_hover();
            self.geometry(self.terminal_size);
            self.dirty = true;
            return false;
        }
        #[cfg(feature = "editor")]
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || (key.code == KeyCode::Backspace
                && (!self.editor.focused || self.editor.vim.mode == editor::vim::Mode::Normal))
        {
            let direction = match key.code {
                KeyCode::Backspace if self.editor.vim.mode != editor::vim::Mode::Insert => {
                    Some('h')
                }
                KeyCode::Char('h') => Some('h'),
                KeyCode::Char('j') => Some('j'),
                KeyCode::Char('k') => Some('k'),
                KeyCode::Char('l') => Some('l'),
                _ => None,
            };
            #[cfg(feature = "typst")]
            let modal = self.settings.is_some();
            #[cfg(not(feature = "typst"))]
            let modal = false;
            if let Some(direction) = direction.filter(|_| {
                !modal && !(self.editor.focused && self.editor.owns_completion_key(key))
            }) {
                self.focus_pane(direction);
                return false;
            }
        }
        #[cfg(feature = "editor")]
        {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if !self.editor.dirty() || self.editor.exit_confirmation {
                    return true;
                }
                self.editor.exit_confirmation = true;
                self.editor.message =
                    "Unsaved changes · Ctrl+C again: discard and exit · :w: save".into();
                self.editor.visible = true;
                self.editor.focused = true;
                self.geometry(self.terminal_size);
                self.dirty = true;
                return false;
            }
            self.editor.exit_confirmation = false;
            if key.code == KeyCode::F(4) {
                if self.editor.visible {
                    self.editor.visible = false;
                    self.editor.focused = false;
                } else {
                    self.files.visible = true;
                    self.files.focused = false;
                    if self.source.is_dir() {
                        self.begin_picker(picker::Target::Editor);
                    } else {
                        self.editor.open(&self.source);
                    }
                }
                self.geometry(self.terminal_size);
                self.dirty = true;
                return false;
            }
            if self.editor.visible && key.code == KeyCode::F(6) {
                self.editor.focused = !self.editor.focused;
                self.dirty = true;
                return false;
            }
            if self.editor.visible
                && key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(
                    key.code,
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
                )
            {
                self.files.resize(
                    key.code,
                    self.terminal_size.width,
                    self.terminal_size.height,
                );
                self.geometry(self.terminal_size);
                self.dirty = true;
                return false;
            }
            if self.editor.visible && self.editor.focused {
                self.editor.key(key);
                self.geometry(self.terminal_size);
                self.dirty = true;
                return false;
            }
        }
        if self.source.is_dir() && key.modifiers.is_empty() {
            if let KeyCode::Char(c @ '1'..='9') = key.code {
                if let Some((root, path)) = self
                    .workspace_history
                    .get(c as usize - '1' as usize)
                    .cloned()
                {
                    if !self.opening {
                        let result = if is_typst(&path) {
                            #[cfg(feature = "typst")]
                            {
                                self.open_typst(&path)
                            }
                            #[cfg(not(feature = "typst"))]
                            {
                                Err(anyhow::anyhow!(
                                    "Build with --features typst to open Typst files"
                                ))
                            }
                        } else {
                            self.opener
                                .input
                                .as_ref()
                                .unwrap()
                                .try_send((path, self.doc.revision + 1))
                                .map_err(anyhow::Error::from)
                                .map(|_| {
                                    self.opening = true;
                                })
                        };
                        match result {
                            Ok(()) => self.files.restore_folder(&root),
                            Err(error) => self.files.notice = Some(format!("{error:#}")),
                        }
                        self.dirty = true;
                    }
                }
                return false;
            }
        }
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
                        live.refresh();
                        // A diagnostic belongs to the previous configuration.
                        // Keep the last-good PDF visible while rebuilding.
                        self.compile_error = None;
                        self.error = None;
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
            self.files.clear_hover();
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
                #[cfg(feature = "editor")]
                if self.browser_editor
                    && !path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
                {
                    self.choose_file(path, picker::Target::Editor);
                    return false;
                }
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
                self.view.fit = view::Fit::Width;
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
                kitty::invalidate_terminal_cache();
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
        self.view = View::new(1);
        self.compile_error = None;
        self.compile_millis = 0;
        self.renderer.invalidate();
        self.encode_latest = None;
        self.render_due = None;
        self.image = None;
        self.displayed = None;
        self.image_area = ratatui::layout::Rect::default();
        self.pointer = None;
        self.hovered = None;
        self.hover_deadline = None;
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
            // Source synchronization must not depend on optional PDF link
            // extraction. At least navigate to the mapped page if it failed.
            self.view.page = location.page;
            self.view.x = 0;
            self.view.y = 0;
            self.destination = None;
            self.schedule();
            self.dirty = true;
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
        // Diagnostics describe the last completed attempt, not the source now
        // being compiled. Hide them during retries, including the debounce.
        if live.compiling && self.compile_error.take().is_some() {
            self.error = self.session.as_ref().and_then(|s| s.warning.clone());
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
        self.reload_source = Some(self.source.clone());
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
        if self
            .reload_source
            .take()
            .is_some_and(|path| path != self.source)
        {
            return;
        }
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
        if let Some(picker) = &mut self.picker {
            let target = picker.target;
            let path = picker.mouse(
                ratatui::layout::Rect::new(
                    0,
                    0,
                    self.terminal_size.width,
                    self.terminal_size.height,
                ),
                event.column,
                event.row,
                event.kind == MouseEventKind::Down(MouseButton::Left),
            );
            if let Some(path) = path {
                self.choose_file(path, target);
            }
            self.dirty = true;
            return;
        }
        #[cfg(feature = "typst")]
        if self.settings.is_some() {
            return;
        }
        let sidebar = self.files.width(self.terminal_size.width);
        let side_area = ratatui::layout::Rect::new(0, 0, sidebar, self.terminal_size.height);
        let content = ratatui::layout::Rect::new(
            sidebar,
            0,
            self.terminal_size.width.saturating_sub(sidebar),
            self.terminal_size.height,
        );
        #[cfg(feature = "editor")]
        let (source_area, preview_area) = editor_regions(
            content,
            self.editor.visible,
            self.editor.horizontal,
            self.editor.ratio,
        );
        #[cfg(not(feature = "editor"))]
        let preview_area = content;
        if event.kind == MouseEventKind::Up(MouseButton::Left) && self.split_drag.take().is_some() {
            return;
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left) {
            self.split_drag = None;
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left)
            && event.row == preview_area.y
            && event.column > preview_area.x + 1
            && event.column < preview_area.right().saturating_sub(1)
        {
            self.begin_picker(picker::Target::Preview);
            return;
        }
        #[cfg(feature = "editor")]
        if self.editor.visible
            && event.kind == MouseEventKind::Down(MouseButton::Left)
            && event.row == source_area.y
            && event.column > source_area.x + 1
            && event.column < source_area.right().saturating_sub(1)
        {
            self.begin_picker(picker::Target::Editor);
            return;
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left) {
            if sidebar > 0 && event.column.abs_diff(sidebar) <= 1 {
                self.split_drag = Some(0);
            } else if sidebar > 0
                && event.column < sidebar
                && event.row.abs_diff(self.files.panels(side_area)[1].y) <= 1
            {
                self.split_drag = Some(1);
            }
            #[cfg(feature = "editor")]
            if self.split_drag.is_none()
                && self.editor.visible
                && (if self.editor.horizontal {
                    event.row.abs_diff(source_area.bottom()) <= 1 && event.column >= sidebar
                } else {
                    event.column.abs_diff(source_area.right()) <= 1
                })
            {
                self.split_drag = Some(2);
            }
        }
        if matches!(
            event.kind,
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved
        ) {
            if let Some(split) = self.split_drag {
                if split < 2 {
                    self.files.drag_split(event.column, event.row, split == 0);
                }
                #[cfg(feature = "editor")]
                if split == 2 {
                    let (pos, size) = if self.editor.horizontal {
                        (event.row, content.height)
                    } else {
                        (event.column.saturating_sub(sidebar), content.width)
                    };
                    self.editor.ratio =
                        ((u32::from(pos) * 100 / u32::from(size.max(1))) as u16).clamp(20, 80);
                }
                self.geometry(self.terminal_size);
                self.dirty = true;
                return;
            }
        }
        if self.split_drag.is_some() {
            return;
        }
        if self.source.is_dir() {
            let hovered = welcome::hit(
                preview_area,
                self.workspace_history.len(),
                event.column,
                event.row,
            );
            if self.workspace_hover != hovered {
                self.workspace_hover = hovered;
                self.dirty = true;
            }
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                if let Some(index) = hovered {
                    self.on_key(KeyEvent::from(KeyCode::Char(char::from(
                        b'1' + index as u8,
                    ))));
                    return;
                }
            }
        }
        #[cfg(feature = "editor")]
        if self.editor.visible {
            let sidebar = self.files.width(self.terminal_size.width);
            let (source, _) = editor_regions(
                ratatui::layout::Rect::new(
                    sidebar,
                    0,
                    self.terminal_size.width.saturating_sub(sidebar),
                    self.terminal_size.height,
                ),
                true,
                self.editor.horizontal,
                self.editor.ratio,
            );
            if source.contains((event.column, event.row).into()) {
                if event.kind == MouseEventKind::Down(MouseButton::Left) {
                    self.editor.focused = true;
                    self.files.focused = false;
                    self.dirty = true;
                }
                self.editor.mouse(event, source);
                self.pointer = None;
                self.hovered = None;
                self.dirty = true;
                return;
            }
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                if event.column < sidebar && self.editor.focused {
                    self.browser_editor = true;
                } else if event.column >= sidebar {
                    self.browser_editor = false;
                }
                self.editor.focused = false;
                self.dirty = true;
            }
        }
        #[cfg(feature = "typst")]
        if self.settings.is_some() {
            return;
        }
        let panel_width = self.files.width(self.terminal_size.width);
        if matches!(event.kind, MouseEventKind::Moved)
            && self.files.hover_recent(
                ratatui::layout::Rect::new(0, 0, panel_width, self.terminal_size.height),
                event.column,
                event.row,
            )
        {
            self.dirty = true;
        }
        if event.row >= self.terminal_size.height {
            return;
        }
        let click = event.kind == MouseEventKind::Down(MouseButton::Left);
        if panel_width > 0 && event.column < panel_width {
            let selected = self.files.select_at(side_area, event.column, event.row);
            if selected {
                self.dirty = true;
            }
            if click {
                #[cfg(feature = "editor")]
                if self.editor.focused {
                    self.browser_editor = true;
                }
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
                if selected {
                    self.on_non_focus_key(KeyEvent::from(KeyCode::Enter));
                }
            }
            return;
        }
        if self.files.focused && !matches!(event.kind, MouseEventKind::Moved) {
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
                self.view.fit = view::Fit::Manual;
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
                        if geometry.document_id != self.doc.id {
                            continue;
                        }
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
                || geometry.document_id != self.doc.id
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
                if input.0.generation == self.renderer.current_generation()
                    && input.0.document_id == self.doc.id
                {
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
    fn footer_notice(&self) -> String {
        if let Some(notice) = &self.files.notice {
            return links::sanitize(notice);
        }
        #[cfg(feature = "typst")]
        {
            if self.live.as_ref().is_some_and(|live| live.compiling) {
                let live = self.live.as_ref().unwrap();
                return if live.iteration > 0 {
                    format!("Typst watch · updating · previous {} ms", live.last_millis)
                } else {
                    "Typst watch · starting…".into()
                };
            }
            if let Some(notice) = &self.settings_notice {
                return links::sanitize(notice);
            }
            // Full diagnostics already appear in the error panel.
            if self.compile_error.is_some() {
                return String::new();
            }
        }
        if self.loading {
            return "Loading page…".into();
        }
        if !self.jump.is_empty() {
            return format!(":{} · Enter: go to page", self.jump);
        }
        #[cfg(feature = "typst")]
        if self.error.is_none() {
            if let Some(live) = &self.live {
                return format!(
                    "Typst watch · {} ms · pass {}",
                    live.last_millis, live.iteration
                );
            }
        }
        self.error
            .as_ref()
            .map(|error| links::sanitize(error))
            .unwrap_or_default()
    }
}

#[cfg(feature = "editor")]
fn editor_regions(
    area: ratatui::layout::Rect,
    visible: bool,
    horizontal: bool,
    ratio: u16,
) -> (ratatui::layout::Rect, ratatui::layout::Rect) {
    if !visible {
        return (ratatui::layout::Rect::default(), area);
    }
    let layout = if horizontal {
        Layout::vertical([
            Constraint::Percentage(ratio.clamp(20, 80)),
            Constraint::Min(0),
        ])
    } else {
        Layout::horizontal([
            Constraint::Percentage(ratio.clamp(20, 80)),
            Constraint::Min(0),
        ])
    };
    let parts = layout.split(area);
    (parts[0], parts[1])
}

fn ui(f: &mut Frame<'_>, app: &mut App) {
    let area = f.area();
    let panel_width = app.files.width(area.width);
    let parts =
        Layout::horizontal([Constraint::Length(panel_width), Constraint::Min(1)]).split(area);
    #[cfg(feature = "editor")]
    let preview = {
        let (source, preview) = editor_regions(
            parts[1],
            app.editor.visible,
            app.editor.horizontal,
            app.editor.ratio,
        );
        if app.editor.visible {
            app.editor.render(f, source);
        }
        preview
    };
    #[cfg(not(feature = "editor"))]
    let preview = parts[1];
    let columns = [parts[0], preview];
    if panel_width > 0 {
        app.files.render(f, columns[0], &app.source);
    }
    if app.source.is_dir() {
        welcome::render(
            f,
            columns[1],
            app.files.folder(),
            &app.workspace_history,
            app.workspace_hover,
        );
        app.files.render_recent_info(f, columns[1]);
        if let Some(picker) = &app.picker {
            picker.render(f);
        }
        return;
    }
    let name = links::sanitize(
        &app.source
            .file_name()
            .unwrap_or(app.source.as_os_str())
            .to_string_lossy(),
    );
    let title = format!(
        "PDF · {name} · {}/{} · {:.0}%{} · ▾ F3",
        app.view.page + 1,
        app.view.pages,
        app.view.zoom * 100.0,
        match app.view.fit {
            view::Fit::Page => " · fit page",
            view::Fit::Width => " · fit width",
            view::Fit::Manual => "",
        }
    );
    let pdf_block = document_block(
        &title,
        panel_width > 0,
        (!app.files.focused || panel_width == 0) && {
            #[cfg(feature = "editor")]
            {
                !app.editor.focused
            }
            #[cfg(not(feature = "editor"))]
            {
                true
            }
        },
    );
    let mut screen = pdf_block.inner(columns[1]);
    if panel_width == 0 {
        screen.height = screen.height.saturating_sub(1);
    }
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
    if let Some(action) = app.hovered.and_then(|id| {
        app.displayed
            .as_ref()?
            .links
            .as_ref()?
            .links
            .get(id)
            .map(|link| &link.action)
    }) {
        render_link_hint(f, columns[1], action);
    } else {
        let notice = app.footer_notice();
        render_footer_notice(f, columns[1], &notice);
    }
    #[cfg(feature = "typst")]
    if let Some(error) = &app.compile_error {
        render_compile_error(f, screen, error);
    }
    app.files.render_recent_info(f, columns[1]);
    if let Some(picker) = &app.picker {
        picker.render(f);
    }
    #[cfg(feature = "typst")]
    if let Some(settings) = &app.settings {
        settings.render(f);
    }
}

fn document_block(title: &str, framed: bool, active: bool) -> Block<'static> {
    if framed {
        files::panel_block(title, active)
    } else {
        Block::default().title(
            Line::from(format!(" {title} ")).style(
                Style::default()
                    .fg(Color::Rgb(180, 192, 211))
                    .bg(Color::Rgb(25, 29, 36)),
            ),
        )
    }
}

fn render_footer_notice(f: &mut Frame<'_>, panel: ratatui::layout::Rect, notice: &str) {
    if notice.is_empty() || panel.height < 2 || panel.width < 4 {
        return;
    }
    let area = ratatui::layout::Rect::new(panel.x + 1, panel.bottom() - 1, panel.width - 2, 1);
    f.render_widget(
        Paragraph::new(format!(" {} ", links::sanitize(notice))).style(
            Style::default()
                .fg(Color::Rgb(180, 192, 211))
                .bg(Color::Rgb(20, 22, 26)),
        ),
        area,
    );
}

// The footer is outside Kitty placements, including the borderless layout.
fn render_link_hint(f: &mut Frame<'_>, panel: ratatui::layout::Rect, action: &LinkAction) {
    if panel.width < 8 || panel.height < 2 {
        return;
    }
    let (label, detail, color) = match action {
        LinkAction::External(url) => (
            "↗ Click to open",
            links::sanitize(url),
            Color::Rgb(110, 205, 192),
        ),
        LinkAction::Internal(destination) => (
            "→ Click to follow",
            format!("Page {} · b: back", destination.page + 1),
            Color::Rgb(110, 205, 192),
        ),
        LinkAction::Unsupported(reason) => (
            "Link unavailable",
            links::sanitize(reason),
            Color::Rgb(224, 182, 112),
        ),
    };
    let hint = Line::from(vec![
        Span::styled(format!(" {label} "), Style::default().fg(color)),
        Span::styled(
            format!("· {detail} "),
            Style::default().fg(Color::Rgb(180, 192, 211)),
        ),
    ]);
    let width = (hint.width().min(u16::MAX as usize) as u16).min(panel.width.saturating_sub(4));
    let area = ratatui::layout::Rect::new(panel.x + 2, panel.bottom() - 1, width, 1);
    f.render_widget(
        Paragraph::new(hint).style(Style::default().bg(Color::Rgb(20, 22, 26))),
        area,
    );
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
    let picker = terminal::picker()?;
    if env::var("LEAFVIEW_DEBUG").is_ok() {
        diagnostics::log(format_args!(
            "leafview: protocol={:?} font={:?} tmux_env={:?} term={:?} term_program={:?} graphics_safe={graphics_safe}",
            picker.protocol_type(),
            picker.font_size(),
            env::var("TMUX").is_ok(),
            env::var("TERM").ok(),
            env::var("TERM_PROGRAM").ok(),
        ));
    }
    // Do not feed unsolicited mouse/focus reports into the capability parser.
    // Enable reporting only after its terminal query has finished.
    crossterm::execute!(
        std::io::stdout(),
        event::EnableFocusChange,
        event::EnableMouseCapture
    )?;
    let font = picker.font_size();
    let (tx_completed, completed) = mpsc::sync_channel(1);
    let renderer = Renderer::new(tx_completed)?;
    let graphics = GraphicsCleanup::new(
        picker.protocol_type() == ratatui_image::picker::ProtocolType::Kitty,
        tmux,
    );
    // Fixed process-owned slots bound terminal memory and permit upload-free
    // revisits. Reusing an evicted slot replaces its terminal image atomically.
    let mut free_image_ids: Vec<u32> = (0..32)
        .rev()
        .map(|slot| graphics.id.wrapping_mul(64).wrapping_add(slot))
        .collect();
    let mut encoded_cache = lru::LruCache::new(std::num::NonZeroUsize::new(32).unwrap());
    let mut encoded_bytes: usize = 0;
    let encoder = Worker::new(move |(geometry, image, _size): EncodeInput, _| {
        let started = Instant::now();
        let key = (
            geometry.document_id,
            geometry.page,
            geometry.raster,
            geometry.crop,
        );
        if let Some(cached) = encoded_cache.get(&key) {
            let cached: &kitty::KittyImage = cached;
            return (geometry, Ok(DisplayImage::Kitty(cached.replay())));
        }
        // Pad the last terminal cell instead of stretching the PDF pixels into
        // ceil-sized Kitty placements. All backends now have identical pixel scale.
        let cells = Size::new(
            image.width().div_ceil(u32::from(font.width.max(1))) as u16,
            image.height().div_ceil(u32::from(font.height.max(1))) as u16,
        );
        let padded_width = u32::from(cells.width) * u32::from(font.width.max(1));
        let padded_height = u32::from(cells.height) * u32::from(font.height.max(1));
        let image = if image.width() == padded_width && image.height() == padded_height {
            image
        } else {
            // Raster pixels are opaque. Row copies avoid per-pixel alpha blending
            // across the entire viewport merely to pad its final terminal cell.
            let source = image.to_rgba8();
            let mut padded = image::RgbaImage::new(padded_width, padded_height);
            let stride = padded_width as usize * 4;
            let source_stride = source.width() as usize * 4;
            let pixels: &mut [u8] = padded.as_mut();
            for (destination, source) in pixels
                .chunks_mut(stride)
                .zip(source.as_raw().chunks(source_stride))
            {
                destination[..source_stride].copy_from_slice(source);
            }
            image::DynamicImage::ImageRgba8(padded)
        };
        let size = cells;
        let result = if picker.protocol_type() == ratatui_image::picker::ProtocolType::Kitty {
            if free_image_ids.is_empty() {
                if let Some((_, old)) = encoded_cache.pop_lru() {
                    encoded_bytes -= old.encoded_len();
                    free_image_ids.push(old.id());
                }
            }
            let image_id = free_image_ids.pop().expect("bounded Kitty image slot");
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
        if let Ok(DisplayImage::Kitty(image)) = &result {
            let bytes = image.encoded_len();
            const ENCODED_LIMIT: usize = 64 * 1024 * 1024;
            if bytes <= ENCODED_LIMIT {
                while encoded_bytes + bytes > ENCODED_LIMIT || encoded_cache.len() == 32 {
                    if let Some((_, old)) = encoded_cache.pop_lru() {
                        encoded_bytes -= old.encoded_len();
                        free_image_ids.push(old.id());
                    } else {
                        break;
                    }
                }
                encoded_bytes += bytes;
                encoded_cache.put(key, image.replay());
            } else {
                // Oversized transfers are not cached; their slot is reusable
                // once this frame has been shown and the next encoding starts.
                free_image_ids.push(image.id());
            }
        }
        if env::var_os("LEAFVIEW_PERF").is_some() {
            diagnostics::log(format_args!(
                "leafview encode page={} elapsed_ms={:.2}",
                geometry.page + 1,
                started.elapsed().as_secs_f64() * 1000.
            ));
        }
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
    let initial_view = View::new(doc.pages);
    let mut app = App {
        keybindings: keybindings::System::new(if path.is_dir() {
            &path
        } else {
            path.parent().unwrap_or(Path::new("."))
        }),
        #[cfg(feature = "editor")]
        editor: editor::Editor::new(),
        picker: None,
        #[cfg(feature = "editor")]
        browser_editor: false,
        #[cfg(feature = "editor")]
        window_prefix: false,
        workspace_hover: None,
        split_drag: None,
        workspace_history: files.workspace_history(),
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
        view: initial_view,
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
        reload_source: None,
        reload_again: false,
        encoding_pending: false,
        encode_latest: None,
        jump: String::new(),
        error: doc.links.error.clone(),
        #[cfg(feature = "typst")]
        compile_error: None,
        dirty: true,
        loading: false,
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
        if app.keybindings.poll_reload() {
            app.dirty = true;
        }
        if let Some(resolution) = app.keybindings.engine.expire(Instant::now()) {
            if app.resolve_binding(resolution) == Some(true) {
                break;
            }
        }
        if stopping.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        #[cfg(feature = "editor")]
        if app.editor.poll() | app.editor.tick() {
            app.dirty = true;
        }
        #[cfg(all(feature = "editor", feature = "typst"))]
        if let Some((path, byte)) = app.editor.saved_location.take() {
            if let Some(live) = &mut app.live {
                live.prefer_source(path, byte);
            }
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
            terminal.draw(|f| {
                ui(f, &mut app);
                app.keybindings.render(f);
            })?;
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
                    app.files.clear_hover();
                    app.pointer = None;
                    app.hovered = None;
                    app.hover_deadline = None;
                    app.dirty = true;
                }
                Event::FocusGained => {
                    kitty::invalidate_terminal_cache();
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
    fn link_hint_renders_on_bottom_border_and_fits_small_panels() {
        use super::*;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(64, 6)).unwrap();
        terminal
            .draw(|f| {
                render_link_hint(
                    f,
                    f.area(),
                    &LinkAction::External("https://example.org".into()),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let rendered = (0..6)
            .map(|y| (0..64).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.lines().last().unwrap().contains("Click to open"));
        assert!(rendered.contains("https://example.org"));
        assert!(rendered.lines().take(5).all(|line| line.trim().is_empty()));
        if let Ok(path) = std::env::var("LEAFVIEW_HINT_SNAPSHOT") {
            std::fs::write(path, rendered).unwrap();
        }
        for width in 0..20 {
            terminal
                .draw(|f| {
                    render_link_hint(
                        f,
                        ratatui::layout::Rect::new(0, 0, width, 2),
                        &LinkAction::Unsupported("unsupported action".into()),
                    );
                })
                .unwrap();
        }
    }

    #[test]
    fn document_without_sidebar_has_only_a_header() {
        use super::*;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 8)).unwrap();
        terminal
            .draw(|f| {
                let block = document_block("PDF · test.pdf", false, true);
                assert_eq!(
                    block.inner(f.area()),
                    ratatui::layout::Rect::new(0, 1, 40, 7)
                );
                f.render_widget(block, f.area());
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let snapshot = (0..8)
            .map(|y| (0..40).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(snapshot.contains("PDF · test.pdf"));
        assert!(!snapshot.contains(['│', '─', '┌', '┘']));
        if let Ok(path) = std::env::var("LEAFVIEW_HEADER_SNAPSHOT") {
            std::fs::write(path, snapshot).unwrap();
        }
    }

    #[test]
    fn footer_notice_stays_below_document() {
        use super::*;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 8)).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(document_block("PDF · document.pdf", false, true), f.area());
                render_footer_notice(f, f.area(), "Compiling…");
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let rows = (0..8)
            .map(|y| (0..60).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>();
        assert!(rows[0].contains("document.pdf"));
        assert!(!rows[0].contains("Compiling"));
        assert!(rows[7].contains("Compiling"));
        if let Ok(path) = std::env::var("LEAFVIEW_FOOTER_SNAPSHOT") {
            std::fs::write(path, rows.join("\n")).unwrap();
        }
    }

    #[cfg(feature = "editor")]
    #[test]
    fn source_is_between_tree_and_pdf() {
        for width in [0, 1, 48, 80, 120, 201] {
            let sidebar = if width >= 48 {
                (width / 3).clamp(22, 38)
            } else {
                0
            };
            let content = ratatui::layout::Rect::new(sidebar, 0, width - sidebar, 30);
            let (source, pdf) = super::editor_regions(content, true, false, 50);
            assert_eq!(source.x, sidebar);
            assert_eq!(source.right(), pdf.x);
            assert_eq!(pdf.right(), width);
        }
    }

    #[test]
    fn low_resolution_requires_explicit_opt_in() {
        assert!(!use_safe_graphics("auto"));
        assert!(!use_safe_graphics(""));
        assert!(!use_safe_graphics("native"));
        assert!(use_safe_graphics("safe"));
        assert!(use_safe_graphics("halfblocks"));
    }
}
