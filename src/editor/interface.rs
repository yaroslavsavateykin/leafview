//! Tab hit geometry and structured language-server interactions.
use super::{lsp, Editor};
use ratatui::layout::Rect;
use serde_json::Value;
impl Editor {
    pub fn tab_regions(&self, area: Rect) -> Vec<(usize, u16, u16)> {
        let mut x = area.x.saturating_add(1);
        let mut tabs = vec![];
        // Keep the active tab visible even with more buffers than available cells.
        let width = area.width.saturating_sub(2);
        let count = self.buffers.len().min(usize::from(width / 12)).max(1);
        let first = self.current.saturating_sub(count - 1);
        for index in first..self.buffers.len().min(first + count) {
            let size = (width / count as u16).min(area.right().saturating_sub(1).saturating_sub(x));
            tabs.push((index, x, size));
            x += size;
        }
        tabs
    }
    pub(super) fn receive_lsp(&mut self, text: &str) {
        let Ok(messages) = serde_json::from_str::<Vec<Value>>(text) else {
            self.message = text.into();
            return;
        };
        let Some(reply) = messages.last() else {
            return;
        };
        if !reply["error"].is_null() {
            self.message = reply["error"].to_string();
            return;
        }
        let method = reply["method"].as_str().unwrap_or("");
        let result = &reply["result"];
        if method == "textDocument/signatureHelp" {
            self.receive_signature(result);
            return;
        }
        if method == "completionItem/resolve" {
            self.resolved_completion(result);
            return;
        }
        let items = if method.ends_with("completion") {
            let mut items = result
                .as_array()
                .or_else(|| result["items"].as_array())
                .cloned()
                .unwrap_or_default();
            if let Some(defaults) = result["itemDefaults"].as_object() {
                for item in &mut items {
                    for key in ["insertTextFormat", "insertTextMode", "data"] {
                        if item.get(key).is_none() {
                            if let Some(v) = defaults.get(key) {
                                item[key] = v.clone();
                            }
                        }
                    }
                    if item.get("textEdit").is_none() {
                        if let Some(range) = defaults.get("editRange") {
                            let text = item["textEditText"]
                                .as_str()
                                .or_else(|| item["insertText"].as_str())
                                .or_else(|| item["label"].as_str())
                                .unwrap_or("")
                                .to_owned();
                            item["textEdit"] = if range.get("start").is_some() {
                                serde_json::json!({"range":range,"newText":text})
                            } else {
                                serde_json::json!({"insert":range["insert"],"replace":range["replace"],"newText":text})
                            };
                        }
                    }
                }
            }
            items
        } else if method.ends_with("definition") || method.ends_with("references") {
            result.as_array().cloned().unwrap_or_else(|| {
                if result.is_object() {
                    vec![result.clone()]
                } else {
                    vec![]
                }
            })
        } else if method.ends_with("signatureHelp") {
            result["signatures"].as_array().cloned().unwrap_or_default()
        } else if method.ends_with("publishDiagnostics") {
            reply["params"]["diagnostics"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        } else {
            let contents = &result["contents"];
            vec![serde_json::json!({"documentation":contents})]
        };
        if method.ends_with("hover") {
            self.windows.hover_scroll = 0;
            self.windows.focus = None;
        }
        let previous = if self.completion_visible() && self.cmp.selected {
            self.popup.as_ref().map(|(_, items, i)| items[*i].clone())
        } else {
            None
        };
        let mut items = items;
        if method.ends_with("completion") {
            if let Some(b) = self.buffers.get(self.current) {
                let local = super::completion_sources::local(b, &self.completion_config);
                for item in local {
                    if !items.iter().any(|v| v["label"] == item["label"]) {
                        items.push(item);
                    }
                }
                items = super::completion_rank::rank(items, b);
            }
        }
        let items: Vec<_> = items
            .into_iter()
            .filter(|v| !v.is_null())
            .take(200)
            .collect();
        if items.is_empty() {
            if method.ends_with("completion") {
                self.popup = None;
                self.popup_text.clear();
            }
            self.message = "LSP: no results".into();
            return;
        }
        let retained = previous.as_ref().and_then(|old| {
            items.iter().position(|v| {
                v["label"] == old["label"]
                    && v["insertText"] == old["insertText"]
                    && v["textEdit"] == old["textEdit"]
            })
        });
        if method.ends_with("completion") {
            self.cmp = super::completion::State::default();
            self.cmp.selected = retained.is_some() || self.completion_config.preselect;
            self.cmp.documentation_hidden = !self.completion_config.documentation_auto_show;
            self.cmp.resolve_due = self.cmp.selected;
        }
        self.popup = Some((method.into(), items, retained.unwrap_or(0)));
        self.refresh_popup();
        if method.ends_with("definition") {
            self.accept_popup();
        }
    }
    fn refresh_popup(&mut self) {
        let Some((_, items, selected)) = &self.popup else {
            return;
        };
        self.popup_text = items
            .iter()
            .enumerate()
            .flat_map(|(i, item)| {
                let text = item
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| {
                        ["label", "value", "message", "uri", "targetUri"]
                            .iter()
                            .find_map(|key| item[*key].as_str().map(str::to_owned))
                    })
                    .unwrap_or_else(|| item.to_string());
                text.lines()
                    .take(20)
                    .map(|line| {
                        format!(
                            "{} {}",
                            if i == *selected { "›" } else { " " },
                            crate::links::sanitize(line)
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
    }
    pub(super) fn move_popup(&mut self, delta: isize) {
        if let Some((_, items, selected)) = &mut self.popup {
            *selected = (*selected as isize + delta).rem_euclid(items.len() as isize) as usize;
        }
        self.refresh_popup();
    }
    pub(super) fn accept_popup(&mut self) {
        self.restore_preview();
        let Some((method, items, selected)) = self.popup.take() else {
            return;
        };
        self.popup_text.clear();
        let item = &items[selected];
        if method.ends_with("completion") {
            let Some(b) = self.buffers.get_mut(self.current) else {
                return;
            };
            let (start, end, mut text) = super::completion::edit(item, b);
            let mut rendered = None;
            if item["insertTextFormat"] == 2 {
                let expanded = super::completion::expand(&text, start);
                text = expanded.0;
                rendered = expanded.1;
            } else if self.completion_config.auto_brackets
                && matches!(item["kind"].as_u64(), Some(2 | 3))
                && !text.contains('(')
                && (end >= b.state.doc.len_chars() || b.state.doc.char(end) != '(')
            {
                text.push_str("()");
            }
            let mut changes = vec![(start, end, Some(text.clone().into()))];
            let mut shift = 0isize;
            if let Some(edits) = item["additionalTextEdits"].as_array() {
                for edit in edits {
                    let a = lsp::value_position(&b.state.doc, &edit["range"]["start"]);
                    let z = lsp::value_position(&b.state.doc, &edit["range"]["end"]);
                    let t = edit["newText"].as_str().unwrap_or("");
                    if z <= start || a >= end {
                        if z <= start {
                            shift += t.chars().count() as isize - (z - a) as isize;
                        }
                        changes.push((a, z, Some(t.into())));
                    }
                }
            }
            changes.sort_by_key(|c| (c.0, c.1));
            if changes.windows(2).any(|w| w[0].1 > w[1].0) {
                self.message = "LSP returned overlapping completion edits".into();
                return;
            }
            b.commit();
            b.edit_many(changes);
            b.set_cursor((start as isize + shift).max(0) as usize + text.chars().count());
            if let Some(mut snippet) = rendered {
                if shift != 0 {
                    for tab in &mut snippet.tabstops {
                        for range in &mut tab.ranges {
                            range.start = (range.start as isize + shift).max(0) as usize;
                            range.end = (range.end as isize + shift).max(0) as usize;
                        }
                    }
                    for range in &mut snippet.ranges {
                        range.start = (range.start as isize + shift).max(0) as usize;
                        range.end = (range.end as isize + shift).max(0) as usize;
                    }
                }
                b.state.selection =
                    snippet.first_selection(helix_core::movement::Direction::Forward, 0);
                b.snippet = helix_core::snippets::ActiveSnippet::new(snippet);
            } else if text.ends_with("()") {
                b.set_cursor(b.cursor().saturating_sub(1));
            }
            b.commit();
            self.schedule_signature();
        } else if method.ends_with("definition") || method.ends_with("references") {
            let Some(uri) = item["uri"].as_str().or_else(|| item["targetUri"].as_str()) else {
                return;
            };
            let Some(path) = lsp::file_path(uri) else {
                self.message = "Unsupported location URI".into();
                return;
            };
            let range = item.get("targetSelectionRange").unwrap_or(&item["range"]);
            let line = range["start"]["line"].as_u64().unwrap_or(0) as usize;
            let column = range["start"]["character"].as_u64().unwrap_or(0) as usize;
            self.pending_jump = Some((path.clone(), line, column));
            self.open(&path);
            if let Some(b) = self
                .buffers
                .get_mut(self.current)
                .filter(|b| b.path == path)
            {
                b.set_cursor(lsp::char_position(&b.state.doc, line, column));
                self.pending_jump = None;
            }
        }
    }
}
