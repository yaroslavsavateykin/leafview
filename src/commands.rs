//! Application command execution; independent from keybinding resolution.
use crate::{
    keybindings::{keys::Stroke, Invocation, Resolution},
    App,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
impl App {
    pub(crate) fn binding_context(&self) -> (Vec<&'static str>, &'static str) {
        if self.picker.is_some() {
            return (vec!["Search"], "insert");
        }
        #[cfg(feature = "typst")]
        if self.settings.is_some() {
            return (vec!["Settings"], "insert");
        }
        #[cfg(feature = "editor")]
        if self.editor.visible && self.editor.focused {
            if self
                .editor
                .owns_tool_window_key(KeyEvent::from(KeyCode::Null))
            {
                return (vec!["Popup"], "any");
            }
            let mode = self.editor.binding_mode();
            let mut contexts = vec![];
            if self.editor.has_completion() {
                contexts.push("Completion");
            }
            contexts.push("Editor");
            if !matches!(mode, "command" | "search" | "operator") {
                contexts.push("Global");
            }
            return (contexts, mode);
        }
        if self.files.focused {
            return (vec!["Explorer", "Global"], "normal");
        }
        (vec!["PDF", "Global"], "normal")
    }
    pub(crate) fn dispatch_bindings(&mut self, key: KeyEvent) -> Option<bool> {
        if self.keybindings.active_panel() {
            if let Some(invocation) = self.keybindings.panel_key(key) {
                return Some(self.execute_command(invocation));
            }
            self.dirty = true;
            return Some(false);
        }
        let (contexts, mode) = self.binding_context();
        let mut stroke = Stroke::event(key);
        if !matches!(mode, "insert" | "command" | "search") {
            if let Some(mapped) = self.keybindings.config.layout.get(&stroke.key) {
                stroke.key = mapped.clone();
            }
        }
        let result =
            self.keybindings
                .engine
                .feed(stroke, &contexts, mode, std::time::Instant::now());
        self.resolve_binding(result)
    }
    pub(crate) fn resolve_binding(&mut self, result: Resolution) -> Option<bool> {
        match result {
            Resolution::Pass => None,
            Resolution::Pending | Resolution::Consumed => {
                self.dirty = true;
                Some(false)
            }
            Resolution::Execute(invocation) => Some(self.execute_command(invocation)),
            Resolution::Replay(keys) => {
                let mut quit = false;
                for key in keys {
                    if let Some(event) = key.to_event() {
                        quit |= self.on_key_legacy(event);
                    }
                }
                self.dirty = true;
                Some(quit)
            }
        }
    }
    /// Shared execution endpoint for keyboard, palette and future extension menus.
    pub(crate) fn execute_command(&mut self, invocation: Invocation) -> bool {
        if let Err(error) = self.keybindings.validate(&invocation) {
            self.files.notice = Some(error);
            self.dirty = true;
            return false;
        }
        let count = invocation.count.clamp(1, 100000);
        let id = invocation.command.as_str();
        match id {
            "commands.open" => self.keybindings.open(false),
            "keybindings.open" => self.keybindings.open(true),
            "keybindings.reload" => {
                if let Err(e) = self.keybindings.reload() {
                    self.files.notice = Some(e);
                }
            }
            "app.quit" => {
                return self.on_key_legacy(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
            }
            "files.find" => {
                #[cfg(feature = "editor")]
                let target = if self.editor.focused {
                    crate::picker::Target::Editor
                } else {
                    crate::picker::Target::Preview
                };
                #[cfg(not(feature = "editor"))]
                let target = crate::picker::Target::Preview;
                self.begin_picker(target);
            }
            "files.recent" => {
                self.files.visible = true;
                self.files.focused = true;
                self.files.recent_focused = true;
                #[cfg(feature = "editor")]
                {
                    self.editor.focused = false;
                }
            }
            "explorer.toggle" => {
                self.files.visible = !self.files.visible;
                self.files.focused = self.files.visible;
                #[cfg(feature = "editor")]
                if self.files.focused {
                    self.editor.focused = false;
                }
            }
            "explorer.focus" => {
                self.files.visible = true;
                self.files.focused = true;
                #[cfg(feature = "editor")]
                {
                    self.editor.focused = false;
                }
            }
            "explorer.next_item" | "explorer.previous_item" | "explorer.open" => {
                let key = if id.ends_with("next_item") {
                    KeyCode::Down
                } else if id.ends_with("previous_item") {
                    KeyCode::Up
                } else {
                    KeyCode::Enter
                };
                for _ in 0..count.min(1000) {
                    if let Some(path) = self.files.key(KeyEvent::from(key)) {
                        #[cfg(feature = "editor")]
                        let target = if self.browser_editor {
                            crate::picker::Target::Editor
                        } else {
                            crate::picker::Target::Preview
                        };
                        #[cfg(not(feature = "editor"))]
                        let target = crate::picker::Target::Preview;
                        self.choose_file(path, target);
                    }
                }
            }
            "pdf.scroll_down" | "pdf.scroll_up" => {
                self.view.scroll(
                    0,
                    (count as i32).saturating_mul(if id.ends_with("down") { 30 } else { -30 }),
                );
                self.schedule();
            }
            "pdf.next_page" | "pdf.previous_page" => {
                self.view
                    .step_page((count as i32) * if id.ends_with("next_page") { 1 } else { -1 });
                self.schedule();
            }
            "pdf.first_page" | "pdf.last_page" => {
                self.view.go_to(if id.ends_with("first_page") {
                    1
                } else {
                    self.view.pages
                });
                self.schedule();
            }
            "pdf.zoom_in" | "pdf.zoom_out" => {
                self.view
                    .adjust_zoom(count as f32 * if id.ends_with("in") { 0.05 } else { -0.05 });
                self.schedule();
            }
            "pdf.fit_page" | "pdf.fit_width" => {
                self.view.x = 0;
                self.view.y = 0;
                self.destination = None;
                self.view.fit = if id == "pdf.fit_page" {
                    crate::view::Fit::Page
                } else {
                    crate::view::Fit::Width
                };
                self.schedule();
            }
            "pdf.reload" => self.reload(),
            #[cfg(feature = "typst")]
            "typst.compile" => {
                if let Some(live) = &mut self.live {
                    live.refresh();
                } else {
                    self.files.notice = Some("No Typst project open".into());
                }
            }
            #[cfg(feature = "editor")]
            "window.left" | "window.right" | "window.up" | "window.down" => {
                self.focus_pane(match id {
                    "window.left" => 'h',
                    "window.right" => 'l',
                    "window.up" => 'k',
                    _ => 'j',
                })
            }
            #[cfg(feature = "editor")]
            "window.vertical" | "window.horizontal" => {
                self.editor.visible = true;
                self.editor.horizontal = id == "window.horizontal";
            }
            #[cfg(feature = "editor")]
            "window.close" => {
                self.editor.visible = false;
                self.editor.focused = false;
            }
            #[cfg(feature = "editor")]
            "editor.toggle" => {
                if self.editor.visible {
                    self.editor.visible = false;
                    self.editor.focused = false;
                } else if !self.source.is_dir() {
                    self.editor.open(&self.source);
                } else {
                    self.begin_picker(crate::picker::Target::Editor);
                }
            }
            #[cfg(feature = "editor")]
            _ if id.starts_with("editor.")
                || id.starts_with("lsp.")
                || id.starts_with("completion.")
                || id.starts_with("popup.") =>
            {
                if let Err(e) = self.editor.execute_command(&invocation) {
                    self.files.notice = Some(e);
                }
            }
            _ => self.files.notice = Some(format!("No application handler for {id}")),
        }
        self.geometry(self.terminal_size);
        self.dirty = true;
        false
    }
}
