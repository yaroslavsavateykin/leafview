# Migration report

## Evidence

Current local `yaroslavsavateykin/nvim` checkout and installed AstroNvim 5 runtime
were inspected; no fetch, branch switch, reset or clean. Source indexing covers
init, polish, lazy setup, lockfile and every Lua file (4300 lines indexed).
`lua/community.lua` returns `{}`; `lua/community/` does not exist. An installed
community-origin plugin is not treated as proof an inactive import is enabled.

Both real PTY Neovim and headless Neovim were run with the user's init.lua,
VeryLazy and InsertEnter lifecycle, Rust/Python/Typst/Markdown/config fixtures.
Rust-analyzer, basedpyright, tinymist and null-ls were observed in relevant buffers.
Global and local maps in n/i/v/x/s/o/c/t/l and verbose map dumps were collected.
PDF fixture is invalid and some hooks retain prior filetype; this is explicitly
not a verified PDF plugin interaction. OpenCode/Marimo/DAP/terminal interactive
sessions were not launched, so their later dynamic maps remain unverified.

Inventory: **619 unique observed mapping records**, **72 separately documented
builtin grammar entries**, **37 context/source-variant groups**. Conservative
exact correspondence identifies **16 observed records** matching implemented
registry actions; **603 have no exact verified mapping** in the inventory. These
are record counts, not unique physical keys or a claim all 603 actions are absent.
Many native Vim capabilities already existed and remain supported by fallback;
their complete equivalence is not counted without individual tests.

## Implemented

- Command metadata registry, stable IDs, availability, source/category/description;
  shared App::execute_command API for bindings and palette. JSON args/count travel
  through Invocation; handlers consume only documented arguments/counts.
- Context/mode tries, sequences, Leader/LocalLeader, numeric counts, timeout,
  Escape, prefix hints, overrides, consuming disable tombstones, error validation.
- Five selectable presets (vim/neovim/astronvim/leafview-default/custom).
  They share compatibility foundations; they are not five complete independent
  replications of their respective editors.
- Global/workspace/project TOML layers, scalar-presence-aware merge, hot reload,
  last-good config on errors, temporary Rust override API.
- F9 palette, F10 Keyboard Shortcuts, recording/conflict confirmation, disable,
  restore command overrides, category/scope controls, preset cycling, import/export.
- Existing Vim grammar and helix-core transactions reused; no rewrite.
- Source, Explorer, PDF, completion/tool windows routed by focus.

## Checks

- Build `--features typst,editor`, default cargo check, all-features check: pass.
- New keybinding tests: 8 passed, benchmark separately passed.
- 10,001-binding debug microbenchmark: 100,000 simple lookups ~56.7 ms on this host;
  includes Instant calls/cloning and is not a GUI latency guarantee.
- Editor tests: 31 passed, external tests separately retained.
- Existing completion/editor PTY smoke: pass.
- New keybindings PTY smoke: TOML `x` remap, leader save, WhichKey, F9 palette,
  F10 settings all pass.
- Keyboard UI TestBackend snapshot rasterized and inspected.
- Full combined unit run: 93 passed, 5 failed, 10 ignored. Failures remain PDF
  link/annotation geometry tests, not concealed as a green full suite.
- Clippy all features/all targets finishes successfully with warnings in existing
  LSP/Vim-tests/main/links and earlier Typst source-map code; fmt/diff check pass.

## Incomplete requirements (explicit)

This is a working central layer, **not completion of all 17 requested stages**:

1. Legacy main/Vim/dialog handling still exists for compatibility. Not every
   LeafView action or Vim production has a stable registry ID. Not every action
   can be fully rebound/disabled without a remaining fallback route.
2. No new complete motion/operator grammar was built. Existing Select is absent;
   Search/Operator Pending are exposed as binding modes atop existing parser,
   not full new independent editor modes. Operators like g~/gu/gU and counted
   text objects require further parity work.
3. There is no embedded terminal, Git engine, dynamic executable plugin loader,
   rename/code-action implementation, Marimo/OpenCode session integration.
4. No buffer-local binding config or arbitrary availability expression DSL;
   project paths are chosen at startup, not recalculated after every workspace switch.
5. TOML config editing/import uses bounded practical UI paths but performs small
   filesystem writes/reloads on UI thread; heavy config parsing is not workerized.
6. WhichKey has keyboard interaction but no mouse selection/group labels or delayed
   reveal; UI filtering is rebuilt per draw, not incrementally cached.
7. Keyboard UI is TUI only: no native GUI, mouse buttons, generic args editor,
   arbitrary import dialogs or full old/new binding column model.
8. Plugin registration is a Rust API plus application handler, not dynamic plugins.
   External preset files aren't discoverable packages yet.
9. Partial hot-path allocations remain (context/string keys and Invocation clones).
   No claim of allocation-free dispatch or exhaustive large-file latency profiling.
10. Physical strokes require a future GUI/terminal protocol frontend. Some terminal
    modifier distinctions cannot be recovered from crossterm events.

No unsupported command is silently turned into shell execution. Registry IDs not
known at load are rejected. The audit JSON is evidence plus explicit limitations,
not a promise every lazy/dynamic plugin state was activated.
