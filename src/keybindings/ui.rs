use super::{
    config::{Config, Entry},
    keys::Stroke,
    Invocation, System,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
pub struct Panel {
    settings: bool,
    query: String,
    selected: usize,
    record: Option<Vec<Stroke>>,
    message: String,
    context: String,
    pending: Option<(String, String)>,
    category: Option<String>,
}
impl Panel {
    pub fn new(settings: bool) -> Self {
        Self {
            settings,
            query: String::new(),
            selected: 0,
            record: None,
            message: String::new(),
            context: "normal".into(),
            pending: None,
            category: None,
        }
    }
}
fn rows(system: &System) -> Vec<String> {
    let Some(panel) = &system.ui else {
        return vec![];
    };
    let q = panel.query.to_lowercase();
    system
        .registry
        .commands
        .values()
        .filter(|c| {
            let keys = system
                .engine
                .effective()
                .into_iter()
                .filter(|b| b.invocation.as_ref().is_some_and(|i| i.command == c.id))
                .map(|b| {
                    b.keys
                        .iter()
                        .map(Stroke::label)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join(" ");
            let context = system
                .engine
                .effective()
                .into_iter()
                .filter(|b| b.invocation.as_ref().is_some_and(|i| i.command == c.id))
                .map(|b| format!("{} {}", b.context, b.mode))
                .collect::<Vec<_>>()
                .join(" ");
            panel
                .category
                .as_ref()
                .is_none_or(|category| category == &c.category)
                && format!(
                    "{} {} {} {} {}",
                    c.id, c.description, c.category, keys, context
                )
                .to_lowercase()
                .contains(&q)
        })
        .map(|c| c.id.clone())
        .collect()
}
fn save(system: &mut System, id: &str, key: &str, entry: Option<Entry>) -> Result<(), String> {
    let path = system.paths.first().ok_or("no config path")?.clone();
    let mut config = match std::fs::read_to_string(&path) {
        Ok(text) => Config::parse(&text)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => return Err(e.to_string()),
    };
    let scope = system.ui.as_ref().unwrap().context.clone();
    if let Some(entry) = entry {
        config
            .bindings
            .entry(scope)
            .or_default()
            .insert(key.into(), entry);
    } else if let Some(map) = config.bindings.get_mut(&scope) {
        map.remove(key);
    }
    let _ = id;
    let contents = config.export()?;
    std::fs::create_dir_all(path.parent().ok_or("no parent")?).map_err(|e| e.to_string())?;
    // Atomic write preserves other key entries and prevents partial hot reload.
    use std::io::Write;
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(|e| e.to_string())?;
    temp.write_all(contents.as_bytes())
        .map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    system.reload()
}
pub fn key(system: &mut System, key: KeyEvent) -> Option<Invocation> {
    let ids = rows(system);
    let panel = system.ui.as_mut()?;
    if key.code == KeyCode::Esc {
        if panel.record.take().is_some() {
            return None;
        }
        system.ui = None;
        return None;
    }
    if let Some(record) = &mut panel.record {
        if key.code == KeyCode::Enter {
            let sequence = record
                .iter()
                .map(Stroke::label)
                .collect::<Vec<_>>()
                .join(" ");
            panel.record = None;
            let id = ids.get(panel.selected).cloned()?;
            let (scope, mode) = super::config::scope(&panel.context);
            let conflicts = system
                .engine
                .bindings
                .iter()
                .filter(|b| {
                    b.context == scope
                        && b.mode == mode
                        && b.keys
                            .iter()
                            .map(Stroke::label)
                            .collect::<Vec<_>>()
                            .join(" ")
                            == sequence
                })
                .filter_map(|b| b.invocation.as_ref().map(|i| i.command.clone()))
                .collect::<Vec<_>>();
            if !conflicts.is_empty() {
                panel.message = format!(
                    "Conflict: {}. Enter replaces at user priority; Esc cancels.",
                    conflicts.join(", ")
                );
                panel.pending = Some((id, sequence));
                return None;
            }
            if let Err(e) = save(system, &id, &sequence, Some(Entry::Command(id.clone()))) {
                system.ui.as_mut().unwrap().message = e;
            }
            return None;
        }
        if record.len() < 16 {
            record.push(Stroke::event(key));
        }
        return None;
    }
    if let Some((id, sequence)) = panel.pending.take() {
        if key.code == KeyCode::Enter {
            if let Err(e) = save(system, &id, &sequence, Some(Entry::Command(id.clone()))) {
                system.ui.as_mut().unwrap().message = e;
            }
        }
        return None;
    }
    match key.code {
        KeyCode::F(2) if panel.settings => {
            let mut categories = system
                .registry
                .commands
                .values()
                .map(|c| c.category.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .map(Some)
                .collect::<Vec<_>>();
            categories.insert(0, None);
            let index = categories
                .iter()
                .position(|c| *c == panel.category)
                .unwrap_or(0);
            panel.category = categories[(index + 1) % categories.len()].clone();
            panel.selected = 0;
        }
        KeyCode::F(5) if panel.settings => {
            let id = ids.get(panel.selected)?.clone();
            let scope = panel.context.clone();
            let path = system.paths[0].clone();
            let result = (|| -> Result<(), String> {
                let mut config = std::fs::read_to_string(&path)
                    .ok()
                    .map(|s| Config::parse(&s))
                    .transpose()?
                    .unwrap_or_default();
                if let Some(map) = config.bindings.get_mut(&scope) {
                    map.retain(|_, entry| match entry {
                        Entry::Command(c) => c != &id,
                        Entry::Detail { command, .. } => command.as_ref() != Some(&id),
                    });
                }
                std::fs::write(&path, config.export()?).map_err(|e| e.to_string())?;
                system.reload()
            })();
            system.ui.as_mut().unwrap().message = result
                .err()
                .unwrap_or_else(|| "Restored user overrides for selected command".into());
        }
        KeyCode::F(6) if panel.settings => {
            let presets = ["astronvim", "vim", "neovim", "leafview-default", "custom"];
            let current = presets
                .iter()
                .position(|p| *p == system.config.preset)
                .unwrap_or(0);
            let path = system.paths[0].clone();
            let result = (|| -> Result<(), String> {
                let mut c = std::fs::read_to_string(&path)
                    .ok()
                    .map(|s| Config::parse(&s))
                    .transpose()?
                    .unwrap_or_default();
                c.preset = presets[(current + 1) % presets.len()].into();
                std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::write(&path, c.export()?).map_err(|e| e.to_string())?;
                system.reload()
            })();
            system.ui.as_mut().unwrap().message = result
                .err()
                .unwrap_or_else(|| format!("Preset: {}", system.config.preset));
        }
        KeyCode::F(7) if panel.settings => {
            let path = system.paths[0].with_file_name("keybindings-export.toml");
            let result = system
                .config
                .export()
                .and_then(|text| std::fs::write(&path, text).map_err(|e| e.to_string()));
            system.ui.as_mut().unwrap().message = result
                .err()
                .unwrap_or_else(|| format!("Exported {}", path.display()));
        }
        KeyCode::F(8) if panel.settings => {
            let input = system.paths[0].with_file_name("keybindings-import.toml");
            let result = (|| -> Result<(), String> {
                let text = std::fs::read_to_string(&input).map_err(|e| e.to_string())?;
                Config::parse(&text)?;
                let mut paths = system.paths.clone();
                paths[0] = input;
                super::config::load(&paths, &system.registry)?;
                std::fs::write(&system.paths[0], text).map_err(|e| e.to_string())?;
                system.reload()
            })();
            system.ui.as_mut().unwrap().message = result
                .err()
                .unwrap_or_else(|| "Imported keybindings-import.toml".into());
        }
        KeyCode::Down => panel.selected = (panel.selected + 1).min(ids.len().saturating_sub(1)),
        KeyCode::Up => panel.selected = panel.selected.saturating_sub(1),
        KeyCode::Enter if !panel.settings => {
            let id = ids.get(panel.selected)?.clone();
            system.ui = None;
            return Some(Invocation::new(&id));
        }
        KeyCode::Enter if panel.settings => {
            panel.record = Some(vec![]);
            panel.message = "Record sequence; Enter saves, Esc cancels".into();
        }
        KeyCode::Tab if panel.settings => {
            let scopes = ["normal", "insert", "visual", "pdf", "explorer", "global"];
            let i = scopes.iter().position(|s| *s == panel.context).unwrap_or(0);
            panel.context = scopes[(i + 1) % scopes.len()].into();
        }
        KeyCode::Delete if panel.settings => {
            let id = ids.get(panel.selected)?.clone();
            let scope = panel.context.clone();
            let (ctx, mode) = super::config::scope(&scope);
            let bindings = system
                .engine
                .effective()
                .into_iter()
                .filter(|b| {
                    b.context == ctx
                        && b.mode == mode
                        && b.invocation.as_ref().is_some_and(|i| i.command == id)
                })
                .map(|b| {
                    b.keys
                        .iter()
                        .map(Stroke::label)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>();
            for binding in bindings {
                if let Err(e) = save(
                    system,
                    &id,
                    &binding,
                    Some(Entry::Detail {
                        command: None,
                        args: serde_json::Value::Null,
                        disabled: true,
                        priority: 0,
                    }),
                ) {
                    system.ui.as_mut().unwrap().message = e;
                }
            }
        }
        KeyCode::Backspace => {
            panel.query.pop();
            panel.selected = 0;
        }
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            panel.query.push(c);
            panel.selected = 0;
        }
        _ => {}
    }
    None
}
pub fn render(system: &System, f: &mut Frame) {
    if let Some(panel) = &system.ui {
        let screen = f.area();
        let width = screen.width.saturating_sub(4).min(120);
        let height = screen.height.saturating_sub(4).min(30);
        if width < 8 || height < 5 {
            return;
        }
        let area = Rect::new(
            (screen.width - width) / 2,
            (screen.height - height) / 2,
            width,
            height,
        );
        let ids = rows(system);
        let mut lines = vec![
            Line::from("F2 category · Tab scope · F5 reset · F6 preset · F7 export · F8 import"),
            Line::from(format!(
                "Search: {}    scope: {}    category: {} (F2)",
                panel.query,
                panel.context,
                panel.category.as_deref().unwrap_or("All")
            )),
        ];
        let first = panel
            .selected
            .saturating_sub(height.saturating_sub(7) as usize);
        for (i, id) in ids
            .iter()
            .enumerate()
            .skip(first)
            .take(height.saturating_sub(6) as usize)
        {
            let c = &system.registry.commands[id];
            let bindings = system
                .engine
                .effective()
                .into_iter()
                .filter(|b| b.invocation.as_ref().is_some_and(|v| v.command == *id))
                .map(|b| {
                    format!(
                        "{} [{} {}; {}]",
                        b.keys
                            .iter()
                            .map(Stroke::label)
                            .collect::<Vec<_>>()
                            .join(" "),
                        b.context,
                        b.mode,
                        b.source
                    )
                })
                .collect::<Vec<_>>()
                .join(" · ");
            lines.push(Line::from(format!(
                "{} {}  {}  {}",
                if i == panel.selected { "›" } else { " " },
                c.description,
                if c.available { "" } else { "[unavailable]" },
                bindings
            )));
        }
        if let Some(record) = &panel.record {
            lines.push(Line::from(format!(
                "Recording: {}",
                record
                    .iter()
                    .map(Stroke::label)
                    .collect::<Vec<_>>()
                    .join(" → ")
            )));
        }
        lines.push(Line::from(panel.message.clone()));
        let overlaps = system
            .engine
            .conflicts
            .iter()
            .map(|c| {
                format!(
                    "{} > {}{}",
                    system.engine.bindings[c.winner].source,
                    system.engine.bindings[c.loser].source,
                    if c.prefix { " (prefix timeout)" } else { "" }
                )
            })
            .collect::<Vec<_>>();
        lines.push(Line::from(format!(
            "Conflicts/overrides: {} {}",
            overlaps.len(),
            overlaps.first().map(String::as_str).unwrap_or("")
        )));
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(lines)
                .style(
                    Style::default()
                        .fg(Color::Rgb(210, 220, 235))
                        .bg(Color::Rgb(20, 25, 33)),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if panel.settings {
                            " Keyboard Shortcuts · Enter edit · Del disable · Esc "
                        } else {
                            " Command Palette · Enter execute · Esc "
                        }),
                ),
            area,
        );
        return;
    }
    if !system.engine.waiting() {
        return;
    }
    let hints = system.engine.hints();
    let screen = f.area();
    let height = (hints.len() as u16 + 3)
        .min(screen.height / 2)
        .max(3)
        .min(screen.height);
    let area = Rect::new(0, screen.height - height, screen.width, height);
    let mut lines = vec![Line::from(system.engine.prefix())];
    for (key, id) in hints {
        let desc = system
            .registry
            .commands
            .get(&id)
            .map(|c| c.description.as_str())
            .unwrap_or("Group");
        lines.push(Line::from(format!("  {key:12} {desc}")));
    }
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .style(
                Style::default()
                    .fg(Color::Rgb(155, 220, 230))
                    .bg(Color::Rgb(25, 32, 43)),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" WhichKey · Esc cancel "),
            ),
        area,
    );
}
