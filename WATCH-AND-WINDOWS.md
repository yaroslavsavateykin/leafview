# Typst watch and editor windows

## Compilation diagnosis

LeafView already embeds a persistent Typst compiler process (`--typst-compiler`),
not a fresh `typst compile` command per save. Its `World`, font store, file store
and comemo cache survive across passes. `FileStore::reset()` is the intended
watch behavior in typst-kit 0.15.1: updated sources are edited in place using
`Source::replace`, not recreated from scratch.

The actual bug was cancellation: a newer filesystem event changed the desired
generation, `CompilerProcess::compile` killed the running process, and the worker
then dropped its cached World. Frequent saves during compilation could therefore
repeatedly turn warm watch passes into cold starts.

Changes:

- Superseding edits no longer kill the compiler. Finish the current pass, keep its
  cache, discard stale output and compile the latest generation. Explicit app
  shutdown and the existing 60-second timeout still terminate the child.
- Do not discard the compiler on PDF-opening/publication errors unrelated to the
  compiler's health.
- Coalesce duplicate filesystem events; ignore known source saves with identical
  bytes. The existing 200 ms debounce remains.
- Cache syntax-span lookups while building the per-glyph source map, avoiding
  repeated syntax-tree lookup and path resolution for each glyph of a span.
- Footer reports `Typst watch`, last compile milliseconds and pass number.
  Compiler replies also carry process ID and iteration for regression tests.

This is watch-style **incremental compilation on disk saves**, not compilation of
unsaved editor buffers. PDF export, link extraction, snapshot loading and raster
rendering still have costs after Typst layout. It is not a guarantee of sub-100ms
updates for large projects. Time in the footer measures the compiler pass, not
end-to-end render latency. Large document performance should be profiled with the
actual project before attributing all remaining latency to compilation.

## Tool windows

- Hover is rendered from original LSP Markdown rather than list-formatted text;
  no `›` prefixed to every line or mangled code fence.
- Documentation is capped at half the editor height; argument help is a compact
  strip (up to a third of the height), with active parameter highlighting.
- Tool titles show focus/close shortcuts; focused borders are highlighted.
- F7 cycles Documentation -> Arguments -> Source (only available tools).
- F8 closes the focused tool; from Source it closes documentation first, then
  arguments. `q` closes a focused tool, Escape returns focus to Source.
- In a focused tool: arrows/j/k, PgUp/PgDn, Ctrl-U/D/B/F scroll without editing.
- Regular typing returns to Source. Signature dismissed in a call remains hidden
  until leaving/changing call context, rather than reopening on every keystroke.
- Existing completion keys remain for completion; F7/F8 are configurable as
  `keys.window_next` / `keys.window_close` in completion.json.

## File tabs and panes

File tabs have a right-aligned `×` mouse target. Ctrl-Q closes the current file;
unsaved files are protected (`:bd!` is still explicit discard). Ctrl-PgUp/PgDn
switch file tabs. Switching/closing a file clears obsolete LSP tool state.

Existing Source/PDF panes remain: F6 toggles focus, Ctrl-W then h/j/k/l navigates,
Ctrl-W then c hides Source, Ctrl-W then v/s changes vertical/horizontal layout.
F4 reopens Source. These are pane commands, distinct from F7/F8 tool-window focus.
There is still one source view with multiple file tabs, not arbitrary multiple
independent source splits.

## Checks

- `cargo check --all-features`: pass.
- Editor unit/render tests: 31 passed (6 explicit external tests ignored).
- Compiler integration tests: 2 passed; same PID across edits and error recovery.
  On the small 8-page fixture one measured debug run was cold 84 ms, edit 28 ms,
  error recovery 43 ms. Not a benchmark of the user's project.
- CompilerProcess persistence test: same PID and sequential iterations across
  generations passed with a 3000-line fixture.
- Typst live unit tests: 3 passed.
- Hover cell render inspected as `/tmp/leafview-hover-windows.png`.
