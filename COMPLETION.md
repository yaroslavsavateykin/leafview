# LeafView completion: Neovim compatibility audit

## Reference inspected

Local checkout `/home/yaroslav/.config/nvim` has origin
`git@github.com:yaroslavsavateykin/nvim.git`. No remote changes were fetched.
Runtime inspection used `nvim --headless`, loaded `blink.cmp` through lazy.nvim,
and inspected `blink.cmp.config`, effective highlights, and mini.icons.
Installed AstroNvim reports tag `v5`; blink.cmp reports `v1.10.2`.

Relevant actual sources:

- `lua/lazy_setup.lua`: AstroNvim `^5`, community + plugins imports.
- AstroNvim `lua/astronvim/plugins/blink.lua`: actual mappings and completion overrides.
- AstroNvim `lua/astronvim/plugins/luasnip.lua`: active LuaSnip integration.
- blink.cmp `lua/blink/cmp/config/`: version-specific defaults.
- User `lua/plugins/user.lua`: **inactive** (`if true then return {} end`).
  Its LuaSnip extension and lsp_signature example are NOT effective configuration.
- User Python configuration uses basedpyright, not pyright.

Evaluated runtime: `preselect=false`, `auto_insert=true`, `ghost_text=false`;
menu columns are kind_icon + label/label_description (not kind/source columns).
Rounded borders, menu height 10, min width 15; docs width up to 80, height up to
20, automatic documentation delay 0 ms (update delay 50 ms). Docs are associated
with the selected item, not an automatically selected first item.
Blink signature provider is disabled; AstroLSP/Neovim handles signature help.

Effective Astro mappings:

| Keys | Behavior |
|---|---|
| Tab / Down / Ctrl-N / Ctrl-J | next selection; Tab previews insertion |
| Shift-Tab / Up / Ctrl-P / Ctrl-K | previous selection |
| Enter | accept selected item; otherwise normal Enter |
| Ctrl-E | hide and restore preview |
| Ctrl-Space | show menu, then toggle selected documentation |
| Ctrl-U / Ctrl-D | documentation scroll |
| Ctrl-B / Ctrl-F | documentation scroll inherited from default preset |
| Tab / Shift-Tab with no menu | snippet forward/backward, then fallback |
| Ctrl-N / Ctrl-P with no menu | show menu |
| Tab after a non-whitespace character | show menu |

Escape is not a blink hide mapping: in Insert mode it also leaves Insert mode.
LeafView keeps F1/hover, F12/definition and Ctrl-S/signature as existing LeafView
commands; these are not claimed to be bindings from the reference Neovim.
Ctrl-J/K are routed to completion before LeafView's pane navigation while the
completion menu is open.

## Requested LeafView behavior override

The follow-up request explicitly changes the reference's `auto_insert=true`:
LeafView now defaults to **`auto_insert=false`**. Tab/arrows/Ctrl-N/P change only
selection; Enter inserts. Browsing does not dirty the buffer or change its version.
Set `auto_insert=true` explicitly only to restore the Neovim preview behavior.
Ctrl-P is reserved for completion in Insert mode, not intercepted by the file picker.

Completion is filtered/ranked before truncation with Helix's nucleo fuzzy matcher,
`filterText`, exact/prefix boosts and `sortText` tie-breaking. This fixes tinymist's
unfiltered `import/include` items ahead of `page` and preserves the selected item
when the server replaces a locally populated list. Resolve replies must still
match the item that was requested; navigation no longer cancels unrelated requests.

Signature help has independent state, coexists with completion, updates while
editing/moving inside calls, respects activeSignature/activeParameter (including
UTF-16 parameter ranges) and highlights the current parameter and its docs.
Bounded context tracking ignores nested collections, strings and comments.
`signature_auto_show`, `signature_delay_ms` and `auto_pairs` are configurable.
Typing an opening delimiter inserts its mate; typing the closing delimiter skips
it. This also lets tinymist produce signatures for calls under construction.

## Implementation

Built on existing helix-core Buffer transactions and existing bounded LSP worker;
no replacement editor or second LSP implementation. New modules:

- `src/editor/completion.rs`: configuration, selection, preview, resolve, snippets.
- `src/editor/completion_render.rs`: cursor-relative menu and adjacent documentation.
- `src/editor/completion_sources.rs`: bounded buffer/snippet providers and worker-side paths.
- `src/editor/completion_tests.rs`: state, edits, Unicode, rendering, external servers.

The UI remains nonblocking while a worker requests completion/resolve/signature.
Typing cancels pending work via a shared flag and sends `$/cancelRequest`;
late replies are ignored using path/version/cursor validation. Requests are still
serialized within the existing LSP worker. Buffer candidates use a bounded
201-line neighborhood; directory enumeration is off the UI thread and bounded.
LSP completion supports arrays/lists, itemDefaults, UTF-16 ranges, InsertReplaceEdit
(insert range), deferred item resolution and additionalTextEdits in a single
Helix transaction. Snippet parsing/rendering/tabstop mapping use helix-core.
Acceptance is a separate undo step. Simple kind-based automatic parentheses are
supported. Ghost text is dimmed terminal text, disabled by default.

## User configuration

`$XDG_CONFIG_HOME/leafview/completion.json` (or `~/.config/leafview/completion.json`).
Override path with `LEAFVIEW_COMPLETION_CONFIG`. JSON allows partial objects;
unknown fields and invalid values produce a startup editor message rather than
silently accepting misspelled options. See `examples/completion.json`.

Sizes and RGB colors, border shape, debounce, preselect, preview, ghost text,
auto brackets, documentation auto-show, provider selection and key bindings are
configurable. Key arrays replace the corresponding action's defaults. A binding
uses `C-n`, `Tab`, `S-Tab`, `Enter`, `Up`, `Down`, `Esc`, or `C-Space` notation.
Snippets use `all` or extension (`rs`, `py`, `typ`) and LSP snippet strings.
A Nerd Font is required to display the actual mini.icons glyphs from the reference.
Python now defaults to `basedpyright-langserver --stdio`; LEAFVIEW_LSP remains
available for overriding the server command. Add Mason's bin directory to PATH
if servers are installed by Neovim rather than system-wide.

## Checks performed

- `cargo check` and `cargo build --features editor`: pass.
- `cargo test --features editor editor::`: 29 passed, 6 explicitly external tests ignored.
- With Mason on PATH, `cargo test --features editor editor:: -- --ignored`:
  6 passed: Rust/Python/Typst completion AND active-parameter signature tests,
  real Typst `#page` filtering/non-mutating navigation/automatic signature,
  automatic Typst completion, Typst LSP/formatter and large-file benchmark.
- Ratatui TestBackend rendering checked at 120x30 and narrow sizes. A rasterized
  cell snapshot was inspected; this is not a real-terminal pixel parity test.
- Full test suite: 65 passed, 6 ignored, five failures in PDF link/annotation geometry tests
  (`link_tests` and `render::tests::annotation_geometry_matches_poppler_pixels`).
  These are outside completion; no baseline checkout was reset to establish origin.
- `cargo check --all-features` and `cargo build --features typst,editor`: pass.
- `scripts/helix-editor-smoke.py` and new `scripts/completion-smoke.py`: pass
  with the combined typst,editor build. The latter runs real tinymist in a PTY,
  checks Tab/Ctrl-P navigation leaves the saved code unchanged, and verifies an
  automatically opened signature after typing `(`.

## Remaining differences — not a claim of exact parity

This is a substantial completion implementation, **not full blink/LuaSnip parity**:

- Uses Helix/nucleo fuzzy ranking rather than blink's matcher/frecency database;
  no exact source score offsets or fallback scheduling. Path candidates currently
  take precedence over LSP for paths.
- No Lua runtime, LuaSnip dynamic nodes, direct friendly-snippets import, choice
  cycling, or mirrored-placeholder synchronization. Standard LSP snippet parsing
  and tab navigation work; choices initially use their first value.
- Markdown rendering handles headings, bold, inline code and fenced code with the
  existing lexical highlighter; it is not Neovim Tree-sitter or a full Markdown
  renderer (tables, images, rich links and nested constructs differ).
- Signature highlighting uses LSP active parameters, with bounded call-context
  fallback; automatic updates are context-driven rather than exact AstroLSP
  server-advertised trigger scheduling. Very large (>16K chars) call expressions
  and complex language-specific syntax may exceed the context tracker.
- Automatic parentheses are kind-based only (not semantic-token resolution).
  Completion commands, commitCharacters and insertTextMode are not implemented.
- Optional `auto_insert=true` preview still uses real Buffer transactions rather
  than blink's extmarks. The default non-inserting browsing has no such mutations.
- Debounce is configurable; documentation resolve scheduling is immediate on tick,
  and existing 10/15/20-second LSP timeouts are not user-configurable here.
- Cancellation during initial LSP initialization still waits for initialization;
  cancellation of established requests is polled every 10 ms.
- Popup placement is bounded and chooses below/above, east/west/vertical docs;
  exact north bottom-up ordering, width measurement and scrolloff differ.
- Real language tests validate completion and active signature parameters;
  Typst additionally has a real PTY flow. This does not certify every possible
  resolve/import/snippet/signature combination or pixel-identical Neovim output.
