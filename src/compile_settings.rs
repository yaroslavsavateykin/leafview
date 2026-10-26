//! Keyboard-only settings editor; drafts never touch the running compiler.
use crate::compile_config::{Effective, Profile, Session};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::path::Path;

const FIELDS: &[&str] = &[
    "Profile",
    "root",
    "inputs",
    "fonts",
    "jobs",
    "package",
    "cache",
    "cert",
    "timestamp",
    "features",
    "standards",
    "pages",
    "ignore_system",
    "ignore_embedded",
    "pretty",
    "no_tags",
    "write_pdf",
    "auto_compile",
    "auto_sync",
    "Apply",
    "Save project",
    "Save global",
    "Cancel",
];
pub enum Action {
    None,
    Cancel,
    Apply(Box<Session>, Box<Effective>, Option<bool>),
}
pub struct Settings {
    pub draft: Session,
    value: Effective,
    selected: usize,
    edit: Option<String>,
    pub error: Option<String>,
}
impl Settings {
    pub fn new(session: &Session) -> Self {
        let mut draft = session.clone();
        // Re-read disk layers when opening settings; runtime/CLI remain higher priority.
        for (path, target) in [
            (&session.global_path, &mut draft.global),
            (&session.project_path, &mut draft.project),
        ] {
            match crate::compile_config::Config::load(path) {
                Ok(config) => *target = config,
                Err(error) => draft.warning = Some(format!("{error:#}")),
            }
        }
        let mut panel = Self {
            draft,
            value: Effective::default(),
            selected: 0,
            edit: None,
            error: None,
        };
        panel.error = panel.draft.warning.clone();
        match panel.draft.effective() {
            Ok(v) => panel.value = v,
            Err(e) => panel.error = Some(format!("{e:#}")),
        }
        panel
    }
    fn json(&self) -> serde_json::Value {
        serde_json::to_value(Profile::snapshot(&self.value).expect("serializable options"))
            .expect("serializable profile")
    }
    fn stash(&mut self) -> Result<()> {
        anyhow::ensure!(
            self.draft.names().len() <= 128,
            "too many profiles (maximum 128)"
        );
        self.draft
            .runtime
            .profiles
            .insert(self.draft.selected.clone(), Profile::snapshot(&self.value)?);
        Ok(())
    }
    fn select_profile(&mut self, name: String) -> Result<()> {
        anyhow::ensure!(
            !name.trim().is_empty() && name.len() <= 128,
            "profile name must contain 1–128 bytes"
        );
        self.stash()?;
        let old = self.draft.selected.clone();
        self.draft.selected = name;
        match self.draft.effective() {
            Ok(v) => self.value = v,
            Err(e) => {
                self.draft.selected = old;
                return Err(e);
            }
        }
        Ok(())
    }
    fn commit_edit(&mut self, text: String) -> Result<()> {
        if self.selected == 0 {
            return self.select_profile(text);
        }
        let field = FIELDS[self.selected];
        let mut json = self.json();
        let parsed = if text.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text)
                .context("enter a JSON value (strings in quotes, arrays in brackets)")?
        };
        // Empty inputs clears all keys, unlike merging a partial config layer.
        json[field] = parsed;
        let mut p: Profile = serde_json::from_value(json)?;
        p.resolve_paths(self.draft.project_path.parent().unwrap());
        let mut v = Effective::default();
        p.options.apply(&mut v.options);
        v.options.no_pdf = !p.write_pdf.unwrap_or(true);
        v.auto_compile = p.auto_compile.unwrap_or(true);
        v.auto_sync = p.auto_sync.unwrap_or(true);
        crate::compile_config::validate_values(&v.options)?;
        self.value = v;
        Ok(())
    }
    pub fn key(&mut self, key: KeyEvent, input: &Path) -> Action {
        if let Some(mut text) = self.edit.take() {
            match key.code {
                KeyCode::Esc => return Action::None,
                KeyCode::Enter => {
                    if let Err(e) = self.commit_edit(text.clone()) {
                        self.error = Some(format!("{e:#}"));
                        self.edit = Some(text);
                    } else {
                        self.error = None;
                    }
                    return Action::None;
                }
                KeyCode::Backspace => {
                    text.pop();
                }
                KeyCode::Delete => text.clear(),
                KeyCode::Char(c)
                    if !key.modifiers.intersects(
                        crossterm::event::KeyModifiers::CONTROL
                            | crossterm::event::KeyModifiers::ALT,
                    ) =>
                {
                    text.push(c)
                }
                _ => {}
            }
            self.edit = Some(text);
            return Action::None;
        }
        match key.code {
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Tab | KeyCode::Down => self.selected = (self.selected + 1) % FIELDS.len(),
            KeyCode::BackTab | KeyCode::Up => {
                self.selected = (self.selected + FIELDS.len() - 1) % FIELDS.len()
            }
            KeyCode::Left | KeyCode::Right if self.selected == 0 => {
                let names = self.draft.names();
                let i = names
                    .iter()
                    .position(|n| n == &self.draft.selected)
                    .unwrap_or(0);
                let next = if key.code == KeyCode::Right {
                    (i + 1) % names.len()
                } else {
                    (i + names.len() - 1) % names.len()
                };
                if let Err(e) = self.select_profile(names[next].clone()) {
                    self.error = Some(format!("{e:#}"));
                }
            }
            KeyCode::Enter => {
                if self.selected == FIELDS.len() - 1 {
                    return Action::Cancel;
                }
                if self.selected >= 19 {
                    let result = (|| -> Result<()> {
                        crate::compile_config::validate(input, &self.value.options)?;
                        self.stash()?;
                        Ok(())
                    })();
                    if let Err(e) = result {
                        self.error = Some(format!("{e:#}"));
                        return Action::None;
                    }
                    let save = match self.selected {
                        20 => Some(false),
                        21 => Some(true),
                        _ => None,
                    };
                    return Action::Apply(
                        Box::new(self.draft.clone()),
                        Box::new(self.value.clone()),
                        save,
                    );
                }
                if self.selected >= 12 {
                    let field = FIELDS[self.selected];
                    let text = (!self.json()[field].as_bool().unwrap_or(false)).to_string();
                    if let Err(e) = self.commit_edit(text) {
                        self.error = Some(format!("{e:#}"));
                    }
                } else {
                    self.edit = Some(if self.selected == 0 {
                        self.draft.selected.clone()
                    } else {
                        self.json()[FIELDS[self.selected]].to_string()
                    });
                }
            }
            _ => {}
        }
        Action::None
    }
    pub fn render(&self, f: &mut Frame<'_>) {
        let outer = f.area();
        let width = outer.width.min(96);
        let height = outer.height.min(33);
        let area = Rect::new(
            outer.x + (outer.width - width) / 2,
            outer.y + (outer.height - height) / 2,
            width,
            height,
        );
        f.render_widget(Clear, area);
        let json = self.json();
        let available = height.saturating_sub(8).max(1) as usize;
        let start = self.selected.saturating_sub(available.saturating_sub(1));
        let mut lines: Vec<ratatui::text::Line<'_>> = Vec::new();
        for (i, field) in FIELDS.iter().enumerate().skip(start).take(available) {
            let value = if i == 0 {
                self.draft.selected.clone()
            } else if i >= 19 {
                String::new()
            } else {
                json[*field].to_string()
            };
            let text = if i == self.selected {
                self.edit.as_ref().map(|s| format!("{s}▏")).unwrap_or(value)
            } else {
                value
            };
            let line = format!(
                "{} {:<16} {}",
                if i == self.selected { "›" } else { " " },
                field,
                crate::links::sanitize(&text)
            );
            lines.push(ratatui::text::Line::styled(
                line,
                if i == self.selected {
                    Style::default()
                        .fg(Color::LightCyan)
                        .bg(Color::Rgb(37, 53, 65))
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default()
                },
            ));
        }
        lines.push("".into());
        lines.push("Tab/Shift+Tab ↑↓ select · Enter edit/toggle · Esc cancel".into());
        lines.push("Profile: ←→ choose · Enter rename/create · edit: Delete clears".into());
        lines.push("Values are JSON: \"path\", [\"fonts\"], {\"mode\":\"teacher\"}, null".into());
        if let Some(e) = &self.error {
            lines.push(ratatui::text::Line::styled(
                crate::links::sanitize(e),
                Style::default().fg(Color::LightRed),
            ));
        }
        f.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Compile Settings ")
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .style(
                    Style::default()
                        .fg(Color::Rgb(220, 225, 234))
                        .bg(Color::Rgb(25, 29, 36)),
                ),
            area,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panel_is_keyboard_editable_and_renders() {
        let d = tempfile::tempdir().unwrap();
        let s = Session::from_paths(
            d.path().join("g.json"),
            d.path().join(".leafview.json"),
            Profile::default(),
            None,
        );
        let mut panel = Settings::new(&s);
        panel.selected = 2;
        panel
            .commit_edit(r#"{"mode":"teacher","solutions":"true"}"#.into())
            .unwrap();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 35)).unwrap();
        terminal.draw(|f| panel.render(f)).unwrap();
        let buffer = terminal.backend().buffer();
        let rendered = (0..35)
            .map(|y| {
                (0..100)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("Compile Settings") && rendered.contains("teacher"));
        if let Ok(path) = std::env::var("LEAFVIEW_SETTINGS_SNAPSHOT") {
            std::fs::write(path, rendered).unwrap();
        }
    }
    #[test]
    fn draft_edit_cancel_and_profile_creation() {
        let d = tempfile::tempdir().unwrap();
        let input = d.path().join("main.typ");
        std::fs::write(&input, "hello").unwrap();
        let session = Session::from_paths(
            d.path().join("global.json"),
            d.path().join(".leafview.json"),
            Profile::default(),
            None,
        );
        let mut p = Settings::new(&session);
        p.commit_edit("teacher".into()).unwrap();
        p.selected = 2;
        p.commit_edit(r#"{"mode":"teacher"}"#.into()).unwrap();
        assert_eq!(
            p.value.options.inputs,
            vec![("mode".into(), "teacher".into())]
        );
        p.commit_edit("{}".into()).unwrap();
        assert!(p.value.options.inputs.is_empty());
        p.selected = 4;
        assert!(p.commit_edit("0".into()).is_err());
        assert!(p.commit_edit("257".into()).is_err());
        p.selected = 3;
        p.commit_edit("[\"fonts\"]".into()).unwrap();
        assert_eq!(p.value.options.fonts, vec![d.path().join("fonts")]);
        assert!(session.runtime.profiles.is_empty());
        assert!(matches!(
            p.key(KeyEvent::from(KeyCode::Esc), &input),
            Action::Cancel
        ));
    }
}
