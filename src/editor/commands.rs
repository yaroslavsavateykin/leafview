//! Command adapters reuse existing Vim transactions, completion state and LSP worker.
use super::Editor;
use crate::keybindings::{keys, Invocation};
impl Editor {
    pub fn binding_mode(&self) -> &'static str {
        self.vim.binding_mode()
    }
    pub fn has_completion(&self) -> bool {
        self.completion_visible()
    }
    pub fn execute_command(&mut self, invocation: &Invocation) -> Result<(), String> {
        let id = invocation.command.as_str();
        match id {
            "editor.save" => self.save(),
            "editor.close" => self.close_buffer(false),
            "editor.next_buffer" | "editor.previous_buffer" => {
                if !self.buffers.is_empty() {
                    self.clear_tools();
                    let delta = if id.ends_with("next_buffer") {
                        1
                    } else {
                        self.buffers.len() - 1
                    };
                    self.current = (self.current + delta) % self.buffers.len();
                    self.vim.reset_mode();
                }
            }
            "editor.mode.normal" => {
                self.clear_tools();
                self.vim.reset_mode();
            }
            "lsp.format" => self.format(),
            _ if id.starts_with("lsp.") => self.lsp_command(id.trim_start_matches("lsp.")),
            "completion.show" => self.request_lsp("complete", false),
            "completion.next" | "completion.previous" => {
                if self.completion_visible() {
                    self.restore_preview();
                    if self.cmp.selected {
                        self.move_popup(if id.ends_with("next") { 1 } else { -1 });
                    } else {
                        self.cmp.selected = true;
                    }
                    self.cmp.resolve_due = true;
                    self.cmp.resolved = false;
                    self.cmp.documentation_scroll = 0;
                    self.cmp.documentation_hidden = !self.completion_config.documentation_auto_show;
                } else {
                    self.request_lsp("complete", false);
                }
            }
            "completion.accept" => {
                if self.completion_visible() && self.cmp.selected {
                    self.restore_preview();
                    if !self.cmp.resolved
                        && self
                            .popup
                            .as_ref()
                            .is_some_and(|(_, items, i)| items[*i].get("data").is_some())
                    {
                        self.cmp.accepting = true;
                        self.cmp.resolve_due = true;
                    } else {
                        self.accept_popup();
                    }
                } else {
                    self.key(crossterm::event::KeyEvent::from(
                        crossterm::event::KeyCode::Enter,
                    ));
                }
            }
            "completion.hide" => {
                self.restore_preview();
                self.popup = None;
                self.popup_text.clear();
            }
            "popup.close" => self.close_tool_window(),
            "popup.next" => {
                self.window_key(crossterm::event::KeyEvent::from(
                    crossterm::event::KeyCode::F(7),
                ));
            }
            _ => {
                let sequence = crate::keybindings::registry::vim_commands()
                    .into_iter()
                    .find(|(name, _, _)| format!("editor.{name}") == id)
                    .map(|(_, _, seq)| seq)
                    .ok_or_else(|| format!("Unknown editor command {id}"))?;
                if self.buffers.is_empty() {
                    return Err("No source buffer".into());
                }
                let mut seq = String::new();
                if invocation.count > 1 {
                    seq.push_str(&invocation.count.to_string());
                }
                seq.push_str(sequence);
                for key in keys::sequence(&seq, "Space", ",")? {
                    if let Some(event) = key.to_event() {
                        self.vim.key(&mut self.buffers[self.current], event);
                    }
                }
            }
        }
        Ok(())
    }
}
