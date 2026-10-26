//! PDF-only, lazy directory browser and bounded persistent recent-file history.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::{
    collections::{BTreeMap, BTreeSet},
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
    pub recent_focused: bool,
    sidebar_width: Option<u16>,
    tree_height: Option<u16>,
    recent_selected: usize,
    recent_state: ListState,
    root: PathBuf,
    expanded: BTreeSet<PathBuf>,
    entries: Vec<Entry>,
    selected: usize,
    state: ListState,
    recent: Vec<PathBuf>,
    history_path: Option<PathBuf>,
    pub notice: Option<String>,
    pdf_folders: BTreeMap<PathBuf, Option<bool>>,
    folder_scanner: crate::workers::Worker<Vec<PathBuf>, Vec<(PathBuf, Option<bool>)>>,
    scan_pending: bool,
    scan_due: bool,
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
            .filter(|p| p.is_absolute() && is_document(p))
            .take(20)
            .collect();
        let mut files = Self {
            visible: enabled,
            focused: enabled,
            recent_focused: false,
            sidebar_width: None,
            tree_height: None,
            recent_selected: 0,
            recent_state: ListState::default(),
            root,
            expanded: BTreeSet::new(),
            entries: Vec::new(),
            selected: 0,
            state: ListState::default(),
            recent,
            history_path,
            notice: None,
            pdf_folders: BTreeMap::new(),
            folder_scanner: crate::workers::Worker::new(|paths: Vec<PathBuf>, stop| {
                paths
                    .into_iter()
                    .map(|path| {
                        let mut budget = 20000;
                        let found = contains_pdf(&path, 0, &mut budget, stop);
                        (path, found)
                    })
                    .collect()
            }),
            scan_pending: false,
            scan_due: true,
        };
        files.refresh();
        files
    }
    pub fn width(&self, width: u16) -> u16 {
        if self.visible && width >= 48 {
            self.sidebar_width
                .unwrap_or((width / 3).clamp(22, 38))
                .clamp(16, width.saturating_sub(24))
        } else {
            0
        }
    }
    pub fn cycle_focus(&mut self, width: u16) {
        if self.width(width) == 0 {
            return;
        }
        if !self.focused {
            self.focused = true;
            self.recent_focused = false;
        } else if !self.recent_focused {
            self.recent_focused = true;
        } else {
            self.focused = false;
        }
    }
    pub fn focus_direction(&mut self, direction: char, width: u16) {
        if self.width(width) == 0 {
            return;
        }
        match direction {
            'h' => self.focused = true,
            'l' => self.focused = false,
            'j' if self.focused => self.recent_focused = true,
            'k' if self.focused => self.recent_focused = false,
            _ => {}
        }
    }
    pub fn resize(&mut self, direction: KeyCode, width: u16, height: u16) {
        let current = self.width(width);
        if current == 0 {
            return;
        }
        let tree_height = self.panels(Rect::new(0, 0, current, height))[0].height;
        match direction {
            KeyCode::Left => self.sidebar_width = Some(current.saturating_sub(1)),
            KeyCode::Right => self.sidebar_width = Some(current.saturating_add(1)),
            KeyCode::Up if self.focused => self.tree_height = Some(tree_height.saturating_sub(1)),
            KeyCode::Down if self.focused => self.tree_height = Some(tree_height.saturating_add(1)),
            _ => {}
        }
    }
    pub fn record(&mut self, path: &Path) {
        if !path.is_file() || !is_document(path) {
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
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Ok(results) = self.folder_scanner.output.try_recv() {
            self.scan_pending = false;
            self.pdf_folders.extend(results);
            changed = true;
        }
        if self.scan_due && !self.scan_pending {
            let paths: Vec<_> = self
                .entries
                .iter()
                .filter(|e| e.directory)
                .map(|e| e.path.clone())
                .collect();
            if self
                .folder_scanner
                .input
                .as_ref()
                .unwrap()
                .try_send(paths)
                .is_ok()
            {
                self.scan_pending = true;
                self.scan_due = false;
            }
        }
        changed
    }
    fn refresh(&mut self) {
        self.scan_due = true;
        let selection = self
            .entries
            .get(self.selected)
            .map(|e| (e.path.clone(), e.recent));
        self.entries.clear();
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
        if self.recent_focused {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    self.recent_selected =
                        (self.recent_selected + 1).min(self.recent.len().saturating_sub(1))
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.recent_selected = self.recent_selected.saturating_sub(1)
                }
                KeyCode::Home | KeyCode::Char('g') => self.recent_selected = 0,
                KeyCode::End | KeyCode::Char('G') => {
                    self.recent_selected = self.recent.len().saturating_sub(1)
                }
                KeyCode::Enter => return self.recent.get(self.recent_selected).cloned(),
                _ => {}
            }
            return None;
        }
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
    pub fn panels(&self, area: Rect) -> [Rect; 2] {
        let minimum = area.height.min(3);
        let tree_height = self
            .tree_height
            .unwrap_or((u32::from(area.height) * 65 / 100) as u16)
            .clamp(minimum, area.height.saturating_sub(minimum).max(minimum));
        let parts =
            Layout::vertical([Constraint::Length(tree_height), Constraint::Min(0)]).split(area);
        [parts[0], parts[1]]
    }
    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect, current: &Path) {
        let [tree_area, recent_area] = self.panels(area);
        let recent_items: Vec<_> = self
            .recent
            .iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy();
                ListItem::new(crate::links::sanitize(&format!("◷ {name}"))).style(
                    Style::default().fg(if path == current {
                        Color::LightCyan
                    } else {
                        Color::Rgb(214, 217, 224)
                    }),
                )
            })
            .collect();
        self.recent_state.select(if self.recent.is_empty() {
            None
        } else {
            Some(self.recent_selected.min(self.recent.len() - 1))
        });
        let recent_active = self.focused && self.recent_focused;
        let recent_list = List::new(recent_items)
            .block(panel_block("Recent", recent_active))
            .style(Style::default().bg(Color::Rgb(25, 29, 36)))
            .highlight_style(
                Style::default()
                    .bg(if recent_active {
                        Color::Rgb(42, 67, 80)
                    } else {
                        Color::Rgb(32, 39, 48)
                    })
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(if recent_active { "› " } else { "  " });
        f.render_stateful_widget(recent_list, recent_area, &mut self.recent_state);
        let current_is_pdf = is_document(current);
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
                let current_folder = entry.directory
                    && current_is_pdf
                    && current.parent() == Some(entry.path.as_path());
                let ancestor =
                    entry.directory && current_is_pdf && current.starts_with(&entry.path);
                let marker = if current_folder { "●" } else { marker };
                let label = if entry.recent {
                    format!(
                        "{marker} {name} — {}",
                        entry.path.parent().unwrap_or(Path::new("")).display()
                    )
                } else {
                    format!("{}{marker} {name}", "  ".repeat(entry.depth.min(24)))
                };
                ListItem::new(crate::links::sanitize(&label)).style(tree_entry_style(
                    entry,
                    current,
                    current_folder,
                    ancestor,
                    self.pdf_folders.get(&entry.path).copied().flatten(),
                ))
            })
            .collect();
        self.state.select(Some(self.selected));
        let active = self.focused && !self.recent_focused;
        let list = List::new(items)
            .block(panel_block("PDF tree", active))
            .style(Style::default().bg(Color::Rgb(25, 29, 36)))
            .highlight_style(
                Style::default()
                    .bg(if active {
                        Color::Rgb(42, 67, 80)
                    } else {
                        Color::Rgb(32, 39, 48)
                    })
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(if active { "› " } else { "  " });
        f.render_stateful_widget(list, tree_area, &mut self.state);
    }
}
fn tree_entry_style(
    entry: &Entry,
    current: &Path,
    current_folder: bool,
    ancestor: bool,
    contains: Option<bool>,
) -> Style {
    if current_folder {
        Style::default()
            .fg(Color::Rgb(110, 240, 225))
            .bg(Color::Rgb(24, 61, 65))
            .add_modifier(Modifier::BOLD)
    } else if ancestor {
        Style::default()
            .fg(Color::Rgb(100, 196, 202))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(if entry.path == current {
            Color::LightCyan
        } else if entry.directory {
            match contains {
                Some(true) => Color::Rgb(125, 201, 160),
                Some(false) => Color::Rgb(100, 108, 122),
                None => Color::Rgb(180, 192, 211),
            }
        } else {
            Color::Rgb(214, 217, 224)
        })
    }
}
pub fn panel_block(title: &str, active: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {}{} ", if active { "● " } else { "" }, title))
        .border_style(Style::default().fg(if active {
            Color::Cyan
        } else {
            Color::Rgb(65, 73, 85)
        }))
}
// Bounded recursive check off the UI thread. Unknown is distinct from empty.
fn contains_pdf(
    path: &Path,
    depth: usize,
    budget: &mut usize,
    stop: &std::sync::atomic::AtomicBool,
) -> Option<bool> {
    use std::sync::atomic::Ordering;
    if depth > 24 || *budget == 0 || stop.load(Ordering::Relaxed) {
        return None;
    }
    let entries = std::fs::read_dir(path).ok()?;
    let mut directories = Vec::new();
    let mut unknown = false;
    for item in entries {
        if *budget == 0 || stop.load(Ordering::Relaxed) {
            return None;
        }
        *budget -= 1;
        let Ok(item) = item else {
            unknown = true;
            continue;
        };
        let Ok(kind) = item.file_type() else {
            unknown = true;
            continue;
        };
        if kind.is_file() && is_pdf(&item.path()) {
            return Some(true);
        }
        if kind.is_dir() {
            directories.push(item.path());
        }
    }
    for path in directories {
        match contains_pdf(&path, depth + 1, budget, stop) {
            Some(true) => return Some(true),
            None => unknown = true,
            _ => {}
        }
    }
    if unknown {
        None
    } else {
        Some(false)
    }
}
fn is_document(path: &Path) -> bool {
    is_pdf(path)
        || path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("typ"))
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
        if kind.is_dir() || (kind.is_file() && is_document(&item.path())) {
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
    fn tree_is_pdf_typst_sorted_and_lazy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("chapter")).unwrap();
        for name in [
            "b.PDF",
            "a.pdf",
            "ignore.tex",
            "chapter/nested.pdf",
            "chapter/source.typ",
        ] {
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
        assert_eq!(entries.len(), 5);
        assert!(entries[1].path.ends_with("nested.pdf"));
        assert!(entries[2].path.ends_with("source.typ"));
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
        assert!(rendered.contains("PDF tree"));
        assert!(rendered.contains("Recent"));
        assert!(rendered.contains("Research notes.pdf"));
        assert!(!rendered.contains("Ctrl+"));
    }
    #[test]
    fn folder_pdf_scan_distinguishes_nested_empty_and_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let stop = std::sync::atomic::AtomicBool::new(false);
        std::fs::create_dir(dir.path().join("nested")).unwrap();
        assert_eq!(contains_pdf(dir.path(), 0, &mut 100, &stop), Some(false));
        std::fs::write(dir.path().join("nested/paper.PDF"), "pdf").unwrap();
        assert_eq!(contains_pdf(dir.path(), 0, &mut 100, &stop), Some(true));
        assert_eq!(contains_pdf(dir.path(), 0, &mut 0, &stop), None);
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(contains_pdf(dir.path(), 0, &mut 100, &stop), None);
    }
    #[test]
    fn tab_cycles_all_panes() {
        let dir = tempfile::tempdir().unwrap();
        let mut files = Files::new(dir.path(), true);
        files.cycle_focus(100);
        assert!(files.focused && files.recent_focused);
        files.cycle_focus(100);
        assert!(!files.focused);
        files.cycle_focus(100);
        assert!(files.focused && !files.recent_focused);
        files.visible = false;
        files.focused = false;
        files.cycle_focus(100);
        assert!(!files.focused);
    }
    #[test]
    fn directional_focus_and_split_resize() {
        let dir = tempfile::tempdir().unwrap();
        let mut files = Files::new(dir.path(), true);
        files.focus_direction('j', 100);
        assert!(files.focused && files.recent_focused);
        files.focus_direction('l', 100);
        assert!(!files.focused);
        files.focus_direction('k', 100);
        assert!(!files.focused); // No window above the full-height PDF.
        files.focus_direction('h', 100);
        assert!(files.focused && files.recent_focused);
        files.focus_direction('k', 100);
        assert!(!files.recent_focused);
        let width = files.width(100);
        files.resize(KeyCode::Right, 100, 40);
        assert_eq!(files.width(100), width + 1);
        files.resize(KeyCode::Left, 100, 40);
        assert_eq!(files.width(100), width);
        let rect = Rect::new(0, 0, width, 40);
        let before = files.panels(rect)[0].height;
        files.resize(KeyCode::Down, 100, 40);
        assert_eq!(files.panels(rect)[0].height, before + 1);
        for _ in 0..100 {
            files.resize(KeyCode::Left, 100, 40);
            files.resize(KeyCode::Up, 100, 40);
        }
        assert_eq!(files.width(100), 16);
        assert_eq!(files.panels(rect)[0].height, 3);
        files.visible = false;
        files.focused = false;
        files.focus_direction('h', 100);
        assert!(!files.visible && !files.focused);
    }
    #[test]
    fn panel_fits_small_terminals() {
        let dir = tempfile::tempdir().unwrap();
        let files = Files::new(dir.path(), true);
        assert_eq!(files.width(40), 0);
        assert_eq!(files.width(90), 30);
    }
}
