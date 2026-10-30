//! Small window controller: text keeps keyboard ownership unless explicitly focused.
use super::Editor;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Window {
    Documentation,
    Signature,
}
#[derive(Default)]
pub(super) struct Windows {
    pub focus: Option<Window>,
    pub signature_hidden: bool,
    pub signature_scroll: u16,
    pub hover_scroll: u16,
}
impl Editor {
    pub(super) fn document_available(&self) -> bool {
        self.popup.as_ref().is_some_and(|(method, items, i)| {
            if method.ends_with("completion") {
                self.cmp.selected
                    && !self.cmp.documentation_hidden
                    && !super::completion::documentation(&items[*i]).is_empty()
            } else {
                true
            }
        })
    }
    pub(super) fn window_key(&mut self, key: KeyEvent) -> bool {
        if self.completion_config.matches("window_close", key) {
            self.close_tool_window();
            return true;
        }
        if self.completion_config.matches("window_next", key) {
            let mut available = vec![];
            if self.document_available() {
                available.push(Window::Documentation);
            }
            if !self.windows.signature_hidden && !self.signature.value["signatures"].is_null() {
                available.push(Window::Signature);
            }
            self.windows.focus = match self.windows.focus {
                None => available.first().copied(),
                Some(w) => available
                    .iter()
                    .position(|v| *v == w)
                    .and_then(|i| available.get(i + 1).copied()),
            };
            return true;
        }
        if self.windows.focus.is_none() {
            return false;
        }
        if key.code == KeyCode::Esc {
            self.windows.focus = None;
            return true;
        }
        if key.code == KeyCode::Char('q') && key.modifiers.is_empty() {
            self.close_tool_window();
            return true;
        }
        let delta = match key.code {
            KeyCode::Up | KeyCode::Char('k') => Some(-1),
            KeyCode::Down | KeyCode::Char('j') => Some(1),
            KeyCode::PageUp => Some(-6),
            KeyCode::PageDown => Some(6),
            KeyCode::Char('u' | 'b') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(-4),
            KeyCode::Char('d' | 'f') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(4),
            _ => None,
        };
        if let Some(delta) = delta {
            let scroll = match self.windows.focus.unwrap() {
                Window::Signature => &mut self.windows.signature_scroll,
                Window::Documentation if self.completion_visible() => {
                    &mut self.cmp.documentation_scroll
                }
                _ => &mut self.windows.hover_scroll,
            };
            *scroll = (*scroll as i32 + delta).clamp(0, 2000) as u16;
            return true;
        }
        // Ordinary typing returns to the source without swallowing the key.
        self.windows.focus = None;
        false
    }
    pub(super) fn close_tool_window(&mut self) {
        let target = self.windows.focus.take().or_else(|| {
            if self.document_available() {
                Some(Window::Documentation)
            } else if !self.windows.signature_hidden && !self.signature.value.is_null() {
                Some(Window::Signature)
            } else {
                None
            }
        });
        match target {
            Some(Window::Documentation) if self.completion_visible() => {
                self.cmp.documentation_hidden = true
            }
            Some(Window::Documentation) => {
                self.popup = None;
                self.popup_text.clear();
            }
            Some(Window::Signature) => {
                self.windows.signature_hidden = true;
                self.signature_due = None;
            }
            None => {}
        }
    }
    pub fn owns_tool_window_key(&self, key: KeyEvent) -> bool {
        self.windows.focus.is_some()
            || self.completion_config.matches("window_next", key)
            || self.completion_config.matches("window_close", key)
    }
    pub(super) fn clear_tools(&mut self) {
        self.restore_preview();
        self.popup = None;
        self.popup_text.clear();
        self.cmp = Default::default();
        self.signature = Default::default();
        self.windows = Default::default();
        self.auto_due = None;
        self.signature_due = None;
        self.lsp_cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
