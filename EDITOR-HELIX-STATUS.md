# Helix Core integration — implementation status

This is a working, **partial** native editor implementation, not completion of
the full Vim/LSP/IDE specification. Changes were made on main without switching
branches. The separate `feat/editor` prototype was inspected, not overwritten or
merged. Its modalkit implementation and embedded Tinymist are not in main.

## Architecture and dependencies

Feature `editor` activates helix-core 25.07.1 pinned to commit
`a05c151bb6e8e9c65ec390b0ae2afe7a5efd619b` from upstream Git. Helix components
are MPL-2.0; no upstream implementation files were copied. Leafview remains MIT;
redistribution must retain dependency license notices and comply with MPL for
any modifications to covered files. No helix-term or editor PTY is used.

* `src/editor/buffer.rs`: Helix Rope/Selection/Transaction/History adapter;
  transaction composition groups insert sessions into one undo step. Snapshot
  clones share Rope structure. File saves stream Rope chunks to an atomic file.
* `src/editor/vim/`: separate modal/pending-operator/count/register/macro machine.
* `src/editor/render.rs`: viewport-only Ratatui rendering, Unicode cell widths;
  no full-document String conversion on ordinary input or redraw.
* `src/editor/tasks.rs`: bounded owned worker for file IO, external formatters,
  explicit LSP requests; timeouts and process cleanup.
* `src/editor/lsp.rs`: bounded framed stdio JSON-RPC, UTF-16 positions,
  initialize/open/change/save/close, request versions, stale reply rejection.
* `src/main.rs`: F4 source pane, F6 focus; Ctrl+h/j/k/l and Ctrl+arrows
  navigate geometrically between tree, Recent, source and PDF in every mode;
  Ctrl+w w from Normal returns to PDF;
  source keys do not enter viewer handlers. Mouse clicks activate panes and
  place the source cursor; wheel scrolls source; Insert arrows/Home/End move
  the cursor. Ctrl+C exits; dirty buffers require a second Ctrl+C confirmation.
  F2 toggles the tree globally (Esc retains its Vim meaning in source).

## Usage

```
cargo build --features typst,editor
LEAFVIEW_GRAPHICS=safe target/debug/leafview main.typ
```

F4 opens/hides source to the right of the file tree, with PDF to its right;
F4 also makes the file tree visible. F6 changes focus. Active source has a
brighter blue-gray background, cursor line and highlighted border; its filename
is shown once in the top border, mode/position at bottom left. Alt+Left/Right or
Normal-mode Ctrl+w then </> resize source/PDF (20–80%); Ctrl+w = equalizes,
Ctrl+w w returns to PDF. PDF stays allocated while resizing. Insert Ctrl+n/p
cycles bounded neighborhood word completion; pending Vim operators show hints. `:w` saves asynchronously; the existing
Typst file watcher recompiles on save. `:e PATH`, `:tabnew PATH`, `:bn`, `:bp`,
`:tabnext`, `:tabprevious` switch retained buffers. `:sp` / `:vsp` change the
source/PDF split orientation (NOT arbitrary independent editor windows).
`:q` hides a clean source pane; `:q!` hides but retains unsaved buffers.

`:format` selects external typstyle/rustfmt/black/clang-format/prettier by file
extension. Executables must be installed. Results are applied only if path and
version still match; formatting is undoable. `tinymist` and `typstyle` feature
names imply editor for build compatibility but do NOT embed those tools.

Explicit LSP commands: `:hover`, `:complete`, `:definition`, `:references`,
`:signature`, `:symbols`, `:diagnostics`, `:lsp-restart`. Responses use a structured popup for hover/signatures, completion acceptance,
and definition/reference navigation. F1/F12/Ctrl+Space/Ctrl+s invoke these actions.
Servers are external tinymist/rust-analyzer/clangd/pyright-langserver/typescript-language-server; override
with `LEAFVIEW_LSP='program args'`. Sync currently happens on explicit requests,
on explicit requests and debounced automatic completion/signature requests. One
active document/server is maintained in a dedicated LSP worker.

## Tests run

* Nine editor tests passed: buffer operations, insert grouping, undo/redo,
  Unicode, counts, named registers, macros, dot repeat, Visual deletion,
  paragraph objects, search, CRLF save/reopen, substitution, snapshot sharing,
  stale save protection, real external rustfmt + undo, tiny viewport rendering,
  LSP framing/bounds and a fake external stdio server.
* Real Neovim 0.11.6 comparison checks final text AND byte cursor coordinates
  for `diw`, `ciwNEW<Esc>`, `2dd`, `d3w`, `vld`, `Vjd`, `ggVGd`, `dap`.
* `scripts/helix-editor-smoke.py`: real PTY, F4 open, `ciw`, async save,
  hide and terminal restoration passed with typst+editor debug build.
* `cargo check --all-features` and no-default-features passed.
* All-features unit run excluding existing link fixture/annotation suites:
  57 passed, 3 ignored, 7 filtered. This is NOT a clean full-suite claim.

## Performance measurements

Command:
`cargo test --release --features editor benchmark_large_files -- --ignored --nocapture`

Generated ASCII multiline files (~1/10/100 MiB), warm local filesystem;
Rust 1.98.1. Open includes file read + Rope construction; excludes PDF/compiler.
1000 `ix<Esc>` edit triplets; 100 TestBackend renders at 100x40. RSS is current
process RSS from /proc/self/status, not isolated peak or editor-only allocation.

| File | Open ms | Edit p50 us | Edit p99 us | Render us | RSS KiB |
|---|---:|---:|---:|---:|---:|
| 1 MiB | 2.187 | 2.037 | 9.159 | 308.627 | 7000 |
| 10 MiB | 14.732 | 1.745 | 6.023 | 318.607 | 17748 |
| 100 MiB | 180.708 | 3.173 | 10.938 | 342.660 | 124008 |

No old-editor baseline exists in main; these numbers do NOT prove a speedup.
Terminal/network writes, syntax/LSP, very long single lines and cold disk are
not included. Search/substitution can still do linear UI-thread work.

## Remaining requirements / limitations

* Viewport lexical syntax highlighting now supports Typst and basic Rust,
  Python, JS/TS and C/C++ tokens, with a 256-line bounded cache per buffer and
  version invalidation. It is NOT Tree-sitter/semantic highlighting; multiline
  strings/comments and embedded Typst code context are approximate.
* Clickable retained-buffer tabs and divider dragging are implemented; arbitrary
  editor windows and layout persistence are not. More real-terminal mouse coverage
  is still needed.
* No automatic initial source pane, source diagnostic jump or new bidirectional
  source/PDF sync; existing compiler source maps remain unchanged.
* Vim coverage is a subset, not full Neovim. Visual Block change/paste semantics,
  registers +/clipboard, nested macro invocation, multi-count text objects,
  mapping configuration file, operator+find/%, exact word-motion edge cases and
  `==` are incomplete. `==` explicitly advises :format rather than pretending
  to indent. Combining graphemes are handled for h/l, not all edit operations.
* Basic completion acceptance (including UTF-16 textEdit), default snippet expansion,
  and definition/reference navigation are implemented. No snippet tabstop navigation,
  additionalTextEdits, rename, code-action
  application, workspace edits, automatic restart, continuous LSP sync or incremental
  didChange. Diagnostics can be requested explicitly; no persistent gutter.
* Format-on-save is enabled by default for known file extensions. Disable with
  `:set noformatonsave`, enable with `:set formatonsave`. Missing/failed external
  formatter refuses the save with a message; no silent fallback. Stale results
  are neither applied nor saved. Tested with actual rustfmt. No formatter
  configuration file or LSP formatting. Formatting
  currently replaces the whole Rope text in one undoable transaction and does
  not preserve cursor mapping. File saves lack external-change conflict checks.
* History is unbounded (upstream History); per-file input limited to 256 MiB.
* LSP and IO use separate workers; a slow LSP request no longer delays save/open.
  Root detection/server capabilities still need extension.
* No actual SSH/tmux visual certification for the new source pane.

## Interaction fixes verification

Pointer row selection in tree/Recent, clickable/hovered welcome workspaces,
Visual-mode mouse dragging, clickable tabs, Ctrl+PageUp/PageDown and draggable
pane boundaries were added. Ctrl+arrows resize again; Ctrl+hjkl retains focus
navigation. A release build with typst+editor passed. Native editor PTY smoke
passed. Real Tinymist 0.15.10 (Typst 0.15.1) returned non-null hover, definition,
signature and completion responses; Typstyle 0.15.2 formatted a test source.
Both tools installed in ~/.local/bin in this environment. Rustfmt save/undo tests
passed. Terminal TestBackend snapshots were inspected, not a real GUI screenshot.
Full all-features run: 68 passed, 5 failed (existing PDF link/annotation geometry
suites), 4 ignored at that point. Strict Clippy still reports existing warnings in
LSP/Vim/main/tests/links; it is not a clean lint claim.
