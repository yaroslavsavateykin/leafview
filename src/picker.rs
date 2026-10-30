//! Independent file choice for source and preview panes.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Clear, Paragraph},
    Frame,
};
use std::path::{Path, PathBuf};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Editor,
    Preview,
}
pub struct Picker {
    pub target: Target,
    root: PathBuf,
    entries: Vec<PathBuf>,
    pub selected: usize,
    pub query: String,
    pub error: String,
}
impl Picker {
    pub fn new(root: &Path, target: Target) -> Self {
        let mut picker = Self {
            target,
            root: root.to_owned(),
            entries: vec![],
            selected: 0,
            query: String::new(),
            error: String::new(),
        };
        picker.refresh();
        picker
    }
    fn refresh(&mut self) {
        self.entries.clear();
        self.error.clear();
        match std::fs::read_dir(&self.root) {
            Ok(entries) => {
                self.entries = entries
                    .take(10000)
                    .flatten()
                    .filter_map(|entry| {
                        let kind = entry.file_type().ok()?;
                        let path = entry.path();
                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        let allowed = if self.target == Target::Preview {
                            matches!(ext, "pdf" | "typ")
                        } else {
                            matches!(
                                ext,
                                "typ"
                                    | "rs"
                                    | "py"
                                    | "js"
                                    | "ts"
                                    | "tsx"
                                    | "jsx"
                                    | "json"
                                    | "toml"
                                    | "yaml"
                                    | "yml"
                                    | "md"
                                    | "txt"
                                    | "c"
                                    | "h"
                                    | "cpp"
                                    | "hpp"
                                    | "css"
                                    | "html"
                                    | "sh"
                                    | "tex"
                                    | "bib"
                            )
                        };
                        ((kind.is_dir() || (kind.is_file() && allowed))
                            && path
                                .file_name()?
                                .to_string_lossy()
                                .to_lowercase()
                                .contains(&self.query.to_lowercase()))
                        .then_some(path)
                    })
                    .collect();
                self.entries.sort_by_key(|p| {
                    (
                        !p.is_dir(),
                        p.file_name().unwrap_or_default().to_ascii_lowercase(),
                    )
                });
            }
            Err(e) => self.error = e.to_string(),
        }
        self.selected = 0;
    }
    pub fn key(&mut self, key: KeyEvent) -> Option<PathBuf> {
        match key.code {
            KeyCode::Down | KeyCode::Tab => {
                self.selected = (self.selected + 1).min(self.entries.len().saturating_sub(1))
            }
            KeyCode::Up | KeyCode::BackTab => self.selected = self.selected.saturating_sub(1),
            KeyCode::Enter => {
                let path = self.entries.get(self.selected)?.clone();
                if path.is_dir() {
                    self.root = path;
                    self.query.clear();
                    self.refresh();
                } else {
                    return Some(path);
                }
            }
            KeyCode::Left if self.query.is_empty() => {
                if let Some(parent) = self.root.parent() {
                    self.root = parent.to_owned();
                    self.refresh();
                }
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.refresh();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.query.push(c);
                self.refresh();
            }
            _ => {}
        }
        None
    }
    pub fn area(area: Rect) -> Rect {
        let width = area.width.saturating_sub(4).min(90);
        let height = area.height.saturating_sub(4).min(25);
        Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        )
    }
    pub fn mouse(&mut self, area: Rect, x: u16, y: u16, click: bool) -> Option<PathBuf> {
        let area = Self::area(area);
        if x <= area.x
            || x >= area.right().saturating_sub(1)
            || y < area.y + 3
            || y >= area.bottom().saturating_sub(1)
        {
            return None;
        }
        let offset = self
            .selected
            .saturating_sub(usize::from(area.height.saturating_sub(5)));
        let index = offset + usize::from(y - area.y - 3);
        if index < self.entries.len() {
            self.selected = index;
            if click {
                return self.key(KeyEvent::from(KeyCode::Enter));
            }
        }
        None
    }
    pub fn render(&self, f: &mut Frame<'_>) {
        let area = Self::area(f.area());
        let offset = self
            .selected
            .saturating_sub(usize::from(area.height.saturating_sub(5)));
        let mut lines = vec![
            ratatui::text::Line::from(crate::links::sanitize(&self.root.display().to_string())),
            ratatui::text::Line::from(format!("Search: {}", self.query)),
        ];
        if !self.error.is_empty() {
            lines[1] = ratatui::text::Line::from(crate::links::sanitize(&self.error));
        }
        for (i, path) in self
            .entries
            .iter()
            .enumerate()
            .skip(offset)
            .take(usize::from(area.height.saturating_sub(4)))
        {
            lines.push(ratatui::text::Line::styled(
                format!(
                    "{} {}{}",
                    if i == self.selected { "›" } else { " " },
                    crate::links::sanitize(&path.file_name().unwrap_or_default().to_string_lossy()),
                    if path.is_dir() { "/" } else { "" }
                ),
                Style::default().bg(if i == self.selected {
                    Color::Rgb(42, 67, 80)
                } else {
                    Color::Rgb(25, 29, 36)
                }),
            ));
        }
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(lines)
                .style(
                    Style::default()
                        .fg(Color::Rgb(210, 225, 230))
                        .bg(Color::Rgb(25, 29, 36)),
                )
                .block(crate::files::panel_block(
                    if self.target == Target::Editor {
                        "Open source · Enter · ← parent · Esc"
                    } else {
                        "Open preview · Enter · ← parent · Esc"
                    },
                    true,
                )),
            area,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picker_renders_and_hits_rows() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.typ"), "").unwrap();
        let mut picker = Picker::new(dir.path(), Target::Editor);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| picker.render(f)).unwrap();
        let buffer = terminal.backend().buffer();
        let snapshot = (0..30)
            .map(|y| {
                (0..100)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(snapshot.contains("Open source"));
        assert!(snapshot.contains("main.typ"));
        std::fs::write("/tmp/leafview-picker-render.txt", snapshot).unwrap();
        let area = Picker::area(Rect::new(0, 0, 100, 30));
        assert_eq!(
            picker.mouse(Rect::new(0, 0, 100, 30), area.x + 2, area.y + 3, true),
            Some(dir.path().join("main.typ"))
        );
    }
    #[test]
    fn targets_filter_and_search_independently() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a.typ", "b.pdf", "c.rs"] {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        let mut source = Picker::new(dir.path(), Target::Editor);
        assert_eq!(source.entries.len(), 2);
        source.key(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!(
            source.key(KeyEvent::from(KeyCode::Enter)),
            Some(dir.path().join("c.rs"))
        );
        let preview = Picker::new(dir.path(), Target::Preview);
        assert_eq!(preview.entries.len(), 2);
        assert!(!preview
            .entries
            .iter()
            .any(|p| p.extension().unwrap() == "rs"));
    }
}
