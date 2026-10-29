pub mod buffer;
pub mod focus;
mod lsp;
mod render;
mod syntax;
mod tasks;
#[cfg(test)]
mod tests;
pub mod vim;
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
}
impl Editor {
    pub fn new() -> Self {
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
            message: String::new(),
            next_open: None,
        }
    }
    pub fn open(&mut self, path: &Path) {
        self.visible = true;
        self.focused = true;
        if let Some(i) = self.buffers.iter().position(|b| b.path == path) {
            self.current = i;
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
        let Some(b) = self.buffers.get_mut(self.current) else {
            return;
        };
        let height = area.height.saturating_sub(3) as usize;
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
                let row = (b.scroll + usize::from(event.row.saturating_sub(area.y + 1)))
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
                b.set_cursor(pos);
            }
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
    pub fn poll(&mut self) -> bool {
        let Ok(reply) = self.worker.output.try_recv() else {
            return false;
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
                }
                self.message = "Saved".into();
            }
            Ok(Reply::Lsp(path, version, text)) => {
                if self
                    .buffers
                    .get(self.current)
                    .is_some_and(|b| b.path == path && b.version == version)
                {
                    self.message = text;
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
        true
    }
    pub fn key(&mut self, key: KeyEvent) -> bool {
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
        let command = self.vim.key(&mut self.buffers[self.current], key);
        if let Some(command) = command {
            match command.as_str() {
                "w" | "write" => self.save(),
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
                "bn" | "tabnext" => self.current = (self.current + 1) % self.buffers.len(),
                "bp" | "tabprevious" => {
                    self.current = (self.current + self.buffers.len() - 1) % self.buffers.len()
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
    fn lsp_command(&mut self, command: &str) {
        if let Some(b) = self.buffers.get(self.current) {
            let task = Task::Lsp(
                b.path.clone(),
                b.version,
                b.state.doc.clone(),
                b.cursor(),
                command.into(),
            );
            self.submit(task);
        }
    }
    fn format(&mut self) {
        if let Some(b) = self.buffers.get(self.current) {
            let task = Task::Format(b.path.clone(), b.version, b.state.doc.clone());
            self.submit(task);
        }
    }
}
