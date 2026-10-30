//! Completion policy and UI state. Defaults mirror AstroNvim 5 + blink.cmp 1.10.2.
use super::Editor;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub debounce_ms: u64,
    pub menu_height: u16,
    pub menu_min_width: u16,
    pub menu_max_width: u16,
    pub documentation_width: u16,
    pub documentation_height: u16,
    pub documentation_auto_show: bool,
    pub signature_auto_show: bool,
    pub signature_delay_ms: u64,
    pub ghost_text: bool,
    pub auto_insert: bool,
    pub preselect: bool,
    pub auto_brackets: bool,
    pub auto_pairs: bool,
    pub sources: Vec<String>,
    /// Filetype -> trigger -> LSP snippet body (Lua code is deliberately not executed).
    pub snippets: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    pub background: [u8; 3],
    pub foreground: [u8; 3],
    pub selected_background: [u8; 3],
    pub border_color: [u8; 3],
    pub rounded_border: bool,
    pub keys: std::collections::BTreeMap<String, Vec<String>>,
}
impl Default for Config {
    fn default() -> Self {
        let mut keys = std::collections::BTreeMap::new();
        for (action, bindings) in [
            ("next", vec!["Down", "Tab", "C-n", "C-j"]),
            ("previous", vec!["Up", "S-Tab", "C-p", "C-k"]),
            ("accept", vec!["Enter"]),
            ("hide", vec!["C-e"]),
            ("show", vec!["C-Space"]),
            ("documentation_up", vec!["C-u", "C-b"]),
            ("documentation_down", vec!["C-d", "C-f"]),
            ("snippet_forward", vec!["Tab"]),
            ("snippet_backward", vec!["S-Tab"]),
            ("window_next", vec!["F7"]),
            ("window_close", vec!["F8"]),
        ] {
            keys.insert(
                action.into(),
                bindings.into_iter().map(str::to_owned).collect(),
            );
        }
        Self {
            debounce_ms: 50,
            menu_height: 10,
            menu_min_width: 15,
            menu_max_width: 70,
            documentation_width: 80,
            documentation_height: 20,
            documentation_auto_show: true,
            signature_auto_show: true,
            signature_delay_ms: 80,
            ghost_text: false,
            auto_insert: false,
            preselect: false,
            auto_brackets: true,
            auto_pairs: true,
            snippets: Default::default(),
            sources: vec![
                "lsp".into(),
                "path".into(),
                "snippets".into(),
                "buffer".into(),
            ],
            background: [20, 22, 27],
            foreground: [155, 160, 169],
            selected_background: [38, 52, 63],
            border_color: [58, 62, 71],
            rounded_border: true,
            keys,
        }
    }
}
impl Config {
    pub fn load() -> Result<Self, String> {
        let path = std::env::var_os("LEAFVIEW_COMPLETION_CONFIG")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("XDG_CONFIG_HOME")
                    .map(std::path::PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(|h| std::path::PathBuf::from(h).join(".config"))
                    })
                    .map(|p| p.join("leafview/completion.json"))
            });
        let Some(path) = path else {
            return Ok(Self::default());
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }
    pub(super) fn matches(&self, action: &str, key: KeyEvent) -> bool {
        let name = match (key.code, key.modifiers) {
            (KeyCode::Char(' ' | '@'), m) if m.contains(KeyModifiers::CONTROL) => "C-Space".into(),
            (KeyCode::Char(c), m) if m.contains(KeyModifiers::CONTROL) => {
                format!("C-{}", c.to_ascii_lowercase())
            }
            (KeyCode::Tab, _) => "Tab".into(),
            (KeyCode::BackTab, _) => "S-Tab".into(),
            (KeyCode::Enter, _) => "Enter".into(),
            (KeyCode::Esc, _) => "Esc".into(),
            (KeyCode::F(n), _) => format!("F{n}"),
            (KeyCode::Up, _) => "Up".into(),
            (KeyCode::Down, _) => "Down".into(),
            _ => return false,
        };
        self.keys
            .get(action)
            .is_some_and(|keys| keys.contains(&name))
    }
}
#[derive(Default)]
pub(super) struct State {
    pub selected: bool,
    pub documentation_hidden: bool,
    pub documentation_scroll: u16,
    pub resolve_due: bool,
    pub resolved: bool,
    pub accepting: bool,
    pub preview: Option<(usize, String, usize, usize)>,
    pub resolving_item: Option<Value>,
}
impl Editor {
    pub fn owns_completion_key(&self, key: KeyEvent) -> bool {
        self.completion_visible()
            && [
                "next",
                "previous",
                "accept",
                "hide",
                "show",
                "documentation_up",
                "documentation_down",
            ]
            .iter()
            .any(|action| self.completion_config.matches(action, key))
    }
    pub(super) fn completion_visible(&self) -> bool {
        self.popup
            .as_ref()
            .is_some_and(|(m, _, _)| m == "textDocument/completion")
    }
    pub(super) fn restore_preview(&mut self) {
        if let Some((start, old, len, cursor)) = self.cmp.preview.take() {
            if let Some(b) = self.buffers.get_mut(self.current) {
                b.edit(start, start + len, &old);
                b.set_cursor(cursor);
            }
        }
    }
    pub(super) fn completion_key(&mut self, key: KeyEvent) -> bool {
        let visible = self.completion_visible();
        if visible {
            for (action, delta) in [("next", 1), ("previous", -1)] {
                if self.completion_config.matches(action, key) {
                    self.restore_preview();
                    if self.cmp.selected {
                        self.move_popup(delta);
                    } else if let Some((_, items, i)) = self.popup.as_mut() {
                        *i = if delta > 0 { 0 } else { items.len() - 1 };
                    }
                    self.cmp.accepting = false;
                    self.cmp.selected = true;
                    self.cmp.documentation_scroll = 0;
                    self.cmp.documentation_hidden = !self.completion_config.documentation_auto_show;
                    self.cmp.resolve_due = true;
                    self.cmp.resolved = false;
                    self.preview_completion();
                    return true;
                }
            }
            if self.completion_config.matches("accept", key) && self.cmp.selected {
                self.restore_preview();
                if !self.cmp.resolved
                    && self
                        .popup
                        .as_ref()
                        .is_some_and(|(_, v, i)| v[*i].get("data").is_some())
                {
                    self.cmp.accepting = true;
                    self.cmp.resolve_due = true;
                } else {
                    self.accept_popup();
                }
                return true;
            }
            if self.completion_config.matches("hide", key) || key.code == KeyCode::Esc {
                self.restore_preview();
                self.popup = None;
                self.popup_text.clear();
                self.cmp = State::default();
                // Escape is not a blink mapping: it also leaves Insert mode.
                return key.code != KeyCode::Esc;
            }
            for (action, down) in [("documentation_up", false), ("documentation_down", true)] {
                if self.completion_config.matches(action, key)
                    && self.cmp.selected
                    && !self.cmp.documentation_hidden
                {
                    self.cmp.documentation_scroll = if down {
                        self.cmp.documentation_scroll.saturating_add(4).min(1000)
                    } else {
                        self.cmp.documentation_scroll.saturating_sub(4)
                    };
                    return true;
                }
            }
        }
        if self.vim.mode == super::vim::Mode::Insert {
            if self.completion_config.matches("show", key) {
                if visible && self.cmp.selected {
                    self.cmp.documentation_hidden = !self.cmp.documentation_hidden;
                } else if !visible {
                    self.request_lsp("complete", false);
                }
                return true;
            }
            if !visible {
                for (action, backwards) in [("snippet_forward", false), ("snippet_backward", true)]
                {
                    if self.completion_config.matches(action, key) && self.jump_snippet(backwards) {
                        return true;
                    }
                }
                if matches!(key.code, KeyCode::Char('n' | 'p'))
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.code == KeyCode::Tab
                        && self.buffers.get(self.current).is_some_and(|b| {
                            b.cursor() > 0 && !b.state.doc.char(b.cursor() - 1).is_whitespace()
                        })
                {
                    self.request_lsp("complete", false);
                    return true;
                }
            }
        }
        if visible {
            // A typed character commits the preview, just as blink auto_insert does.
            if matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace)
                && !key.modifiers.contains(KeyModifiers::CONTROL)
            {
                self.cmp.preview = None;
            } else {
                self.restore_preview();
            }
            self.cmp.accepting = false;
            self.cmp.resolve_due = false;
        }
        false
    }
    fn preview_completion(&mut self) {
        if !self.completion_config.auto_insert {
            return;
        }
        let Some((_, items, selected)) = &self.popup else {
            return;
        };
        let item = &items[*selected];
        let Some(b) = self.buffers.get_mut(self.current) else {
            return;
        };
        let (start, end, text) = edit(item, b);
        let text = if item["insertTextFormat"] == 2 {
            expand(&text, start).0
        } else {
            text
        };
        let old = b.state.doc.slice(start..end).to_string();
        let cursor = b.cursor();
        let len = text.chars().count();
        b.edit(start, end, &text);
        b.set_cursor(start + len);
        self.cmp.preview = Some((start, old, len, cursor));
    }
    pub(super) fn resolve_completion(&mut self) {
        if self.lsp_pending.is_some() || !self.cmp.resolve_due || !self.completion_visible() {
            return;
        }
        self.restore_preview();
        self.cmp.resolve_due = false;
        let (_, items, i) = self.popup.as_ref().unwrap();
        if items[*i].get("data").is_none() {
            self.cmp.resolved = true;
            if self.cmp.accepting {
                self.cmp.accepting = false;
                self.accept_popup();
            } else {
                self.preview_completion();
            }
            return;
        }
        self.cmp.resolving_item = Some(items[*i].clone());
        self.request_lsp(&format!("resolve:{}", items[*i]), true);
    }
    pub(super) fn resolved_completion(&mut self, result: &Value) {
        if !self.completion_visible() || self.cmp.resolve_due {
            return;
        }
        if let Some(expected) = self.cmp.resolving_item.take() {
            if !self
                .popup
                .as_ref()
                .is_some_and(|(_, items, i)| items[*i] == expected)
            {
                return;
            }
        }
        if let Some((_, items, i)) = &mut self.popup {
            if let Some(fields) = result.as_object() {
                for (k, v) in fields {
                    items[*i][k] = v.clone();
                }
            }
        }
        self.cmp.resolved = true;
        if self.cmp.accepting {
            self.cmp.accepting = false;
            self.accept_popup();
        } else {
            self.preview_completion();
        }
    }
    fn jump_snippet(&mut self, backwards: bool) -> bool {
        let Some(b) = self.buffers.get_mut(self.current) else {
            return false;
        };
        let Some(mut snippet) = b.snippet.take() else {
            return false;
        };
        if !snippet.is_valid(&b.state.selection) {
            return false;
        }
        let (selection, last) = if backwards {
            let Some(s) = snippet.prev_tabstop(&b.state.selection) else {
                b.snippet = Some(snippet);
                return false;
            };
            (s, false)
        } else {
            snippet.next_tabstop(&b.state.selection)
        };
        b.state.selection = selection;
        if !last {
            b.snippet = Some(snippet);
        }
        true
    }
}
pub(super) fn edit(item: &Value, b: &super::Buffer) -> (usize, usize, String) {
    let edit = &item["textEdit"];
    let text = edit["newText"]
        .as_str()
        .or_else(|| item["insertText"].as_str())
        .or_else(|| item["label"].as_str())
        .unwrap_or("")
        .to_owned();
    let end = b.cursor();
    let mut start = end;
    while start > b.start(b.line())
        && (b.state.doc.char(start - 1).is_alphanumeric() || b.state.doc.char(start - 1) == '_')
    {
        start -= 1;
    }
    if let Some(range) = edit
        .get("range")
        .or_else(|| edit.get("insert"))
        .or_else(|| edit.get("replace"))
    {
        start = super::lsp::value_position(&b.state.doc, &range["start"]);
        return (
            start,
            super::lsp::value_position(&b.state.doc, &range["end"]),
            text,
        );
    }
    (start, end, text)
}
pub(super) fn expand(
    text: &str,
    start: usize,
) -> (String, Option<helix_core::snippets::RenderedSnippet>) {
    use helix_core::{
        indent::IndentStyle,
        snippets::{Snippet, SnippetRenderCtx},
        Rope,
    };
    // Helix renders choice nodes empty unless the host supplies a choice UI.
    // Use the first choice as LuaSnip does on expansion; preserve tab navigation.
    let choices = regex::Regex::new(r"\$\{(\d+)\|((?:\\.|[^}])*)\|\}").unwrap();
    let normalized = choices.replace_all(text, |caps: &regex::Captures<'_>| {
        let first = caps[2].split(',').next().unwrap_or("");
        format!("${{{}:{}}}", &caps[1], first)
    });
    let Ok(snippet) = Snippet::parse(&normalized) else {
        return (text.into(), None);
    };
    let mut rendered = snippet.prepare_render();
    let mut ctx = SnippetRenderCtx {
        resolve_var: Box::new(|_| None),
        tab_width: 4,
        indent_style: IndentStyle::Spaces(4),
        line_ending: "\n",
    };
    let (text, _) = snippet.render_at(&mut rendered, Rope::new().slice(..), false, &mut ctx, start);
    (text.to_string(), Some(rendered))
}
pub(super) fn documentation(item: &Value) -> String {
    fn content(v: &Value) -> String {
        if let Some(s) = v.as_str() {
            s.into()
        } else if let Some(a) = v.as_array() {
            a.iter().map(content).collect::<Vec<_>>().join("\n\n")
        } else {
            v["value"].as_str().unwrap_or("").into()
        }
    }
    let detail = item["detail"].as_str().unwrap_or("");
    let doc = content(&item["documentation"]);
    if detail.is_empty() {
        doc
    } else {
        format!("```\n{detail}\n```\n\n{doc}")
    }
}
pub(super) fn kind(kind: u64) -> (&'static str, &'static str) {
    match kind {
        2 => ("", "Method"),
        3 => ("", "Function"),
        4 => ("", "Constructor"),
        5 => ("", "Field"),
        6 => ("", "Variable"),
        7 => ("", "Class"),
        8 => ("", "Interface"),
        9 => ("", "Module"),
        10 => ("", "Property"),
        11 => ("", "Unit"),
        12 => ("", "Value"),
        13 => ("", "Enum"),
        14 => ("", "Keyword"),
        15 => ("", "Snippet"),
        16 => ("", "Color"),
        17 => ("", "File"),
        18 => ("", "Reference"),
        19 => ("", "Folder"),
        20 => ("", "EnumMember"),
        21 => ("", "Constant"),
        22 => ("", "Struct"),
        23 => ("", "Event"),
        24 => ("", "Operator"),
        25 => ("", "TypeParameter"),
        _ => ("", "Text"),
    }
}
