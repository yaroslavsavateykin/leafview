//! Typed controls for all supported Typst flags, document inputs and viewer behavior.
use crate::compile_config::{Effective, Flags, Session};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::path::Path;

#[derive(Clone, Copy)]
struct Field {
    key: &'static str,
    label: &'static str,
    help: &'static str,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Kind {
    Text,
    Number,
    List,
    Toggle,
    Inverse,
}
const TYPST: &[Field] = &[
    Field {
        key: "root",
        label: "Project root",
        help: "--root · Sandbox root containing the input. Empty = input directory.",
        kind: Kind::Text,
    },
    Field {
        key: "fonts",
        label: "Font directories",
        help: "--font-path · One directory per ; entry. Empty = no extra fonts.",
        kind: Kind::List,
    },
    Field {
        key: "jobs",
        label: "Compiler threads",
        help: "--jobs · 1–256. Empty = automatic (currently 2 threads).",
        kind: Kind::Number,
    },
    Field {
        key: "pages",
        label: "PDF pages",
        help: "--pages · e.g. 1-3,5,8-. Empty = all pages.",
        kind: Kind::Text,
    },
    Field {
        key: "ignore_system",
        label: "System fonts",
        help: "--ignore-system-fonts (inverted) · Use fonts installed on this system.",
        kind: Kind::Inverse,
    },
    Field {
        key: "ignore_embedded",
        label: "Bundled fonts",
        help: "--ignore-embedded-fonts (inverted) · Use bundled Typst fonts.",
        kind: Kind::Inverse,
    },
];
const ADVANCED: &[Field] = &[
    Field {
        key: "package",
        label: "Local packages",
        help: "--package-path · Local Typst package directory. Empty = system default.",
        kind: Kind::Text,
    },
    Field {
        key: "cache",
        label: "Package cache",
        help: "--package-cache-path · Downloaded package cache. Empty = system default.",
        kind: Kind::Text,
    },
    Field {
        key: "cert",
        label: "CA certificate",
        help: "--cert · Custom CA certificate file for package downloads. Usually leave empty.",
        kind: Kind::Text,
    },
    Field {
        key: "timestamp",
        label: "Creation timestamp",
        help: "--creation-timestamp · Unix seconds. Empty = current date/time.",
        kind: Kind::Number,
    },
    Field {
        key: "features",
        label: "Experimental features",
        help: "--features · Comma-separated: html, bundle, a11y-extras. Usually leave empty.",
        kind: Kind::List,
    },
    Field {
        key: "standards",
        label: "PDF standards",
        help: "--pdf-standard · Comma-separated standards, e.g. 1.7 or a-2b.",
        kind: Kind::List,
    },
    Field {
        key: "pretty",
        label: "Readable PDF encoding",
        help: "--pretty · Human-readable PDF output; usually larger files.",
        kind: Kind::Toggle,
    },
    Field {
        key: "no_tags",
        label: "PDF accessibility tags",
        help: "--no-pdf-tags (inverted) · Include PDF accessibility tags.",
        kind: Kind::Inverse,
    },
];
const LEAFVIEW: &[Field] = &[
    Field {
        key: "auto_compile",
        label: "Compile on save",
        help: "Automatically rebuild when the source or dependencies change. r builds manually.",
        kind: Kind::Toggle,
    },
    Field {
        key: "write_pdf",
        label: "Save PDF beside source",
        help: "Publish <input>.pdf after successful builds. Off = private preview only.",
        kind: Kind::Toggle,
    },
    Field {
        key: "auto_sync",
        label: "Follow source edits",
        help: "Move to an off-screen edited location after compilation; preserve zoom.",
        kind: Kind::Toggle,
    },
];
pub enum Action {
    None,
    Cancel,
    Apply(Box<Session>, Box<Effective>, bool),
}
pub struct Settings {
    draft: Session,
    value: Effective,
    tab: usize,
    selected: usize,
    edit: Option<TextEdit>,
    inputs: Option<InputsEditor>,
    input_actions: bool,
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
                Ok(v) => *target = v,
                Err(e) => draft.warning = Some(format!("{e:#}")),
            }
        }
        let mut panel = Self {
            error: draft.warning.clone(),
            draft,
            value: Effective::default(),
            tab: 0,
            selected: 0,
            edit: None,
            inputs: None,
            input_actions: false,
        };
        match panel.draft.editable() {
            Ok(v) => panel.value = v,
            Err(e) => panel.error = Some(format!("{e:#}")),
        }
        panel
    }
    fn fields(&self) -> Vec<Field> {
        if self.tab == 2 {
            LEAFVIEW.to_vec()
        } else {
            TYPST.iter().chain(ADVANCED.iter()).copied().collect()
        }
    }
    fn json(&self) -> serde_json::Value {
        serde_json::to_value(Flags::snapshot(&self.value).expect("serializable flags"))
            .expect("serializable flags")
    }
    fn display(&self, field: Field, editing: bool) -> String {
        let json = self.json();
        let v = &json[field.key];
        match field.kind {
            Kind::Inverse | Kind::Toggle => {
                let mut on = v.as_bool().unwrap_or(false);
                if matches!(field.kind, Kind::Inverse) {
                    on = !on;
                }
                if on { "On" } else { "Off" }.into()
            }
            Kind::List => v
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(if field.key == "fonts" { "; " } else { ", " })
                })
                .unwrap_or_default(),
            _ => {
                if v.is_null() {
                    String::new()
                } else if let Some(s) = v.as_str() {
                    s.into()
                } else {
                    v.to_string()
                }
            }
        }
        .pipe(|text| {
            if text.is_empty() && !editing {
                "Default".into()
            } else {
                text
            }
        })
    }
    fn commit_edit(&mut self, text: String) -> Result<()> {
        let field = self.fields()[self.selected];
        let text = text.trim();
        let v = match field.kind {
            Kind::Text => {
                if text.is_empty() || text == "null" {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(if text.starts_with('"') {
                        serde_json::from_str::<String>(text)?
                    } else {
                        text.into()
                    })
                }
            }
            Kind::Number => {
                if text.is_empty() || text == "null" {
                    serde_json::Value::Null
                } else {
                    serde_json::from_str(text).context("enter a whole number")?
                }
            }
            Kind::List => {
                if text.starts_with('[') {
                    serde_json::from_str(text)?
                } else {
                    serde_json::to_value(
                        text.split(if field.key == "fonts" { ';' } else { ',' })
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>(),
                    )?
                }
            }
            _ => serde_json::from_str(text)?,
        };
        let mut json = self.json();
        json[field.key] = v;
        let flags: Flags = serde_json::from_value(json)?;
        let mut value = Effective::default();
        flags.apply(&mut value);
        crate::compile_config::validate_values(&value.options)?;
        self.value = value;
        Ok(())
    }
    fn finish_edit(&mut self) -> bool {
        if let Some(text) = self.edit.take() {
            if let Err(e) = self.commit_edit(text.text.clone()) {
                self.error = Some(format!("{e:#}"));
                self.edit = Some(text);
                return false;
            }
        }
        self.error = None;
        true
    }
    fn apply(&mut self, input: &Path, save: bool) -> Action {
        let mut resolved = Flags::snapshot(&self.value).expect("serializable flags");
        resolved.resolve_paths(input.parent().unwrap());
        let mut effective = Effective::default();
        resolved.apply(&mut effective);
        if let Err(e) = crate::compile_config::validate(input, &effective.options) {
            self.error = Some(format!("{e:#}"));
            return Action::None;
        }
        self.draft.runtime = Some(Flags::snapshot(&self.value).expect("serializable flags"));
        Action::Apply(Box::new(self.draft.clone()), Box::new(effective), save)
    }
    fn switch_tab(&mut self, right: bool) {
        self.tab = (self.tab + if right { 1 } else { 2 }) % 3;
        self.selected = 0;
        self.input_actions = false;
        if self.tab == 1 {
            self.inputs = Some(InputsEditor::new(&self.value.options.inputs));
        }
    }
    pub fn key(&mut self, key: KeyEvent, input: &Path) -> Action {
        if self.tab == 1 && self.input_actions {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('s' | 'a'))
            {
                match self.inputs.as_mut().unwrap().finish() {
                    Ok(values) => {
                        self.value.options.inputs = values;
                        return self.apply(input, key.code == KeyCode::Char('s'));
                    }
                    Err(e) => {
                        self.inputs.as_mut().unwrap().error = Some(format!("{e:#}"));
                        self.input_actions = false;
                        return Action::None;
                    }
                }
            }
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.selected = (self.selected + 2) % 3,
                KeyCode::Right | KeyCode::Char('l') => self.selected = (self.selected + 1) % 3,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => self.input_actions = false,
                KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                    self.selected = (self.selected + 1) % 3
                }
                KeyCode::Enter => {
                    if self.selected == 2 {
                        return Action::Cancel;
                    }
                    match self.inputs.as_mut().unwrap().finish() {
                        Ok(values) => {
                            self.value.options.inputs = values;
                            return self.apply(input, self.selected == 1);
                        }
                        Err(e) => {
                            self.inputs.as_mut().unwrap().error = Some(format!("{e:#}"));
                            self.input_actions = false;
                        }
                    }
                }
                KeyCode::Esc => return Action::Cancel,
                _ => {}
            }
            return Action::None;
        }
        if self.tab == 1
            && self.inputs.as_ref().is_some_and(|e| {
                e.edit.is_none() && (e.rows.is_empty() || e.row + 1 >= e.rows.len())
            })
            && matches!(key.code, KeyCode::Down | KeyCode::Char('j'))
        {
            self.input_actions = true;
            self.selected = 0;
            return Action::None;
        }
        if self.tab == 1 && self.inputs.as_ref().is_some_and(|e| e.edit.is_none()) {
            if matches!(key.code, KeyCode::Left | KeyCode::Right) {
                let result = self.inputs.as_mut().unwrap().finish();
                match result {
                    Ok(values) => {
                        self.value.options.inputs = values;
                        self.inputs = None;
                        self.switch_tab(key.code == KeyCode::Right);
                    }
                    Err(e) => self.inputs.as_mut().unwrap().error = Some(format!("{e:#}")),
                }
                return Action::None;
            }
            if key.code == KeyCode::Esc {
                return Action::Cancel;
            }
        }
        if let Some(editor) = &mut self.inputs {
            let save = key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('s' | 'a'));
            if save || key.code == KeyCode::F(2) {
                match editor.finish() {
                    Ok(values) => {
                        self.value.options.inputs = values;
                        self.inputs = None;
                        if !save {
                            self.tab = 0;
                        }
                        self.error = None;
                        if save {
                            return self.apply(input, key.code == KeyCode::Char('s'));
                        }
                    }
                    Err(e) => editor.error = Some(format!("{e:#}")),
                }
            } else if key.code == KeyCode::Esc && editor.edit.is_none() {
                return Action::Cancel;
            } else {
                editor.key(key);
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('s' | 'a'))
        {
            if !self.finish_edit() {
                return Action::None;
            }
            return self.apply(input, key.code == KeyCode::Char('s'));
        }
        if let Some(mut text) = self.edit.take() {
            match key.code {
                KeyCode::Esc => return Action::None,
                KeyCode::Enter => {
                    self.edit = Some(text);
                    self.finish_edit();
                    return Action::None;
                }
                _ => text.key(key),
            }
            self.edit = Some(text);
            return Action::None;
        }
        let n = self.fields().len();
        let extra = 0;
        let total = n + 3;
        match key.code {
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h' | 'l') => {
                let right = matches!(key.code, KeyCode::Right | KeyCode::Char('l'));
                if self.selected >= n {
                    self.selected = n + (self.selected - n + if right { 1 } else { 2 }) % 3;
                } else {
                    self.switch_tab(right);
                }
                self.error = None;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1) % total
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                self.selected = (self.selected + total - 1) % total
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.selected < n {
                    let field = self.fields()[self.selected];
                    if matches!(field.kind, Kind::Toggle | Kind::Inverse) {
                        let text = (!self.json()[field.key].as_bool().unwrap_or(false)).to_string();
                        if let Err(e) = self.commit_edit(text) {
                            self.error = Some(format!("{e:#}"));
                        }
                    } else {
                        self.edit = Some(TextEdit::new(self.display(field, true)));
                    }
                } else {
                    match self.selected - n - extra {
                        0 => return self.apply(input, false),
                        1 => return self.apply(input, true),
                        _ => return Action::Cancel,
                    }
                }
            }
            _ => {}
        }
        Action::None
    }
    pub fn render(&self, f: &mut Frame<'_>) {
        let outer = f.area();
        let width = outer.width.saturating_sub(4).min(92);
        let height = outer.height.saturating_sub(2).min(30);
        let area = Rect::new(
            outer.x + (outer.width - width) / 2,
            outer.y + (outer.height - height) / 2,
            width,
            height,
        );
        let bg = Color::Rgb(24, 29, 37);
        let muted = Color::Rgb(135, 148, 167);
        let accent = Color::Rgb(111, 225, 208);
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Compilation ")
            .border_style(Style::default().fg(accent))
            .style(Style::default().bg(bg));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let parts = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(inner);
        let mut tabs = Vec::new();
        for (index, name) in ["Typst", "Document inputs", "LeafView"].iter().enumerate() {
            tabs.push(Span::styled(
                format!("  {name}  "),
                if self.tab == index {
                    Style::default()
                        .fg(bg)
                        .bg(accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(muted)
                },
            ));
            tabs.push(Span::raw(" "));
        }
        let tabs = Line::from(tabs);
        f.render_widget(Paragraph::new(tabs), parts[0]);
        let name = self
            .draft
            .input
            .file_name()
            .unwrap_or(self.draft.input.as_os_str())
            .to_string_lossy();
        f.render_widget(
            Paragraph::new(Line::styled(
                format!("  {}", crate::links::sanitize(&name)),
                Style::default().fg(muted),
            )),
            parts[1],
        );
        let fields = self.fields();
        let n = fields.len();
        let rows: Vec<(String, String)> = fields
            .iter()
            .map(|field| (field.label.into(), self.display(*field, false)))
            .collect();
        let available = parts[2].height.max(1) as usize;
        let start = self
            .selected
            .min(rows.len().saturating_sub(1))
            .saturating_sub(available - 1);
        let lines: Vec<Line<'_>> = rows
            .iter()
            .enumerate()
            .skip(start)
            .take(available)
            .map(|(i, (label, value))| {
                let selected = i == self.selected;
                let value = if selected {
                    self.edit
                        .as_ref()
                        .map(|s| s.visible(usize::from(width.saturating_sub(32))))
                        .unwrap_or_else(|| value.clone())
                } else {
                    value.clone()
                };
                Line::styled(
                    format!(
                        " {} {:<24} {}",
                        if selected { "›" } else { " " },
                        label,
                        crate::links::sanitize(&value)
                    ),
                    if selected {
                        Style::default()
                            .fg(accent)
                            .bg(Color::Rgb(35, 53, 64))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(217, 224, 234))
                    },
                )
            })
            .collect();
        f.render_widget(Paragraph::new(lines), parts[2]);
        let help = self.error.clone().unwrap_or_else(|| {
            if self.tab == 1 {
                String::new()
            } else if self.selected < n {
                fields[self.selected].help.into()
            } else {
                "Apply keeps changes in memory; Save remembers them for this file only.".into()
            }
        });
        f.render_widget(
            Paragraph::new(crate::links::sanitize(&help))
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(if self.error.is_some() {
                    Color::LightRed
                } else {
                    muted
                })),
            parts[3],
        );
        if let Some(editor) = &self.inputs {
            editor.render(f, parts[2]);
            if let Some(error) = &editor.error {
                f.render_widget(
                    Paragraph::new(error.as_str()).style(Style::default().fg(Color::LightRed)),
                    parts[3],
                );
            }
        }
        let first_action = if self.tab == 1 { 0 } else { n };
        let button_width = (parts[4].width / 3).min(20);
        let row = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(button_width),
            Constraint::Length(button_width),
            Constraint::Length(button_width),
        ])
        .split(parts[4]);
        for (index, (label, caption)) in [
            ("Apply", "Session · Ctrl+A"),
            ("Save", "This file · Ctrl+S"),
            ("Cancel", "Discard · Esc"),
        ]
        .iter()
        .enumerate()
        {
            let style =
                if (self.tab != 1 || self.input_actions) && self.selected == first_action + index {
                    Style::default()
                        .fg(bg)
                        .bg(accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Rgb(217, 224, 234))
                        .bg(Color::Rgb(35, 45, 57))
                };
            f.render_widget(
                Paragraph::new(vec![
                    Line::styled(format!(" {label} "), style),
                    Line::styled(*caption, Style::default().fg(muted)),
                ])
                .alignment(ratatui::layout::Alignment::Center),
                row[index + 1],
            );
        }
    }
}
#[derive(Clone)]
struct TextEdit {
    text: String,
    cursor: usize,
}
impl TextEdit {
    fn new(text: String) -> Self {
        let cursor = text.len();
        Self { text, cursor }
    }
    fn previous(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }
    fn next(&self) -> usize {
        self.text[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |c| self.cursor + c.len_utf8())
    }
    fn key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('u') {
            self.text.clear();
            self.cursor = 0;
            return;
        }
        match key.code {
            KeyCode::Left => self.cursor = self.previous(),
            KeyCode::Right => self.cursor = self.next(),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.text.len(),
            KeyCode::Backspace if self.cursor > 0 => {
                let p = self.previous();
                self.text.replace_range(p..self.cursor, "");
                self.cursor = p;
            }
            KeyCode::Delete if self.cursor < self.text.len() => {
                let n = self.next();
                self.text.replace_range(self.cursor..n, "");
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }
            _ => {}
        }
    }
    fn visible(&self, width: usize) -> String {
        let before: Vec<char> = self.text[..self.cursor].chars().collect();
        let keep = width.saturating_sub(3).max(1);
        let skip = before.len().saturating_sub(keep);
        format!(
            "{}{}▏{}",
            if skip > 0 { "…" } else { "" },
            before[skip..].iter().collect::<String>(),
            self.text[self.cursor..]
                .chars()
                .take(width.saturating_sub(before.len() - skip + 1))
                .collect::<String>()
        )
    }
}
struct InputsEditor {
    rows: Vec<(String, String)>,
    row: usize,
    column: usize,
    edit: Option<TextEdit>,
    error: Option<String>,
}
impl InputsEditor {
    fn new(values: &[(String, String)]) -> Self {
        Self {
            rows: values.to_vec(),
            row: 0,
            column: 0,
            edit: None,
            error: None,
        }
    }
    fn commit(&mut self) {
        if let Some(edit) = self.edit.take() {
            if let Some(pair) = self.rows.get_mut(self.row) {
                if self.column == 0 {
                    pair.0 = edit.text;
                } else {
                    pair.1 = edit.text;
                }
            }
        }
    }
    fn finish(&mut self) -> Result<Vec<(String, String)>> {
        self.commit();
        let mut keys = std::collections::BTreeSet::new();
        for (key, _) in &self.rows {
            anyhow::ensure!(!key.trim().is_empty(), "Every input needs a key");
            anyhow::ensure!(keys.insert(key.clone()), "Duplicate input key: {key}");
        }
        let options = crate::typst_options::Options {
            inputs: self.rows.clone(),
            ..Default::default()
        };
        crate::compile_config::validate_values(&options)?;
        Ok(self.rows.clone())
    }
    fn key(&mut self, key: KeyEvent) {
        self.error = None;
        if self.edit.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.edit = None;
                }
                KeyCode::Enter => self.commit(),
                KeyCode::Tab | KeyCode::BackTab => {
                    self.commit();
                    self.column = 1 - self.column;
                }
                _ => self.edit.as_mut().unwrap().key(key),
            }
            return;
        }
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.row = (self.row + 1).min(self.rows.len().saturating_sub(1))
            }
            KeyCode::Up | KeyCode::Char('k') => self.row = self.row.saturating_sub(1),
            KeyCode::Left => self.column = 0,
            KeyCode::Right => self.column = 1,
            KeyCode::Tab | KeyCode::BackTab => self.column = 1 - self.column,
            KeyCode::Char('n') | KeyCode::Insert => {
                self.rows.push((String::new(), String::new()));
                self.row = self.rows.len() - 1;
                self.column = 0;
                self.edit = Some(TextEdit::new(String::new()));
            }
            KeyCode::Delete => {
                if !self.rows.is_empty() {
                    self.rows.remove(self.row);
                    self.row = self.row.min(self.rows.len().saturating_sub(1));
                }
            }
            KeyCode::Enter => {
                if self.rows.is_empty() {
                    self.key(KeyEvent::from(KeyCode::Insert));
                } else {
                    let pair = &self.rows[self.row];
                    self.edit = Some(TextEdit::new(if self.column == 0 {
                        pair.0.clone()
                    } else {
                        pair.1.clone()
                    }));
                }
            }
            _ => {}
        }
    }
    fn render(&self, f: &mut Frame<'_>, area: Rect) {
        f.render_widget(Clear, area);
        let inner = area;
        f.render_widget(
            Block::default().style(
                Style::default()
                    .bg(Color::Rgb(24, 29, 37))
                    .fg(Color::Rgb(217, 224, 234)),
            ),
            area,
        );
        let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);
        f.render_widget(
            Paragraph::new("   KEY                          VALUE")
                .style(Style::default().fg(Color::DarkGray)),
            parts[0],
        );
        let available = parts[1].height.max(1) as usize;
        let start = self.row.saturating_sub(available - 1);
        let lines: Vec<Line<'_>> = if self.rows.is_empty() {
            vec![Line::from(
                "   No inputs yet. Press n to add a key/value pair.",
            )]
        } else {
            self.rows
                .iter()
                .enumerate()
                .skip(start)
                .take(available)
                .map(|(i, (key, value))| {
                    let style = |column| {
                        if i == self.row && self.column == column {
                            Style::default()
                                .fg(Color::LightCyan)
                                .bg(Color::Rgb(35, 53, 64))
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        }
                    };
                    let cell = |column, text: &str, width| {
                        if i == self.row && self.column == column {
                            self.edit
                                .as_ref()
                                .map(|e| e.visible(width))
                                .unwrap_or_else(|| text.chars().take(width).collect())
                        } else {
                            text.chars().take(width).collect()
                        }
                    };
                    Line::from(vec![
                        Span::raw(if i == self.row { " › " } else { "   " }),
                        Span::styled(
                            format!("{:<28}", crate::links::sanitize(&cell(0, key, 28))),
                            style(0),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            crate::links::sanitize(&cell(
                                1,
                                value,
                                inner.width.saturating_sub(34) as usize,
                            )),
                            style(1),
                        ),
                    ])
                })
                .collect()
        };
        f.render_widget(Paragraph::new(lines), parts[1]);
    }
}
// Keep value presentation explicit without a dependency for a one-off transform.
trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}
#[cfg(test)]
mod tests {
    use super::*;
    fn session(d: &Path) -> Session {
        Session::from_paths(
            d.join("a.typ"),
            d.join("global.json"),
            d.join(".leafview.json"),
            Flags::default(),
        )
    }
    #[test]
    fn typed_controls_and_tabs() {
        let d = tempfile::tempdir().unwrap();
        let s = session(d.path());
        let mut p = Settings::new(&s);
        assert_eq!(p.fields().len(), 14);
        p.selected = 2;
        assert!(p.commit_edit("0".into()).is_err());
        p.commit_edit("2".into()).unwrap();
        p.selected = 0;
        p.commit_edit(".".into()).unwrap();
        assert_eq!(p.value.options.root, Some(".".into()));
        p.key(KeyEvent::from(KeyCode::Right), &s.input);
        assert_eq!(p.tab, 1);
        assert!(p.inputs.is_some());
        p.key(KeyEvent::from(KeyCode::Down), &s.input);
        assert!(p.input_actions);
        p.key(KeyEvent::from(KeyCode::Char('l')), &s.input);
        assert_eq!(p.selected, 1);
        p.key(KeyEvent::from(KeyCode::Up), &s.input);
        assert!(!p.input_actions);
        p.key(KeyEvent::from(KeyCode::Right), &s.input);
        assert_eq!(p.fields().len(), 3);
        assert_eq!(p.tab, 2);
        let before = p.value.auto_compile;
        p.key(KeyEvent::from(KeyCode::Enter), &s.input);
        assert_ne!(p.value.auto_compile, before);
    }
    #[test]
    fn cursor_and_input_cells_preserve_unicode_and_validate_keys() {
        let mut text = TextEdit::new("абв".into());
        text.key(KeyEvent::from(KeyCode::Left));
        text.key(KeyEvent::from(KeyCode::Char('X')));
        assert_eq!(text.text, "абXв");
        text.key(KeyEvent::from(KeyCode::Backspace));
        text.key(KeyEvent::from(KeyCode::Delete));
        assert_eq!(text.text, "аб");
        text.key(KeyEvent::from(KeyCode::Home));
        text.key(KeyEvent::from(KeyCode::Char('Я')));
        assert_eq!(text.text, "Яаб");
        let mut editor = InputsEditor::new(&[]);
        editor.key(KeyEvent::from(KeyCode::Char('n')));
        for c in "mode".chars() {
            editor.key(KeyEvent::from(KeyCode::Char(c)));
        }
        editor.key(KeyEvent::from(KeyCode::Tab));
        editor.key(KeyEvent::from(KeyCode::Enter));
        for c in "teacher".chars() {
            editor.key(KeyEvent::from(KeyCode::Char(c)));
        }
        assert_eq!(
            editor.finish().unwrap(),
            vec![("mode".into(), "teacher".into())]
        );
        editor.rows.push(("mode".into(), "duplicate".into()));
        assert!(editor.finish().is_err());
        editor.key(KeyEvent::from(KeyCode::Delete));
        assert_eq!(editor.rows.len(), 1);
    }
    #[test]
    fn action_row_uses_directional_keys() {
        let d = tempfile::tempdir().unwrap();
        let s = session(d.path());
        let mut p = Settings::new(&s);
        let n = p.fields().len();
        p.selected = n - 1;
        p.key(KeyEvent::from(KeyCode::Char('j')), &s.input);
        assert_eq!(p.selected, n);
        p.key(KeyEvent::from(KeyCode::Char('l')), &s.input);
        assert_eq!(p.selected, n + 1);
        assert_eq!(p.tab, 0);
        p.key(KeyEvent::from(KeyCode::Right), &s.input);
        assert_eq!(p.selected, n + 2);
        p.key(KeyEvent::from(KeyCode::Char('h')), &s.input);
        assert_eq!(p.selected, n + 1);
        p.key(KeyEvent::from(KeyCode::Char('k')), &s.input);
        assert_eq!(p.selected, n);
    }
    #[test]
    fn render_tabs() {
        let d = tempfile::tempdir().unwrap();
        let p = Settings::new(&session(d.path()));
        let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        t.draw(|f| p.render(f)).unwrap();
        let b = t.backend().buffer();
        let text = (0..30)
            .map(|y| (0..100).map(|x| b[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("Typst") && text.contains("LeafView") && text.contains("Document inputs")
        );
        assert!(text.contains("CA certificate"));
        if let Ok(path) = std::env::var("LEAFVIEW_SETTINGS_SNAPSHOT") {
            std::fs::write(&path, text).unwrap();
            let mut p = p;
            p.switch_tab(true);
            t.draw(|f| p.render(f)).unwrap();
            let b = t.backend().buffer();
            let text = (0..30)
                .map(|y| (0..100).map(|x| b[(x, y)].symbol()).collect::<String>())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!text.contains("Custom inputs") && !text.contains("F2 accept"));
            assert!(text.contains("Ctrl+S"));
            std::fs::write(format!("{path}.inputs"), text).unwrap();
        }
    }
}
