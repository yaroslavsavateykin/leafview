# Configuration

Main file: `$XDG_CONFIG_HOME/leafview/keybindings.toml`, normally
`~/.config/leafview/keybindings.toml`. `LEAFVIEW_KEYBINDINGS` overrides this path.
Existing JSON compile/completion configuration remains unchanged.

```toml
[keybindings]
preset = "astronvim"
leader = "Space"
local_leader = ","
timeout_ms = 500

[keybindings.normal]
"j" = "editor.cursor.down"
"Space w" = "editor.save"
"Space x" = { command = "editor.cursor.down", args = {} }
"k" = { disabled = true }

[keybindings.insert]
"Esc" = "editor.mode.normal"

[keybindings.pdf]
"j" = "pdf.scroll_down"

[keybindings.layout]
"о" = "j"
```

`[keybindings.bindings.normal]` is an equivalent explicit syntax. Entries merge
one sequence at a time: changing `j` does not remove any other defaults. Commands
accept JSON-compatible TOML arguments in Invocation, but not every builtin uses
args; count is consumed by navigation/Vim adapters. Unrecognized IDs fail loading;
last good configuration is retained after a failed hot reload.

Logical keys: characters (case sensitive), Space, Esc, Enter, Tab, Shift+Tab,
arrows, PageUp/PageDown, Home/End, F1..F24 and Ctrl/Alt/Shift/Super modifiers.
`gg`, `ciw` split into characters; `Ctrl+w h` is two strokes. Use `<Leader>ff` and
`<LocalLeader>x` for configurable prefixes. Layout translation applies outside
Insert/Command/Search only, so entered text is not transliterated.

Terminal limitations: Ctrl-I/Tab, Ctrl-M/Enter, Ctrl-[ / Esc, some shifted Ctrl
keys and Alt escape prefixes may be indistinguishable. This layer cannot recover
physical key information that the terminal never sends. Ctrl-S may be intercepted
by terminal flow control unless disabled outside LeafView.

## Layers, low to high

1. Safety/discovery defaults and selected builtin preset.
2. Global user TOML.
3. Workspace `.leafview/keybindings.toml` at the nearest `.git` ancestor (or root).
4. Current initial project root `.leafview/project-keybindings.toml`.
5. Ordered context/mode roots (Completion > Editor > Global, etc.).
6. Temporary Rust integration overrides (priority 10000).

Layer priority is 1000 + 100*layer_index + per-entry priority clamped to -49..49.
A disabled binding is a consuming tombstone, so the legacy key does not run.
An empty string is not a valid deletion: use `{disabled=true}`. Restore removes
that entry from user TOML (workspace overrides may still win).

## Keyboard Shortcuts UI

F10 opens the TUI page. Type to search descriptions, IDs, categories and keys.
F2 cycles category, Tab selects target assignment scope. Enter starts recording,
Enter finishes, Escape cancels. Enter itself is currently reserved as recording
terminator; bind it manually in TOML if required. Delete disables selected-command
bindings in that scope. F5 removes matching user overrides. F6 cycles presets.

F7 writes `keybindings-export.toml` beside the global file. F8 validates and imports
`keybindings-import.toml` from that directory. Import is explicit replacement of
that global file, not a merge; keep a backup if desired. Export includes scalar
settings, but currently does not flatten every project override into global TOML.

Conflicting recording prompts for explicit replacement at user priority. Escape
cancels. The page shows binding source/context/mode, unavailable builds and an
override/conflict count. This is a keyboard-driven TUI, not a native GUI; mouse
buttons, arbitrary import file dialogs and editing arbitrary command args are
not yet implemented.
