//! Independent signature state and bounded call-context tracking.
use super::Editor;
use helix_core::Rope;
use serde_json::Value;
use std::path::PathBuf;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Call {
    pub open: usize,
    pub argument: usize,
    pub name: Option<String>,
}
#[derive(Default)]
pub(super) struct State {
    pub value: Value,
    pub path: PathBuf,
    pub call: Option<Call>,
}
// Scan forward to correctly ignore strings, comments and commas in nested values.
// Never scan an entire large document on a keystroke.
pub(super) fn context_for(text: &Rope, cursor: usize, python: bool) -> Option<Call> {
    let start = cursor.saturating_sub(16384);
    let chars: Vec<_> = text.slice(start..cursor).chars().collect();
    let mut stack: Vec<(char, usize, usize, usize)> = vec![];
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;
    let mut block = 0usize;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if comment {
            if c == '\n' {
                comment = false;
            }
            i += 1;
            continue;
        }
        if block > 0 {
            if c == '/' && next == Some('*') {
                block += 1;
                i += 2;
            } else if c == '*' && next == Some('/') {
                block -= 1;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if c == '/' && next == Some('/') {
            comment = true;
            i += 2;
            continue;
        }
        if c == '/' && next == Some('*') {
            block = 1;
            i += 2;
            continue;
        }
        if python && c == '#' {
            comment = true;
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
            i += 1;
            continue;
        }
        match c {
            '(' | '[' | '{' => stack.push((c, start + i, 0, start + i + 1)),
            ')' | ']' | '}' => {
                stack.pop();
            }
            ',' => {
                if let Some(top) = stack.last_mut() {
                    top.2 += 1;
                    top.3 = start + i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let (_, open, argument, arg_start) =
        stack.iter().rev().find(|(c, _, _, _)| *c == '(').copied()?;
    // Parentheses following identifiers/closing expressions are calls, not tuples.
    let mut before = open;
    while before > 0 && text.char(before - 1).is_whitespace() {
        before -= 1;
    }
    if before == 0
        || !(text.char(before - 1).is_alphanumeric()
            || matches!(text.char(before - 1), '_' | ')' | ']'))
    {
        return None;
    }
    let arg = text.slice(arg_start..cursor).to_string();
    let name = arg
        .split_once(':')
        .or_else(|| arg.split_once('='))
        .and_then(|(name, _)| {
            let name = name.trim();
            (!name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-'))
            .then(|| name.to_owned())
        });
    Some(Call {
        open,
        argument,
        name,
    })
}
impl Editor {
    pub(super) fn insert_pair(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        if self.vim.mode != super::vim::Mode::Insert
            || !self.completion_config.auto_pairs
            || key.modifiers.contains(KeyModifiers::CONTROL)
        {
            return false;
        }
        let Some(b) = self.buffers.get_mut(self.current) else {
            return false;
        };
        let p = b.cursor();
        let next = if p < b.state.doc.len_chars() {
            Some(b.state.doc.char(p))
        } else {
            None
        };
        if let KeyCode::Char(c @ (')' | ']' | '}')) = key.code {
            if next == Some(c) {
                b.set_cursor(p + 1);
                return true;
            }
        }
        if let KeyCode::Char(c @ ('(' | '[' | '{')) = key.code {
            // Do not pair inside strings or comments (use existing lexical cache).
            let row = b.line();
            let col = p - b.start(row);
            let line = b.state.doc.line(row);
            let tokens = b.syntax.line(&b.path, b.version, row, line, col + 1);
            let token = tokens
                .get(col)
                .or_else(|| col.checked_sub(1).and_then(|i| tokens.get(i)));
            if token.is_some_and(|t| {
                matches!(
                    t,
                    super::syntax::Token::Comment | super::syntax::Token::String
                )
            }) {
                return false;
            }
            if next.is_none_or(|ch| ch.is_whitespace() || matches!(ch, ')' | ']' | '}' | ',' | ';'))
            {
                let close = match c {
                    '(' => ')',
                    '[' => ']',
                    _ => '}',
                };
                b.edit(p, p, &format!("{c}{close}"));
                b.set_cursor(p + 1);
                return true;
            }
        }
        if key.code == KeyCode::Backspace && p > 0 {
            let prev = b.state.doc.char(p - 1);
            if matches!(
                (prev, next),
                ('(', Some(')')) | ('[', Some(']')) | ('{', Some('}'))
            ) {
                b.edit(p - 1, p + 1, "");
                b.set_cursor(p - 1);
                return true;
            }
        }
        false
    }
    pub(super) fn schedule_signature(&mut self) {
        if !self.completion_config.signature_auto_show || self.vim.mode != super::vim::Mode::Insert
        {
            self.signature = State::default();
            self.signature_due = None;
            return;
        }
        let Some(b) = self.buffers.get(self.current) else {
            return;
        };
        let call = context_for(
            &b.state.doc,
            b.cursor(),
            b.path.extension().is_some_and(|s| s == "py"),
        );
        if call.is_none() {
            self.signature = State::default();
            self.signature_due = None;
            return;
        }
        if self.signature.path != b.path
            || self.signature.call.as_ref().map(|c| c.open) != call.as_ref().map(|c| c.open)
        {
            self.signature = State::default();
            self.windows.signature_hidden = false;
            self.windows.signature_scroll = 0;
        }
        if self.windows.signature_hidden {
            return;
        }
        self.signature_due = Some(
            std::time::Instant::now()
                + std::time::Duration::from_millis(self.completion_config.signature_delay_ms),
        );
    }
    pub(super) fn receive_signature(&mut self, value: &Value) {
        let Some(b) = self.buffers.get(self.current) else {
            return;
        };
        self.signature = State {
            value: value.clone(),
            path: b.path.clone(),
            call: context_for(
                &b.state.doc,
                b.cursor(),
                b.path.extension().is_some_and(|s| s == "py"),
            ),
        };
    }
}
/// LSP parameter labels use UTF-16 offsets, not bytes or Unicode scalar counts.
pub(super) fn active_range(
    value: &Value,
    call: &Call,
) -> Option<(String, std::ops::Range<usize>, String)> {
    let signatures = value["signatures"].as_array()?;
    let index = value["activeSignature"].as_u64().unwrap_or(0) as usize;
    let sig = signatures.get(index).or_else(|| signatures.first())?;
    let label = sig["label"].as_str()?.to_owned();
    let parameters = sig["parameters"].as_array();
    let named = call.name.as_ref().and_then(|name| {
        parameters?.iter().position(|p| {
            p["label"]
                .as_str()
                .is_some_and(|s| s.split([':', '=']).next().unwrap_or("").trim() == name)
        })
    });
    let index = named
        .or_else(|| {
            sig["activeParameter"]
                .as_u64()
                .or_else(|| value["activeParameter"].as_u64())
                .map(|n| n as usize)
        })
        .unwrap_or(call.argument);
    let parameter = parameters.and_then(|p| p.get(index));
    let range = if let Some(p) = parameter {
        if let Some(a) = p["label"].as_array() {
            let start = a.first()?.as_u64()? as usize;
            let end = a.get(1)?.as_u64()? as usize;
            let byte_at = |offset: usize| {
                let mut units = 0;
                for (byte, ch) in label.char_indices() {
                    if units >= offset {
                        return byte;
                    }
                    units += ch.len_utf16();
                }
                label.len()
            };
            let start = byte_at(start);
            start..byte_at(end).max(start)
        } else if let Some(text) = p["label"].as_str() {
            // Restrict search to the argument list, not the function's name.
            let base = label.find('(').map_or(0, |i| i + 1);
            let start = label[base..].find(text).map_or(base, |i| base + i);
            start..(start + text.len()).min(label.len())
        } else {
            0..0
        }
    } else {
        0..0
    };
    let mut doc = super::completion::documentation(sig);
    if let Some(p) = parameter {
        let pdoc = super::completion::documentation(p);
        if !pdoc.is_empty() {
            doc = format!("{pdoc}\n\n{doc}");
        }
    }
    Some((label, range, doc))
}
