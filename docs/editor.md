# Source editor

[Documentation](README.md) · [Project](../README.md)

Build with `--features editor` (and `typst` for live compilation). `F4` toggles
the source pane; `F6` switches source/PDF focus. The editor uses Helix Core's
Rope, Selection, Transaction and History, with LeafView's own modal input and UI.
It is experimental and is not the Helix terminal application or a complete Vim IDE.

## Editing and files

`i` enters Insert mode; Escape returns to Normal. Motions, counts, operators,
Visual selections, internal registers, macros, search and undo/redo are supported
in part. Highlighting is bounded viewport lexical highlighting, not Tree-sitter
or semantic highlighting. Multiline and embedded syntax is approximate.

`:w` saves asynchronously. `:e PATH` / `:tabnew PATH` open retained buffers;
`:bn`, `:bp`, `gt`, `gT`, `Ctrl+PageUp/PageDown` switch tabs. `:ls` lists them,
`:b 2` selects one and `:bd` closes it with dirty protection (`:bd!` discards).
Click tabs or their `×` targets. Mouse dragging selects source text in Visual
mode; `y` copies to the internal register, not the system clipboard.

`:q` hides a clean pane; `:q!` hides it but retains unsaved buffers. `Ctrl+C`
requires confirmation when a source buffer is dirty.

## Panes and file selection

`F3` opens the focused pane's picker. Source selection is independent of preview
selection; opening a source file does not replace the compiled entry document.
The sidebar remembers whether it was entered from source or preview; PDFs always
open as previews. Source pickers include common text and code formats.

`Ctrl+w` then `h/j/k/l` moves focus, `w` cycles source/preview, `v` / `s` selects
vertical/horizontal layout, `<` / `>` resizes, `=` equalizes and `c` hides source.
`Alt+Left/Right` resizes source/PDF. Shared borders are draggable with plain left
button; terminal selection modifiers may intercept Shift+drag.

There is one source view with multiple file tabs, not arbitrary independent
source splits. Layout persistence is not implemented.

## Language servers

Servers are separate executables on PATH:

| Source | Server | Formatter |
| --- | --- | --- |
| Typst | `tinymist` | `typstyle` |
| Rust | `rust-analyzer` | `rustfmt` |
| Python | `basedpyright-langserver --stdio` | `black` |
| C/C++ | `clangd` | `clang-format` |
| JS/TS | `typescript-language-server` | `prettier` |

`LEAFVIEW_LSP='program args'` and `LEAFVIEW_FORMATTER='program args'` override
the commands. Arguments are whitespace-separated, **not shell syntax**. Feature
names `tinymist` and `typstyle` merely enable `editor`; they do not embed tools.

Commands: `:hover`, `:complete`, `:definition`, `:references`, `:signature`,
`:symbols`, `:diagnostics`, `:lsp-restart`. Shortcuts include `F1` / `K` for
hover, `F12` / `gd` for definition, `gr` for references, `Ctrl+Space` for
completion and `Ctrl+s` for signature help. One active server/document is kept
by the dedicated worker; stale path/version/cursor replies are rejected.

## Completion and tool windows

Automatic completion follows a typing pause (default 250 ms). `(` and `,`
trigger signature help. `:set noautocomplete` disables automatic suggestions;
`:set autocomplete` restores them. Completion can combine LSP, local buffer,
path and snippet providers; Helix/nucleo ranks candidates.

- Tab/Down/Ctrl+n/Ctrl+j select next; Shift+Tab/Up/Ctrl+p/Ctrl+k select previous.
- Enter accepts a selected item. Browsing does **not** insert by default.
- Ctrl+e dismisses; Ctrl+Space opens completion or toggles item documentation.
- Snippet placeholders use Helix parsing and tab navigation. Dynamic LuaSnip
  nodes, mirrored-placeholder synchronization and full LuaSnip parity are absent.
- Documentation scroll uses Ctrl+u/d or Ctrl+b/f when completion owns input.

`F7` cycles Documentation → Arguments → Source when those tools exist. `F8`
closes the focused tool; from Source it closes documentation before arguments.
In a focused tool, `q` closes, Escape returns to Source, arrows/j/k and paging
scroll without editing. Ordinary typing returns to Source.

Configure partial JSON in `$XDG_CONFIG_HOME/leafview/completion.json`, or set
`LEAFVIEW_COMPLETION_CONFIG`. [The example](../examples/completion.json) covers
colors, sizes, border style, sources, snippets and bindings. Invalid/unknown
fields produce an editor message. A Nerd Font is optional for completion glyphs;
the checked-in showcase uses DejaVu Sans Mono and its available symbols.

## Formatting

`Alt+f` or `:format` runs the external formatter. Format-on-save is enabled for
recognized extensions; `:set noformatonsave` disables it and
`:set formatonsave` enables it. If the executable is absent, disable formatting
or install it before saving. Formatting is undoable; stale results are ignored.

## Boundaries

No rename/code-action/workspace-edit application, persistent diagnostic gutter,
or full incremental LSP document sync. No system-clipboard registers. Vim edge
cases and some text objects remain incomplete; `==` explicitly directs users to
`:format`. Source-to-PDF following is compiler-span based, not bidirectional
PDF-click-to-source navigation. See [Typst mapping limits](typst.md).

Historical implementation notes and measurements are retained in the
[archive](archive/README.md), not treated as current feature specifications.
