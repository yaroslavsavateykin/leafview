//! Stable command metadata. Execution is owned by the application, never the resolver.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invocation {
    pub command: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default = "one")]
    pub count: usize,
}
fn one() -> usize {
    1
}
impl Invocation {
    pub fn new(id: &str) -> Self {
        Self {
            command: id.into(),
            args: serde_json::Value::Null,
            count: 1,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Command {
    pub id: String,
    pub description: String,
    pub category: String,
    pub available: bool,
    pub source: String,
}
#[derive(Default)]
pub struct Registry {
    pub commands: BTreeMap<String, Command>,
}
impl Registry {
    /// Register metadata for an application/extension command. Duplicate IDs are rejected.
    pub fn register(&mut self, command: Command) -> Result<(), String> {
        if self.commands.contains_key(&command.id) {
            return Err(format!("duplicate command {}", command.id));
        }
        self.commands.insert(command.id.clone(), command);
        Ok(())
    }
    pub fn builtins() -> Self {
        let mut registry = Self::default();
        for (id, desc, category, available) in [
            (
                "app.quit",
                "Quit (protect unsaved files)",
                "Application",
                true,
            ),
            ("commands.open", "Command Palette", "Application", true),
            ("keybindings.open", "Keyboard Shortcuts", "Settings", true),
            (
                "keybindings.reload",
                "Reload Keyboard Shortcuts",
                "Settings",
                true,
            ),
            ("files.find", "Find / Open File", "Files", true),
            ("files.recent", "Focus Recent Files", "Files", true),
            ("explorer.toggle", "Toggle Explorer", "Files", true),
            ("explorer.focus", "Focus Explorer", "Files", true),
            ("explorer.next_item", "Next Explorer Item", "Files", true),
            (
                "explorer.previous_item",
                "Previous Explorer Item",
                "Files",
                true,
            ),
            ("explorer.open", "Open Explorer Item", "Files", true),
            (
                "editor.save",
                "Save File",
                "Editor",
                cfg!(feature = "editor"),
            ),
            (
                "editor.close",
                "Close File (protect unsaved)",
                "Buffers",
                cfg!(feature = "editor"),
            ),
            (
                "editor.next_buffer",
                "Next Buffer",
                "Buffers",
                cfg!(feature = "editor"),
            ),
            (
                "editor.previous_buffer",
                "Previous Buffer",
                "Buffers",
                cfg!(feature = "editor"),
            ),
            (
                "editor.toggle",
                "Toggle Source Pane",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "window.left",
                "Focus Left Pane",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "window.right",
                "Focus Right Pane",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "window.up",
                "Focus Upper Pane",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "window.down",
                "Focus Lower Pane",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "window.vertical",
                "Vertical Source/PDF Split",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "window.horizontal",
                "Horizontal Source/PDF Split",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "window.close",
                "Hide Source Pane",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "popup.next",
                "Focus Next Tool Window",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "popup.close",
                "Close Tool Window",
                "Windows",
                cfg!(feature = "editor"),
            ),
            (
                "lsp.definition",
                "Go to Definition",
                "LSP",
                cfg!(feature = "editor"),
            ),
            (
                "lsp.references",
                "Find References",
                "LSP",
                cfg!(feature = "editor"),
            ),
            (
                "lsp.hover",
                "Hover Documentation",
                "LSP",
                cfg!(feature = "editor"),
            ),
            (
                "lsp.signature",
                "Signature Help",
                "LSP",
                cfg!(feature = "editor"),
            ),
            ("lsp.format", "Format File", "LSP", cfg!(feature = "editor")),
            (
                "lsp.diagnostics",
                "Show Diagnostics",
                "LSP",
                cfg!(feature = "editor"),
            ),
            (
                "completion.show",
                "Show Completion",
                "Completion",
                cfg!(feature = "editor"),
            ),
            (
                "completion.next",
                "Next Completion",
                "Completion",
                cfg!(feature = "editor"),
            ),
            (
                "completion.previous",
                "Previous Completion",
                "Completion",
                cfg!(feature = "editor"),
            ),
            (
                "completion.accept",
                "Accept Completion",
                "Completion",
                cfg!(feature = "editor"),
            ),
            (
                "completion.hide",
                "Hide Completion",
                "Completion",
                cfg!(feature = "editor"),
            ),
            ("pdf.next_page", "Next PDF Page", "PDF", true),
            ("pdf.previous_page", "Previous PDF Page", "PDF", true),
            ("pdf.first_page", "First PDF Page", "PDF", true),
            ("pdf.last_page", "Last PDF Page", "PDF", true),
            ("pdf.scroll_down", "Scroll PDF Down", "PDF", true),
            ("pdf.scroll_up", "Scroll PDF Up", "PDF", true),
            ("pdf.zoom_in", "Zoom PDF In", "PDF", true),
            ("pdf.zoom_out", "Zoom PDF Out", "PDF", true),
            ("pdf.reload", "Reload PDF", "PDF", true),
            ("pdf.fit_page", "Fit Entire PDF Page", "PDF", true),
            ("pdf.fit_width", "Fit PDF Width", "PDF", true),
            (
                "typst.compile",
                "Compile Typst",
                "Typst",
                cfg!(feature = "typst"),
            ),
        ] {
            registry
                .register(Command {
                    id: id.into(),
                    description: desc.into(),
                    category: category.into(),
                    available,
                    source: "builtin".into(),
                })
                .unwrap();
        }
        registry
            .register(Command {
                id: "editor.mode.normal".into(),
                description: "Normal Mode".into(),
                category: "Vim".into(),
                available: cfg!(feature = "editor"),
                source: "helix-core/Vim".into(),
            })
            .unwrap();
        for (name, desc, sequence) in vim_commands() {
            let _ = sequence;
            registry
                .register(Command {
                    id: format!("editor.{name}"),
                    description: desc.into(),
                    category: "Vim".into(),
                    available: cfg!(feature = "editor"),
                    source: "helix-core/Vim".into(),
                })
                .unwrap();
        }
        registry
    }
}
/// Adapter names for existing compositional Vim operations; not a second text engine.
pub fn vim_commands() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("cursor.left", "Cursor Left", "h"),
        ("cursor.down", "Cursor Down", "j"),
        ("cursor.up", "Cursor Up", "k"),
        ("cursor.right", "Cursor Right", "l"),
        ("cursor.word_next", "Next Word", "w"),
        ("cursor.word_previous", "Previous Word", "b"),
        ("cursor.word_end", "Word End", "e"),
        ("cursor.big_word_next", "Next WORD", "W"),
        ("cursor.big_word_previous", "Previous WORD", "B"),
        ("cursor.big_word_end", "WORD End", "E"),
        ("cursor.line_start", "Line Start", "0"),
        ("cursor.first_nonblank", "First Nonblank", "^"),
        ("cursor.line_end", "Line End", "$"),
        ("cursor.file_start", "First Line", "gg"),
        ("cursor.file_end", "Last Line", "G"),
        ("cursor.match_bracket", "Matching Bracket", "%"),
        ("cursor.paragraph_next", "Next Paragraph", "}"),
        ("cursor.paragraph_previous", "Previous Paragraph", "{"),
        ("mode.insert", "Insert Mode", "i"),
        ("mode.append", "Append", "a"),
        ("mode.insert_line_start", "Insert at Line Start", "I"),
        ("mode.append_line_end", "Append at Line End", "A"),
        ("mode.open_below", "Open Line Below", "o"),
        ("mode.open_above", "Open Line Above", "O"),
        ("mode.visual", "Visual Mode", "v"),
        ("mode.visual_line", "Visual Line", "V"),
        ("mode.visual_block", "Visual Block", "Ctrl+v"),
        ("mode.command", "Command Line", ":"),
        ("mode.search", "Search Forward", "/"),
        ("mode.search_backward", "Search Backward", "?"),
        ("undo", "Undo", "u"),
        ("redo", "Redo", "Ctrl+r"),
        ("paste", "Paste After", "p"),
        ("paste_before", "Paste Before", "P"),
        ("delete_character", "Delete Character", "x"),
        ("delete_line", "Delete Line", "dd"),
        ("copy_line", "Copy Line", "yy"),
        ("change_line", "Change Line", "cc"),
        ("delete_word", "Delete Word", "dw"),
        ("change_inner_word", "Change Inner Word", "ciw"),
        ("delete_inner_word", "Delete Inner Word", "diw"),
        ("repeat", "Repeat Last Change", "."),
        ("search.next", "Next Search Match", "n"),
        ("search.previous", "Previous Search Match", "N"),
    ]
}
