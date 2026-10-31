# Development

[Documentation](README.md) · [Project](../README.md)

## Architecture and resource limits

- Main thread: input, navigation, status and terminal writes only. Initial open
  is validated before terminal entry; subsequent reloads run in a worker.
- Renderer: one latest-request mailbox, monotonic generation IDs, bounded result
  slot. Obsolete Poppler jobs are killed and reaped on a new request or shutdown.
- Two Rayon workers prefetch up to three pages ahead and three behind using
  independent child processes. Nearest pairs complete before farther pairs start.
  The sliding seven-page window releases obsolete speculative rasters; prefetch
  stops at the existing memory budget and never evicts the current/nearby pages.
  Hovered internal destinations are warmed as a separate latest-wins, single
  in-flight job. Structured scopes prevent detached jobs or an accumulating
  prefetch backlog.
- Raster LRU: at most eight entries **and 96 MiB** of pixel data, keyed by
  document revision/page/width. Scroll crops cached pixels, not a new PDF render.
- A single encoding worker builds fixed ratatui-image `Protocol`s off-thread.
  One running job plus one latest replacement; generation checked before display.
  This intentionally replaces the prototype's `ThreadProtocol`: a fixed encoded
  protocol allows the old image to remain visible until its replacement is ready.
- Every worker is owned and joined. Subprocess RAII kills/reaps even on errors.
- Poppler: 12 s wall deadline; Linux `prlimit` enforces 512 MiB address space,
  90 MiB file size and 12 s CPU. Output is spooled to anonymous temporary files,
  checked during execution, not accumulated in an unbounded pipe.
- Snapshot cap: 512 MiB. PNG cap: 80 MiB; decoder allocation limit: 96 MiB;
  raster width: 100–3400 pixels; decoded height: at most 12000 pixels.
- Visible encoded viewport: at most 3400 × 2400 pixels. Very large terminals are
  therefore centered rather than allowed to create unlimited encoding buffers.

The cache bound is **not a total RSS promise**: in-flight decoded/cropped images,
encoded terminal strings, allocator overhead, the terminal and Poppler processes
consume additional memory. Pathological PDFs may be rejected by the limits.
No PDF text search, selection, passwords or outlines are implemented.

## Verification and development

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
python3 scripts/pty-smoke.py
```

The Python PTY test needs only the standard library and exercises fallback
output, rapid input, auto-reload, tiny-window resize, shutdown and TTY restoration.
It is not a visual graphics test. CI runs it on Ubuntu 24.04.

Optional synthetic performance test (Typst + Python Pillow required):

```sh
python3 scripts/make-bench-fixtures.py
cargo test --release profile_synthetic_pdfs -- --ignored --nocapture
```

See [VERIFICATION.md](archive/VERIFICATION.md) for actual results, remaining gaps and a
repeatable real-terminal checklist. `scripts/publish-private.sh` is an optional
maintainer helper, never required for installation or daily use; it only creates
private repositories and refuses to modify an existing remote repository.
The Leafview repository itself is now public by the author's choice.

## Optional feature checks

```sh
cargo check --locked --all-features
cargo test --locked --all-features
cargo clippy --locked --all-features --all-targets -- -D warnings
cargo build --locked --release --features typst,editor -j 2
python3 scripts/helix-editor-smoke.py
python3 scripts/completion-smoke.py
python3 scripts/keybindings-smoke.py
```

Smoke scripts expect `target/debug/leafview`; build it with the required features
first. Completion tests need real Tinymist on PATH; other external tests may need
Neovim, language servers or fixture generators. Never run a smoke test against
personal documents. Check [current showcase verification](showcase-verification.md)
for observed test failures; archived reports are not a green CI guarantee.

The workflow currently checks default and Typst builds on Ubuntu 24.04. It does
not cover the full editor feature matrix. No CI badge is shown while the latest
observed public run is failing. Contributions should not mask failures by
silently skipping suites.

## Source map

- `main.rs`, `commands.rs`: application orchestration and command dispatch
- `files.rs`, `picker.rs`, `welcome.rs`: browser, independent pickers and history
- `document.rs`, `links.rs`, `view.rs`: immutable snapshots, annotations and viewport
- `render.rs`, `workers.rs`, `process.rs`: bounded work and subprocess lifecycles
- `kitty.rs`, `terminal.rs`: graphics transport, detection and terminal state
- `typst_*`, `compile_*`, `diagnostics.rs`: compilation, mappings and settings
- `editor/`: Helix buffer adapter, modal input, rendering and external tool workers
- `keybindings/`: command registry, layered mappings and searchable shortcut UI

For public media, use [scripts/showcase](../scripts/showcase/README.md). Keep
large build intermediates and private state out of Git. The optional maintainer
publishing helper is not needed for builds or contributing.
