//! Bounded local providers; no directory IO on the render/input thread.
use super::{completion::Config, Buffer};
use serde_json::{json, Value};
pub(super) fn local(b: &Buffer, config: &Config) -> Vec<Value> {
    let end = b.cursor();
    let mut start = end;
    while start > b.start(b.line())
        && (b.state.doc.char(start - 1).is_alphanumeric() || b.state.doc.char(start - 1) == '_')
    {
        start -= 1;
    }
    let prefix = b.state.doc.slice(start..end).to_string();
    let mut out = vec![];
    if config.sources.iter().any(|s| s == "buffer") && !prefix.is_empty() {
        let mut seen = std::collections::BTreeSet::new();
        for row in b.line().saturating_sub(100)..(b.line() + 101).min(b.state.doc.len_lines()) {
            let text: String = b.state.doc.line(row).chars().take(4096).collect();
            for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if word.len() > prefix.len()
                    && word.starts_with(&prefix)
                    && seen.insert(word.to_owned())
                {
                    out.push(
                        json!({"label":word,"kind":1,"labelDetails":{"description":"Buffer"}}),
                    );
                    if out.len() >= 200 {
                        return out;
                    }
                }
            }
        }
    }
    if config.sources.iter().any(|s| s == "snippets") {
        let ext = b.path.extension().and_then(|s| s.to_str()).unwrap_or("");
        for key in ["all", ext] {
            if let Some(snippets) = config.snippets.get(key) {
                for (trigger, body) in snippets {
                    if trigger.starts_with(&prefix) {
                        out.push(json!({"label":trigger,"insertText":body,"insertTextFormat":2,"kind":15,"labelDetails":{"description":"Snippets"}}));
                    }
                }
            }
        }
    }
    out
}
pub(super) fn paths(path: &std::path::Path, text: &helix_core::Rope, cursor: usize) -> Vec<Value> {
    let line = text.char_to_line(cursor);
    let before = text.slice(text.line_to_char(line)..cursor).to_string();
    let word = before
        .rsplit(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '(' | '[' | '='))
        .next()
        .unwrap_or("");
    let Some((dir, prefix)) = word.rsplit_once('/') else {
        return vec![];
    };
    let directory = if dir.starts_with('/') {
        std::path::PathBuf::from(format!("{dir}/"))
    } else {
        path.parent().unwrap_or(std::path::Path::new(".")).join(dir)
    };
    let Ok(entries) = std::fs::read_dir(directory) else {
        return vec![];
    };
    entries.take(1000).filter_map(Result::ok).filter_map(|entry| {
        let name=entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(prefix) {return None;}
        let folder=entry.file_type().is_ok_and(|t|t.is_dir());
        let label=if folder {format!("{name}/")} else {name};
        let column:usize=text.slice(text.line_to_char(line)..cursor-prefix.chars().count()).chars().map(char::len_utf16).sum();
        let end:usize=text.slice(text.line_to_char(line)..cursor).chars().map(char::len_utf16).sum();
        Some(json!({"label":label,"kind":if folder {19}else{17},"textEdit":{"range":{"start":{"line":line,"character":column},"end":{"line":line,"character":end}},"newText":label},"labelDetails":{"description":"Path"}}))
    }).take(200).collect()
}
