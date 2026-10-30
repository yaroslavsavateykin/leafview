pub mod buffer;
mod commands;
mod completion;
mod completion_rank;
mod completion_render;
mod completion_sources;
#[cfg(test)]
mod completion_tests;
pub mod focus;
#[cfg(test)]
mod interaction_tests;
mod interface;
mod lsp;
mod render;
mod signature;
mod syntax;
mod tasks;
#[cfg(test)]
mod tests;
pub mod vim;
mod windows;
use buffer::Buffer;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::path::{Path, PathBuf};
use tasks::{Reply, Task};

pub struct Editor {
    pub visible: bool,
    pub focused: bool,
    pub horizontal: bool,
    pub ratio: u16,
    pub exit_confirmation: bool,
    pub format_on_save: bool,
    save_after_format: Option<(PathBuf, u64)>,
    window_prefix: bool,
    completion: Option<(usize, usize, Vec<String>, usize)>,
    pub buffers: Vec<Buffer>,
    pub current: usize,
    pub vim: vim::Vim,
    worker: crate::workers::Worker<Task, anyhow::Result<Reply>>,
    pub busy: bool,
    pub message: String,
    next_open: Option<PathBuf>,
    mouse_anchor: Option<usize>,
    pending_jump: Option<(PathBuf, usize, usize)>,
    popup: Option<(String, Vec<serde_json::Value>, usize)>,
    pub popup_text: Vec<String>,
    lsp_worker: crate::workers::Worker<Task, anyhow::Result<Reply>>,
    lsp_pending: Option<(PathBuf, u64, usize, bool)>,
    lsp_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    auto_due: Option<(std::time::Instant, String)>,
    signature_due: Option<std::time::Instant>,
    signature: signature::State,
    windows: windows::Windows,
    lsp_command_pending: String,
    pub autocomplete: bool,
    pub completion_config: completion::Config,
    cmp: completion::State,
    pub viewport_height: usize,
    pub saved_location: Option<(PathBuf, usize)>,
}
impl Editor {
    pub fn new() -> Self {
        let lsp_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (completion_config, config_warning) = match completion::Config::load() {
            Ok(config) => (config, String::new()),
            Err(error) => (
                completion::Config::default(),
                format!("Completion config: {error}"),
            ),
        };
        Self {
            visible: false,
            focused: false,
            horizontal: false,
            ratio: 50,
            exit_confirmation: false,
            format_on_save: true,
            save_after_format: None,
            window_prefix: false,
            completion: None,
            buffers: vec![],
            current: 0,
            vim: vim::Vim::default(),
            worker: crate::workers::Worker::new({
                let mut runner = tasks::Runner::default();
                move |task, stop| runner.run(task, stop)
            }),
            busy: false,
            message: config_warning,
            next_open: None,
            mouse_anchor: None,
            pending_jump: None,
            popup: None,
            popup_text: vec![],
            lsp_worker: crate::workers::Worker::new({
                let mut runner = tasks::Runner::default();
                runner.cancel = lsp_cancel.clone();
                move |task, stop| runner.run(task, stop)
            }),
            lsp_pending: None,
            lsp_cancel,
            auto_due: None,
            signature_due: None,
            signature: signature::State::default(),
            windows: windows::Windows::default(),
            lsp_command_pending: String::new(),
            autocomplete: true,
            completion_config,
            cmp: completion::State::default(),
            viewport_height: 20,
            saved_location: None,
        }
    }
    pub fn open(&mut self, path: &Path) {
        self.visible = true;
        self.focused = true;
        if let Some(i) = self.buffers.iter().position(|b| b.path == path) {
            self.clear_tools();
            self.current = i;
            self.vim.reset_mode();
            self.auto_due = None;
            self.popup = None;
            self.popup_text.clear();
            return;
        }
        if self.busy {
            self.next_open = Some(path.to_owned());
            return;
        }
        if self
            .worker
            .input
            .as_ref()
            .unwrap()
            .try_send(Task::Open(path.to_owned()))
            .is_ok()
        {
            self.busy = true;
            self.message = "Opening source…".into();
        }
    }
    pub fn mouse(&mut self, event: MouseEvent, area: ratatui::layout::Rect) {
        if event.row == area.y + 1 && event.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some((index, x, width)) = self
                .tab_regions(area)
                .into_iter()
                .find(|(_, x, width)| event.column >= *x && event.column < x.saturating_add(*width))
            {
                if event.column >= x.saturating_add(width).saturating_sub(3) {
                    self.current = index;
                    self.close_buffer(false);
                } else {
                    self.clear_tools();
                    self.current = index;
                    self.vim.reset_mode();
                }
            }
            return;
        }
        if event.row < area.y + 2 || event.row >= area.bottom().saturating_sub(2) {
            return;
        }
        let Some(b) = self.buffers.get_mut(self.current) else {
            return;
        };
        let height = area.height.saturating_sub(4) as usize;
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
                let row = (b.scroll + usize::from(event.row.saturating_sub(area.y + 2)))
                    .min(b.state.doc.len_lines() - 1);
                let target = usize::from(event.column.saturating_sub(area.x + 6));
                let mut width = 0;
                let mut pos = b.start(row);
                for ch in b.state.doc.line(row).chars() {
                    if matches!(ch, '\n' | '\r') {
                        break;
                    }
                    let w = if ch == '\t' {
                        4
                    } else {
                        unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0)
                    };
                    if width + w > target {
                        break;
                    }
                    width += w;
                    pos += 1;
                }
                if event.kind == MouseEventKind::Down(MouseButton::Left) {
                    self.mouse_anchor = Some(pos);
                    if self.vim.mode != vim::Mode::Insert {
                        self.vim.reset_mode();
                    }
                } else if let Some(anchor) = self.mouse_anchor {
                    self.vim.mouse_select(anchor);
                }
                b.set_cursor(pos);
            }
            MouseEventKind::Up(MouseButton::Left) => self.mouse_anchor = None,
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                b.scroll = if event.kind == MouseEventKind::ScrollDown {
                    (b.scroll + 3).min(b.state.doc.len_lines().saturating_sub(height.max(1)))
                } else {
                    b.scroll.saturating_sub(3)
                };
                let row = b.line();
                if row < b.scroll {
                    b.set_cursor(b.start(b.scroll));
                } else if row >= b.scroll + height.max(1) {
                    b.set_cursor(b.start(b.scroll + height.saturating_sub(1)));
                }
            }
            _ => {}
        }
    }
    pub fn dirty(&self) -> bool {
        self.buffers.iter().any(Buffer::dirty)
    }
    pub fn tick(&mut self) -> bool {
        if self.visible
            && self.focused
            && self.autocomplete
            && self.vim.mode == vim::Mode::Insert
            && self.lsp_pending.is_none()
            && self
                .auto_due
                .as_ref()
                .is_some_and(|(due, _)| *due <= std::time::Instant::now())
        {
            let (_, command) = self.auto_due.take().unwrap();
            self.request_lsp(&command, true);
        }
        if self.visible
            && self.focused
            && self.vim.mode == vim::Mode::Insert
            && self.lsp_pending.is_none()
            && self
                .signature_due
                .is_some_and(|due| due <= std::time::Instant::now())
        {
            self.signature_due = None;
            self.request_lsp("signature", true);
        }
        self.resolve_completion();
        false
    }
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Ok(reply) = self.lsp_worker.output.try_recv() {
            if let Some((path, version, cursor, automatic)) = self.lsp_pending.take() {
                let cancelled = self
                    .lsp_cancel
                    .swap(false, std::sync::atomic::Ordering::Relaxed);
                let valid = !cancelled
                    && self.visible
                    && self.buffers.get(self.current).is_some_and(|b| {
                        b.path == path && b.version == version && b.cursor() == cursor
                    });
                if valid && (!automatic || (self.focused && self.vim.mode == vim::Mode::Insert)) {
                    match reply {
                        Ok(Reply::Lsp(_, _, text)) => {
                            let resolve_valid = self
                                .lsp_command_pending
                                .strip_prefix("resolve:")
                                .is_none_or(|item| {
                                    serde_json::from_str::<serde_json::Value>(item)
                                        .ok()
                                        .is_some_and(|expected| {
                                            self.popup
                                                .as_ref()
                                                .is_some_and(|(_, items, i)| items[*i] == expected)
                                        })
                                });
                            if resolve_valid {
                                self.receive_lsp(&text);
                            }
                        }
                        Err(e) => {
                            if self.cmp.accepting
                                && self.lsp_command_pending.starts_with("resolve:")
                                && !self.cmp.resolve_due
                                && self
                                    .lsp_command_pending
                                    .strip_prefix("resolve:")
                                    .and_then(|item| {
                                        serde_json::from_str::<serde_json::Value>(item).ok()
                                    })
                                    .is_some_and(|expected| {
                                        self.popup
                                            .as_ref()
                                            .is_some_and(|(_, items, i)| items[*i] == expected)
                                    })
                            {
                                self.cmp.accepting = false;
                                self.accept_popup();
                            }
                            if !automatic {
                                self.message = format!("{e:#}");
                            }
                        }
                        _ => {}
                    }
                    changed = true;
                }
            }
        }
        let Ok(reply) = self.worker.output.try_recv() else {
            return changed;
        };
        self.busy = false;
        match reply {
            Ok(Reply::Opened(path, text)) => {
                self.buffers.push(Buffer::from_text(path, text));
                self.current = self.buffers.len() - 1;
                self.message.clear();
            }
            Ok(Reply::Saved(path, version)) => {
                if let Some(b) = self.buffers.iter_mut().find(|b| b.path == path) {
                    b.saved(version);
                    let byte = b.state.doc.char_to_byte(b.cursor());
                    self.saved_location = Some((path.clone(), byte));
                }
                self.message = "Saved".into();
            }
            Ok(Reply::Lsp(path, version, text)) => {
                if self
                    .buffers
                    .get(self.current)
                    .is_some_and(|b| b.path == path && b.version == version)
                {
                    self.receive_lsp(&text);
                } else {
                    self.message = "Ignored stale LSP reply".into();
                }
            }
            Ok(Reply::Formatted(path, version, text)) => {
                if let Some(b) = self.buffers.iter_mut().find(|b| b.path == path) {
                    if b.version == version {
                        b.edit(0, b.state.doc.len_chars(), &text);
                        b.commit();
                        self.message = "Formatted".into();
                        if self
                            .save_after_format
                            .as_ref()
                            .is_some_and(|(p, v)| *p == path && *v == version)
                        {
                            let task = Task::Save(b.path.clone(), b.version, b.state.doc.clone());
                            self.save_after_format = None;
                            self.submit(task);
                        }
                    } else {
                        self.save_after_format = None;
                        self.message = "Source changed during formatting; save again".into();
                    }
                }
            }
            Err(e) => {
                self.save_after_format = None;
                self.message = format!("{e:#} · :set noformatonsave to save without formatting");
            }
        };
        if let Some(path) = self.next_open.take() {
            self.open(&path);
        }
        if let Some((path, line, column)) = self.pending_jump.clone() {
            if let Some(b) = self
                .buffers
                .get_mut(self.current)
                .filter(|b| b.path == path)
            {
                let pos = lsp::char_position(&b.state.doc, line, column);
                b.set_cursor(pos);
                self.pending_jump = None;
            }
        }
        true
    }
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if self.window_key(key) {
            return false;
        }
        // Navigation/documentation keys must not cancel a completion fetch or
        // change the document version. Resolve replies carry their own item identity.
        let had_completion = self.completion_visible();
        if self.completion_key(key) {
            self.auto_due = None;
            if had_completion && !self.completion_visible() && self.lsp_pending.is_some() {
                self.lsp_cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            return false;
        }
        self.auto_due = None;
        self.signature_due = None;
        if self.lsp_pending.is_some() {
            self.lsp_cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if self.completion_visible() {
            self.popup = None;
            self.popup_text.clear();
        }
        if self.popup.is_some() {
            let completion = self.popup.as_ref().is_some_and(|(method, _, _)| {
                method.ends_with("completion")
                    || method.ends_with("references")
                    || method.ends_with("definition")
            });
            match key.code {
                KeyCode::Esc => {
                    self.popup = None;
                    self.popup_text.clear();
                    return false;
                }
                KeyCode::Down | KeyCode::Tab if completion => {
                    self.move_popup(1);
                    return false;
                }
                KeyCode::Up | KeyCode::BackTab if completion => {
                    self.move_popup(-1);
                    return false;
                }
                KeyCode::Enter if completion => {
                    self.accept_popup();
                    return false;
                }
                _ => {
                    self.popup = None;
                    self.popup_text.clear();
                }
            }
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
            self.close_buffer(false);
            return false;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
            && !self.buffers.is_empty()
        {
            self.clear_tools();
            self.current = (self.current
                + if key.code == KeyCode::PageDown {
                    1
                } else {
                    self.buffers.len() - 1
                })
                % self.buffers.len();
            self.vim.reset_mode();
            return false;
        }
        let action = match key.code {
            KeyCode::F(12) => Some("definition"),
            KeyCode::F(1) => Some("hover"),
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => Some("complete"),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some("signature")
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::ALT) => Some("format"),
            _ => None,
        };
        if let Some(action) = action {
            if action == "format" {
                self.format();
            } else {
                self.lsp_command(action);
            }
            return false;
        }
        if self.window_prefix {
            self.window_prefix = false;
            match key.code {
                KeyCode::Char('>' | '+') => self.ratio = (self.ratio + 5).min(80),
                KeyCode::Char('<' | '-') => self.ratio = self.ratio.saturating_sub(5).max(20),
                KeyCode::Char('=') => self.ratio = 50,
                KeyCode::Char('w' | 'l') => self.focused = false,
                _ => {}
            }
            return false;
        }
        if key.code == KeyCode::Char('w')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && self.vim.mode == vim::Mode::Normal
        {
            self.window_prefix = true;
            self.message = "Ctrl+w: > wider · < narrower · = equal · w PDF".into();
            return false;
        }
        if key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            self.ratio = if key.code == KeyCode::Right {
                (self.ratio + 5).min(80)
            } else {
                self.ratio.saturating_sub(5).max(20)
            };
            return false;
        }
        if self.vim.mode == vim::Mode::Insert
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('n' | 'p'))
        {
            self.complete_word(key.code == KeyCode::Char('p'));
            return false;
        }
        self.completion = None;
        self.message.clear();
        if self.buffers.is_empty() {
            return false;
        }
        self.vim.viewport_height = self.viewport_height;
        let before = self.buffers[self.current].version;
        if self.vim.mode == vim::Mode::Insert
            && self.buffers[self.current].snippet.is_some()
            && matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace)
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            let b = &mut self.buffers[self.current];
            let r = b.state.selection.primary();
            let (a, z) = (r.anchor.min(r.head), r.anchor.max(r.head));
            if a != z {
                b.edit(a, z, "");
                b.set_cursor(a);
            }
        }
        let previous_cursor = self.buffers[self.current].cursor();
        let pair_handled = self.insert_pair(key);
        let command = if pair_handled {
            None
        } else {
            self.vim.key(&mut self.buffers[self.current], key)
        };
        if self.buffers[self.current].version != before
            || self.buffers[self.current].cursor() != previous_cursor
            || self.vim.mode != vim::Mode::Insert
        {
            self.schedule_signature();
        }
        if self.autocomplete
            && self.vim.mode == vim::Mode::Insert
            && self.buffers[self.current].version != before
        {
            let trigger = match key.code {
                KeyCode::Char('(' | ',') => None,
                KeyCode::Char(c) if c.is_alphanumeric() || matches!(c, '_' | '.' | '#') => {
                    Some("complete")
                }
                KeyCode::Backspace => Some("complete"),
                _ => None,
            };
            if let Some(command) = trigger {
                self.auto_due = Some((
                    std::time::Instant::now()
                        + std::time::Duration::from_millis(self.completion_config.debounce_ms),
                    command.into(),
                ));
            }
        }
        if let Some(command) = command {
            match command.as_str() {
                "w" | "write" => self.save(),
                "set autocomplete" => self.autocomplete = true,
                "set noautocomplete" => self.autocomplete = false,
                "bd" | "bdelete" | "tabclose" => self.close_buffer(false),
                "bd!" | "bdelete!" | "tabclose!" => self.close_buffer(true),
                "ls" | "buffers" => {
                    self.message = self
                        .buffers
                        .iter()
                        .enumerate()
                        .map(|(i, b)| {
                            format!(
                                "{} {}{}",
                                i + 1,
                                b.path.display(),
                                if b.dirty() { " [+]" } else { "" }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" · ")
                }
                _ if command.starts_with("b ") => {
                    if let Ok(index) = command[2..].parse::<usize>() {
                        if index > 0 && index <= self.buffers.len() {
                            self.current = index - 1;
                            self.vim.reset_mode();
                        }
                    }
                }
                "set formatonsave" => {
                    self.format_on_save = true;
                    self.message = "Format on save enabled".into();
                }
                "set noformatonsave" => {
                    self.format_on_save = false;
                    self.message = "Format on save disabled".into();
                }
                "q" => {
                    if self.dirty() {
                        self.message = "Unsaved buffers (:q! to hide)".into();
                    } else {
                        self.visible = false;
                        self.focused = false;
                    }
                }
                "q!" => {
                    self.visible = false;
                    self.focused = false;
                }
                "bn" | "tabnext" => {
                    self.clear_tools();
                    self.current = (self.current + 1) % self.buffers.len();
                    self.vim.reset_mode();
                }
                "bp" | "tabprevious" => {
                    self.clear_tools();
                    self.current = (self.current + self.buffers.len() - 1) % self.buffers.len();
                    self.vim.reset_mode();
                }
                "sp" | "split" => self.horizontal = true,
                "vsp" | "vsplit" => self.horizontal = false,
                "format" => self.format(),
                "hover" | "complete" | "definition" | "references" | "signature" | "symbols"
                | "diagnostics" | "lsp-restart" => self.lsp_command(&command),
                _ if command.starts_with("e ") || command.starts_with("tabnew ") => {
                    let path = PathBuf::from(command.split_once(' ').unwrap().1);
                    match path.canonicalize() {
                        Ok(p) => self.open(&p),
                        Err(e) => self.message = e.to_string(),
                    }
                }
                _ if command.starts_with("%s/") => {
                    let parts: Vec<_> = command[3..].split('/').collect();
                    if parts.len() >= 2 {
                        match regex::Regex::new(parts[0]) {
                            Ok(re) => {
                                let b = &mut self.buffers[self.current];
                                let mut edits = vec![];
                                for row in 0..b.state.doc.len_lines() {
                                    let line = b.state.doc.line(row).to_string();
                                    let replaced = if parts.get(2).is_some_and(|v| v.contains('g'))
                                    {
                                        re.replace_all(&line, parts[1])
                                    } else {
                                        re.replace(&line, parts[1])
                                    };
                                    if replaced != line {
                                        edits.push((
                                            b.start(row),
                                            b.start(row) + line.chars().count(),
                                            Some(replaced.as_ref().into()),
                                        ));
                                    }
                                }
                                if !edits.is_empty() {
                                    b.edit_many(edits);
                                    b.commit();
                                }
                            }
                            Err(e) => self.message = e.to_string(),
                        }
                    }
                }
                _ => self.message = format!("Unsupported command: {command}"),
            }
        }
        false
    }
    fn complete_word(&mut self, reverse: bool) {
        let Some(b) = self.buffers.get_mut(self.current) else {
            return;
        };
        if self.completion.is_none() {
            let end = b.cursor();
            let mut start = end;
            while start > b.start(b.line())
                && (b.state.doc.char(start - 1).is_alphanumeric()
                    || b.state.doc.char(start - 1) == '_')
            {
                start -= 1;
            }
            let prefix = b.state.doc.slice(start..end).to_string();
            if prefix.is_empty() {
                return;
            }
            // Bounded neighborhood: completion never scans a 100 MiB buffer.
            let first = b.line().saturating_sub(500);
            let last = (b.line() + 500).min(b.state.doc.len_lines());
            let mut words = std::collections::BTreeSet::new();
            for line in first..last {
                let text: String = b.state.doc.line(line).chars().take(4096).collect();
                for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
                    if word.starts_with(&prefix) && word != prefix {
                        words.insert(word.to_owned());
                    }
                    if words.len() >= 100 {
                        break;
                    }
                }
                if words.len() >= 100 {
                    break;
                }
            }
            if words.is_empty() {
                self.message = "No matching words".into();
                return;
            }
            self.completion = Some((
                start,
                end,
                words.into_iter().collect(),
                if reverse { 0 } else { usize::MAX },
            ));
        }
        let (start, end, words, index) = self.completion.as_mut().unwrap();
        *index = if *index == usize::MAX {
            0
        } else if reverse {
            (*index + words.len() - 1) % words.len()
        } else {
            (*index + 1) % words.len()
        };
        let word = &words[*index];
        b.edit(*start, *end, word);
        *end = *start + word.chars().count();
        b.set_cursor(*end);
        self.message = format!(
            "{}  [{}/{}] · Ctrl+n / Ctrl+p",
            word,
            *index + 1,
            words.len()
        );
    }
    fn submit(&mut self, task: Task) {
        if self.busy {
            self.message = "Background task busy; retry".into();
            return;
        }
        if self.worker.input.as_ref().unwrap().try_send(task).is_ok() {
            self.busy = true;
        }
    }
    pub fn save(&mut self) {
        if self.busy {
            self.message = "Background task busy; save again".into();
            return;
        }
        if self.format_on_save {
            if let Some(b) = self.buffers.get(self.current) {
                let ext = b.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(
                    ext,
                    "typ"
                        | "rs"
                        | "py"
                        | "js"
                        | "ts"
                        | "tsx"
                        | "jsx"
                        | "json"
                        | "c"
                        | "h"
                        | "cpp"
                        | "hpp"
                ) {
                    self.save_after_format = Some((b.path.clone(), b.version));
                    self.format();
                    return;
                }
            }
        }
        if let Some(b) = self.buffers.get_mut(self.current) {
            b.commit();
            let task = Task::Save(b.path.clone(), b.version, b.state.doc.clone());
            self.submit(task);
        }
    }
    fn close_buffer(&mut self, force: bool) {
        if self.buffers.is_empty() {
            return;
        }
        if self.buffers.get(self.current).is_some_and(|b| b.dirty()) && !force {
            self.message = "Unsaved buffer · :bd! to discard".into();
            return;
        }
        self.clear_tools();
        self.buffers.remove(self.current);
        self.current = self.current.min(self.buffers.len().saturating_sub(1));
        self.vim.reset_mode();
        if self.buffers.is_empty() {
            self.visible = false;
            self.focused = false;
        }
    }
    fn lsp_command(&mut self, command: &str) {
        self.request_lsp(command, false);
    }
    fn request_lsp(&mut self, command: &str, automatic: bool) {
        if self.lsp_pending.is_some() {
            if !automatic {
                self.message = "LSP busy; retry".into();
            }
            return;
        }
        if command == "complete" {
            if let Some(b) = self.buffers.get(self.current) {
                let items = completion_sources::local(b, &self.completion_config);
                if !items.is_empty() {
                    self.receive_lsp(
                        &serde_json::json!([{"method":"textDocument/completion","result":items}])
                            .to_string(),
                    );
                }
            }
            if !self
                .completion_config
                .sources
                .iter()
                .any(|s| s == "lsp" || s == "path")
            {
                return;
            }
        }
        if let Some(b) = self.buffers.get(self.current) {
            self.lsp_cancel
                .store(false, std::sync::atomic::Ordering::Relaxed);
            let task = Task::Lsp(
                b.path.clone(),
                b.version,
                b.state.doc.clone(),
                b.cursor(),
                if command == "complete" {
                    format!("complete:{}", self.completion_config.sources.join(","))
                } else {
                    command.into()
                },
            );
            if self
                .lsp_worker
                .input
                .as_ref()
                .unwrap()
                .try_send(task)
                .is_ok()
            {
                self.lsp_pending = Some((b.path.clone(), b.version, b.cursor(), automatic));
                self.lsp_command_pending = command.into();
            }
        }
    }
    fn format(&mut self) {
        if let Some(b) = self.buffers.get(self.current) {
            let task = Task::Format(b.path.clone(), b.version, b.state.doc.clone());
            self.submit(task);
        }
    }
}
