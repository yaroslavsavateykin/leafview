# Architecture

## Existing components reused

- `src/main.rs`: TUI event loop, focus/panes, PDF navigation and worker scheduling.
- `src/editor/buffer.rs`: helix-core Rope, selections, transactions and undo history.
- `src/editor/vim/`: existing count/operator/motion/text-object parser, registers,
  macros, visual modes, command/search entry. This is retained, not reimplemented.
- `src/editor/lsp.rs`, `tasks.rs`: bounded asynchronous language-server integration.
- `src/files.rs`, `picker.rs`: Explorer, recent files and file picker.
- `compile_settings.rs`: Typst-specific settings, not a general settings framework.
- `terminal.rs` is terminal graphics negotiation, **not an embedded shell**.
- No dynamic plugin loader or embedded Git/OpenCode/Marimo currently exists.

## New modules

`keybindings/registry.rs` owns stable command metadata and Invocation (ID, JSON
args, count). `commands.rs` is the application execution endpoint; editor actions
are delegated to `editor/commands.rs`. Unknown/unavailable IDs produce a notice.
Command handlers operate on app state, not by looking up the configured key.
Some editor adapters intentionally use canonical existing Vim operations to
preserve helix-core history and compositional behavior. This is a transitional
adapter, not total separation of every legacy action from keys.

`keybindings/keys.rs` defines frontend-neutral Stroke, modifiers and sequences.
Logical characters and explicit Physical: codes are distinct; terminal events
lack physical scan codes, so physical bindings are representable but not fired
by the current crossterm frontend. Super is represented; terminal support varies.

`keybindings/engine.rs` compiles tries keyed by (context, mode). Resolution is
separate from execution. A pending terminal with children waits for timeout,
then executes its deferred command. Escape cancels; unresolved prefixes replay
to the existing handler to preserve Vim grammar. Counts are attached to commands.
Tries are rebuilt on config changes, not per event. BTreeMap child lookup is
O(log branching); the command registry is not searched on dispatch.

`keybindings/config.rs` parses and merges TOML layers. `keybindings/ui.rs` renders
WhichKey, palette and Keyboard Shortcuts from the same registry/trie data.
`keybindings/mod.rs` manages loading and periodic mtime reload (750 ms).

## Focus/context routing

Settings and file-search text fields do not inherit global Vim character maps.
Tool-window focus is isolated. Completion resolves before Editor, then Global.
Explorer and PDF have separate roots. Operator pending and command/search use
separate modes so a user normal-mode override does not break `diw`.

Custom context names can be configured (`[keybindings.MyContext.any]`); a future
frontend/integration must include that context in the ordered stack passed to
Engine::feed. There is no expression-based `when` language or context tree DSL.

## Extension API

Rust integrations can call Registry::register(Command), register an application
handler in App::execute_command, then System::override_temporarily(bindings).
Temporary bindings have priority 10000. Reload removes temporary bindings.
Registration alone does not invent an executor: no dynamically loaded code,
shell execution or arbitrary plugin callback system is shipped. Presets currently
use one validated builtin selector plus TOML layers, not external preset packages.
