//! Modal editor for the actual compiler flags; no profiles or metadata.
use crate::compile_config::{Effective, Flags, Session};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::path::Path;
const FIELDS: &[&str] = &[
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
    "Save file",
    "Cancel",
];
const APPLY: usize = 18;
pub enum Action {
    None,
    Cancel,
    Apply(Box<Session>, Box<Effective>, bool),
}
pub struct Settings {
    draft: Session,
    value: Effective,
    selected: usize,
    edit: Option<String>,
    pub error: Option<String>,
}
impl Settings {
    pub fn new(session: &Session) -> Self {
        let mut draft = session.clone();
        for (path, target) in [
            (&session.global_path, &mut draft.global),
            (&session.project_path, &mut draft.project),
        ] {
            match crate::compile_config::Config::load(path) {
                Ok(config) => *target = config,
                Err(e) => draft.warning = Some(format!("{e:#}")),
            }
        }
        let mut panel = Self {
            error: draft.warning.clone(),
            draft,
            value: Effective::default(),
            selected: 0,
            edit: None,
        };
        match panel.draft.effective() {
            Ok(v) => panel.value = v,
            Err(e) => panel.error = Some(format!("{e:#}")),
        }
        panel
    }
    fn json(&self) -> serde_json::Value {
        serde_json::to_value(Flags::snapshot(&self.value).expect("serializable flags"))
            .expect("serializable flags")
    }
    fn commit_edit(&mut self, text: String) -> Result<()> {
        let mut json = self.json();
        json[FIELDS[self.selected]] = if text.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text)?
        };
        let mut flags: Flags = serde_json::from_value(json)?;
        flags.resolve_paths(self.draft.input.parent().unwrap());
        let mut value = Effective::default();
        flags.apply(&mut value);
        crate::compile_config::validate_values(&value.options)?;
        self.value = value;
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
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
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
            KeyCode::Enter => {
                if self.selected == 20 {
                    return Action::Cancel;
                }
                if self.selected >= APPLY {
                    if let Err(e) = crate::compile_config::validate(input, &self.value.options) {
                        self.error = Some(format!("{e:#}"));
                        return Action::None;
                    }
                    self.draft.runtime =
                        Some(Flags::snapshot(&self.value).expect("serializable flags"));
                    return Action::Apply(
                        Box::new(self.draft.clone()),
                        Box::new(self.value.clone()),
                        self.selected == 19,
                    );
                }
                if self.selected >= 11 {
                    if let Err(e) = self.commit_edit(
                        (!self.json()[FIELDS[self.selected]]
                            .as_bool()
                            .unwrap_or(false))
                        .to_string(),
                    ) {
                        self.error = Some(format!("{e:#}"));
                    }
                } else {
                    self.edit = Some(self.json()[FIELDS[self.selected]].to_string());
                }
            }
            _ => {}
        }
        Action::None
    }
    pub fn render(&self, f: &mut Frame<'_>) {
        let outer = f.area();
        let width = outer.width.min(96);
        let height = outer.height.min(31);
        let area = Rect::new(
            outer.x + (outer.width - width) / 2,
            outer.y + (outer.height - height) / 2,
            width,
            height,
        );
        f.render_widget(Clear, area);
        let json = self.json();
        let available = height.saturating_sub(7).max(1) as usize;
        let start = self.selected.saturating_sub(available.saturating_sub(1));
        let mut lines: Vec<Line<'_>> = Vec::new();
        for (i, field) in FIELDS.iter().enumerate().skip(start).take(available) {
            let value = if i >= APPLY {
                String::new()
            } else {
                json[*field].to_string()
            };
            let value = if i == self.selected {
                self.edit.as_ref().map(|s| format!("{s}▏")).unwrap_or(value)
            } else {
                value
            };
            lines.push(Line::styled(
                format!(
                    "{} {:<16} {}",
                    if i == self.selected { "›" } else { " " },
                    field,
                    crate::links::sanitize(&value)
                ),
                if i == self.selected {
                    Style::default()
                        .fg(Color::LightCyan)
                        .bg(Color::Rgb(37, 53, 65))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            ));
        }
        lines.push("".into());
        lines.push("Tab/Shift+Tab ↑↓ select · Enter edit/toggle · Esc cancel".into());
        lines.push("JSON values: \"path\", [\"fonts\"], {\"mode\":\"teacher\"}, null".into());
        lines.push("Delete clears edit · Save file stores flags for this path only".into());
        if let Some(e) = &self.error {
            lines.push(Line::styled(
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
    fn edit_clear_validate_and_cancel() {
        let d = tempfile::tempdir().unwrap();
        let input = d.path().join("a.typ");
        std::fs::write(&input, "").unwrap();
        let s = Session::from_paths(
            input.clone(),
            d.path().join("global.json"),
            d.path().join(".leafview.json"),
            Flags::default(),
        );
        let mut p = Settings::new(&s);
        p.selected = 1;
        p.commit_edit(r#"{"mode":"teacher"}"#.into()).unwrap();
        assert_eq!(p.value.options.inputs.len(), 1);
        p.commit_edit("{}".into()).unwrap();
        assert!(p.value.options.inputs.is_empty());
        p.selected = 3;
        assert!(p.commit_edit("0".into()).is_err());
        assert!(s.runtime.is_none());
        assert!(matches!(
            p.key(KeyEvent::from(KeyCode::Esc), &input),
            Action::Cancel
        ));
    }
}
