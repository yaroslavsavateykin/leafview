//! PDF-only, lazy directory browser and bounded persistent recent-file history.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Clone)]
struct Entry {
    path: PathBuf,
    depth: usize,
    directory: bool,
    recent: bool,
}

pub struct Files {
    pub visible: bool,
    pub focused: bool,
    root: PathBuf,
    expanded: BTreeSet<PathBuf>,
    entries: Vec<Entry>,
    selected: usize,
    state: ListState,
    recent: Vec<PathBuf>,
    history_path: Option<PathBuf>,
    pub notice: Option<String>,
}
impl Files {
    pub fn new(input: &Path, enabled: bool) -> Self {
        let root = if input.is_dir() {
            input
        } else {
            input.parent().unwrap_or(input)
        }
        .to_owned();
        let history_path = std::env::var_os("XDG_STATE_HOME")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
            .map(|p| p.join("leafview/recent.json"));
        let recent = history_path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .filter(|b| b.len() <= 256 * 1024)
            .and_then(|b| serde_json::from_slice::<Vec<PathBuf>>(&b).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.is_absolute() && is_pdf(p))
            .take(20)
            .collect();
        let mut files = Self {
            visible: enabled,
            focused: enabled,
            root,
            expanded: BTreeSet::new(),
            entries: Vec::new(),
            selected: 0,
            state: ListState::default(),
            recent,
            history_path,
            notice: None,
        };
        files.refresh();
        files
    }
    pub fn width(&self, width: u16) -> u16 {
        if self.visible && width >= 48 {
            (width / 3).clamp(22, 38)
        } else {
            0
        }
    }
    pub fn record(&mut self, path: &Path) {
        if !path.is_file() || !is_pdf(path) {
            return;
        }
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_owned());
        self.recent.truncate(20);
        if let Some(target) = &self.history_path {
            let result = (|| -> anyhow::Result<()> {
                use std::io::Write;
                let parent = target.parent().unwrap();
                std::fs::create_dir_all(parent)?;
                let mut file = tempfile::NamedTempFile::new_in(parent)?;
                serde_json::to_writer(&mut file, &self.recent)?;
                file.flush()?;
                file.persist(target).map_err(|e| e.error)?;
                Ok(())
            })();
            if let Err(error) = result {
                self.notice = Some(format!("History: {error}"));
            }
        }
        self.refresh();
    }
    fn refresh(&mut self) {
        let selection = self
            .entries
            .get(self.selected)
            .map(|e| (e.path.clone(), e.recent));
        self.entries.clear();
        for path in &self.recent {
            self.entries.push(Entry {
                path: path.clone(),
                depth: 0,
                directory: false,
                recent: true,
            });
        }
        self.entries.push(Entry {
            path: self.root.clone(),
            depth: 0,
            directory: true,
            recent: false,
        });
        if let Err(error) = append_directory(&self.root, 1, &self.expanded, &mut self.entries) {
            self.notice = Some(format!("Directory: {error}"));
        }
        self.selected = selection
            .and_then(|s| {
                self.entries
                    .iter()
                    .position(|e| (e.path.clone(), e.recent) == s)
            })
            .unwrap_or(self.selected)
            .min(self.entries.len().saturating_sub(1));
    }
    pub fn key(&mut self, key: KeyEvent) -> Option<PathBuf> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1).min(self.entries.len().saturating_sub(1))
            }
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Home | KeyCode::Char('g') => self.selected = 0,
            KeyCode::End | KeyCode::Char('G') => {
                self.selected = self.entries.len().saturating_sub(1)
            }
            KeyCode::Char('r') => {
                self.notice = None;
                self.refresh();
            }
            KeyCode::Backspace => {
                if let Some(parent) = self.root.parent() {
                    self.root = parent.to_owned();
                    self.expanded.clear();
                    self.refresh();
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(entry) = self.entries.get(self.selected).cloned() {
                    if !self.expanded.remove(&entry.path) {
                        if let Some(parent) = entry.path.parent() {
                            if let Some(index) = self
                                .entries
                                .iter()
                                .position(|e| !e.recent && e.path == parent)
                            {
                                self.selected = index;
                            }
                        }
                    }
                    self.refresh();
                }
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                if let Some(entry) = self.entries.get(self.selected).cloned() {
                    if entry.directory {
                        if entry.path == self.root && key.code == KeyCode::Enter {
                            self.refresh();
                        } else if self.expanded.contains(&entry.path) && key.code == KeyCode::Enter
                        {
                            self.expanded.remove(&entry.path);
                            self.refresh();
                        } else {
                            self.expanded.insert(entry.path);
                            self.refresh();
                        }
                    } else if key.code == KeyCode::Enter {
                        return Some(entry.path);
                    }
                }
            }
            _ => {}
        }
        None
    }
    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect, current: &Path) {
        let items: Vec<_> = self
            .entries
            .iter()
            .map(|entry| {
                let marker = if entry.recent {
                    "◷"
                } else if entry.directory {
                    if entry.path == self.root || self.expanded.contains(&entry.path) {
                        "▾"
                    } else {
                        "▸"
                    }
                } else {
                    "·"
                };
                let name = entry
                    .path
                    .file_name()
                    .unwrap_or(entry.path.as_os_str())
                    .to_string_lossy();
                let label = if entry.recent {
                    format!(
                        "{marker} {name} — {}",
                        entry.path.parent().unwrap_or(Path::new("")).display()
                    )
                } else {
                    format!("{}{marker} {name}", "  ".repeat(entry.depth.min(24)))
                };
                ListItem::new(crate::links::sanitize(&label)).style(Style::default().fg(
                    if entry.path == current {
                        Color::LightCyan
                    } else if entry.directory {
                        Color::Rgb(180, 192, 211)
                    } else {
                        Color::Rgb(214, 217, 224)
                    },
                ))
            })
            .collect();
        self.state.select(Some(self.selected));
        let title = if self.focused {
            " PDF files • focused "
        } else {
            " PDF files "
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_bottom(" Tab focus · F2 hide ")
                    .border_style(Style::default().fg(if self.focused {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    })),
            )
            .style(Style::default().bg(Color::Rgb(25, 29, 36)))
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(42, 57, 72))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");
        f.render_stateful_widget(list, area, &mut self.state);
    }
}
fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("pdf"))
}
fn append_directory(
    root: &Path,
    depth: usize,
    expanded: &BTreeSet<PathBuf>,
    entries: &mut Vec<Entry>,
) -> std::io::Result<()> {
    if depth > 24 || entries.len() >= 5000 {
        return Ok(());
    }
    let mut children = Vec::new();
    for item in std::fs::read_dir(root)?.take(10000).flatten() {
        let Ok(kind) = item.file_type() else {
            continue;
        };
        // Never follow directory symlinks (cycles / unexpected trees).
        if kind.is_dir() || (kind.is_file() && is_pdf(&item.path())) {
            children.push((!kind.is_dir(), item.file_name(), item.path()));
        }
    }
    children.sort_by_key(|a| (a.0, a.1.to_ascii_lowercase()));
    for (file, _, path) in children {
        if entries.len() >= 5000 {
            break;
        }
        entries.push(Entry {
            path: path.clone(),
            depth,
            directory: !file,
            recent: false,
        });
        if !file && expanded.contains(&path) {
            append_directory(&path, depth + 1, expanded, entries)?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tree_is_pdf_only_sorted_and_lazy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("chapter")).unwrap();
        for name in ["b.PDF", "a.pdf", "ignore.tex", "chapter/nested.pdf"] {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        let mut entries = Vec::new();
        append_directory(dir.path(), 0, &BTreeSet::new(), &mut entries).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].directory);
        assert!(entries[1].path.ends_with("a.pdf"));
        let expanded = BTreeSet::from([dir.path().join("chapter")]);
        entries.clear();
        append_directory(dir.path(), 0, &expanded, &mut entries).unwrap();
        assert_eq!(entries.len(), 4);
        assert!(entries[1].path.ends_with("nested.pdf"));
    }
    #[test]
    fn panel_renders_and_history_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("Research notes.pdf");
        std::fs::write(&pdf, "pdf").unwrap();
        let history = dir.path().join("state/recent.json");
        let mut files = Files::new(dir.path(), true);
        files.recent.clear();
        files.history_path = Some(history.clone());
        files.record(&pdf);
        files.record(&pdf);
        let saved: Vec<PathBuf> = serde_json::from_slice(&std::fs::read(history).unwrap()).unwrap();
        assert_eq!(saved, vec![pdf.clone()]);
        let backend = ratatui::backend::TestBackend::new(38, 14);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| files.render(f, f.area(), &pdf)).unwrap();
        let buffer = terminal.backend().buffer();
        let rendered = (0..14)
            .map(|y| (0..38).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        if let Ok(path) = std::env::var("LEAFVIEW_PANEL_SNAPSHOT") {
            std::fs::write(path, &rendered).unwrap();
        }
        assert!(rendered.contains("PDF files"));
        assert!(rendered.contains("Research notes.pdf"));
        assert!(rendered.contains("Tab focus"));
    }
    #[test]
    fn panel_fits_small_terminals() {
        let dir = tempfile::tempdir().unwrap();
        let files = Files::new(dir.path(), true);
        assert_eq!(files.width(40), 0);
        assert_eq!(files.width(90), 30);
    }
}
