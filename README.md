# Leafview

A small Linux terminal PDF viewer for a Neovim/Typst + tmux workflow.
Poppler renders real PDF pages; ratatui-image handles Kitty graphics, Sixel,
iTerm2 and the halfblocks fallback. No browser or GUI PDF viewer is needed.

Vibe-coded, inspired by [Fancy Cat](https://github.com/freref/fancy-cat).
Works on my Ubuntu/Ghostty/tmux setup; other setups may vary.

## Installation (Ubuntu 24.04+)

```sh
sudo apt update
sudo apt install poppler-utils util-linux
# Install current stable Rust with https://rustup.rs/ if necessary.
cargo install --locked --path .
leafview document.pdf
```

Or `cargo build --locked --release` and run `./target/release/leafview document.pdf`.
Use a current stable Rust toolchain: the lockfile includes recent Ratatui/image
versions, not the old Rust shipped by some distribution packages.

## Keys

| Key | Action |
| --- | --- |
| `h` / `l`, Left / Right | Horizontal scroll within the page |
| `j` / `k`, Down / Up | Vertical scroll within the page |
| Shift + arrows or `H` / `J` / `K` / `L` | Previous page: Left / Up / H / K; next page: Right / Down / L / J |
| `p` / `n`, PageUp / PageDown, Space | Previous / next page (Space: next) |
| Wheel/trackpad up / down | Zoom in / out by 5 percentage points around the pointer |
| Wheel/trackpad right / left | Next / previous page (at most one turn per 250 ms) |
| `+` / `-` (also `=` / `_`) | Zoom by 5 percentage points, 50–300% of pane width |
| `z` | Reset zoom and scroll |
| `g` / `G`, Home / End | First / last page |
| Digits + Enter | Jump to page (one-based) |
| Backspace / Esc while entering digits | Edit / cancel page input |
| `r` | Reload PDF |
| `b` | Back to previous internal-link location |
| `q`, Esc with no page input | Quit |

Trackpad gestures work when the terminal/tmux forwards SGR wheel events;
LeafView cannot read raw touchpad/pinch gestures. Zoom uses small discrete steps
coalesced by the existing renderer, not pixel-level animation. At page edges or
the 3400-pixel raster cap the pointer anchor is clamped. Arrows and `hjkl` scroll
within the page; hold Shift to turn pages instead. Horizontal gesture direction follows
the ScrollLeft/ScrollRight events supplied by your natural-scroll settings.

One status bar shows the requested page, zoom, loading or error state. While
loading, **the previous image remains visible** and is explicitly labelled as
previous: do not mistake it for the requested page.

## Typst Live Preview

```sh
leafview typst main.typ
leafview typst "./notes with spaces/lecture.typ"
leafview typst letuchka.typ --root ../..
leafview typst main.typ --input edition=teacher --font-path ./fonts -j 2
```

Typst support is opt-in (ordinary PDF builds do not include the compiler):

```sh
cargo install --locked --features typst --path . -j 2
```

For a PDF-only installation, use `cargo install --locked --path .`.

LeafView compiles the entry file in an owned background compiler process and
shows the PDF using the **same renderer and graphics policy** as PDF mode.
You do not need to run `typst watch`. Its CLI reports compilation status and
can list dependencies, but does not export source-to-PDF coordinates or the
changed layout region. LeafView instead reuses an incremental compiler world
and reads source spans from that same compilation's layout. The installed `typst` command is checked
at startup; compilation itself uses the pinned **Typst Rust API 0.15.1** bundled
with the `typst` feature, not a potentially different CLI version. This adds substantial
build dependencies and a roughly 47 MiB release executable.

After a save, dependency events are debounced for 200 ms. One compiler is reused
for incremental compilation; at most one job is running and one latest request
is pending. Newer edits invalidate older results. PDF and source coordinates
always come from the same compiled layout. By default successful current builds
**atomically replace `main.pdf` beside `main.typ`**, including an existing PDF.
Compilation and preview still use a private snapshot; publishing only adds a file
copy, not another compile/render. Use `leafview typst main.typ --no-pdf` to keep
output private and leave any existing `main.pdf` untouched. Failed or superseded
builds do not replace the exported file; own PDF writes do not trigger a rebuild.
Compilation errors
keep the previous PDF visible; fix and save again, or press `r` to retry. An
initial error shows a blank placeholder until compilation succeeds.

Changes in included/imported files and compiler-known resources also trigger a
build. LeafView watches dependency **parent directories non-recursively**, so
Neovim atomic saves work without recursively watching your home directory. The
entry file is the root document; the project root defaults to its parent. Set
`--root /path/to/project` or `TYPST_ROOT` for a different root. Relative flag
paths are resolved from the launch directory, like the Typst CLI. Flags may
precede or follow the input; `--flag=value` is supported.

Supported Typst 0.15.1 compile settings: `--root`, repeatable `--input key=value`,
`--font-path` (repeatable/path-separated), `--ignore-system-fonts`,
`--ignore-embedded-fonts`, `--package-path`, `--package-cache-path`, `--cert`,
`--creation-timestamp`, `-j`/`--jobs`, `--features`, `--pdf-standard`,
`--no-pdf-tags`, `--pretty`, `--pages`, `-f pdf`/`--format pdf`.
Standard Typst environment defaults for roots/fonts/packages/cert/timestamp/features
are honored; explicit flags take precedence (font paths add to the configured
search paths). Page selection renumbers source mappings to the exported PDF.
`--diagnostic-format` and `--color` are accepted, but the viewer always displays
sanitized short status messages; `--ppi` has no effect on PDF (PNG export only).
Use `leafview typst --help` or `--version` for help/compiler version.

Not all CLI side effects fit a live PDF viewer: `--deps`, `--deps-format`,
`--make-deps`, `--timings`, `--open`, `--no-serve`, `--no-reload`, `--port`,
non-PDF output formats and a positional output path are **explicitly rejected**,
not forwarded or silently ignored. Use `typst compile/watch` for these exports
or server features. LeafView does not claim full Typst CLI flag compatibility.
`--no-pdf` is a LeafView-specific option, not a Typst compiler flag.

| Key in Typst mode | Action |
| --- | --- |
| `s` | Toggle automatic source synchronization (shown as `sync on/off`) |
| `f` | Follow the last mapped edit manually |
| `r` | Recompile |

The status shows `⏳` while compiling or preparing the image, then `✓` once
loading finishes. Compilation errors appear in a centered reddish bordered
panel with the source path, line/column, source excerpt/caret and available hints.
The last successful PDF stays in memory, temporarily hidden to keep terminal
image placeholders from interfering with the diagnostic panel. A successful
recompile automatically closes the panel and restores PDF display.

Source synchronization compares successful source snapshots, selects the last
changed character/token in the most recently changed source file, then uses
compiler glyph spans to find its new PDF coordinates. Multiple disjoint edits
in one save deterministically select the last change in file order; deletion
uses the nearest surviving mapped token. Repeated spans on different pages,
style-only changes without glyph correspondence and resource-only changes do
not force an arbitrary jump. Manual navigation stays put until a new mapped
source change; zoom is preserved, and a target already visible does not scroll.
Otherwise it is positioned near the upper/left third of the viewport.

Limits: one compile has a 60 s timeout and a 3 GiB address-space cap; source
snapshots are capped at 16 MiB, the map at 500,000 glyph entries. Mapping covers
text/math glyphs, not arbitrary shape/image source expressions. Highly dynamic
macros, generated content and multi-location spans can remain unmapped. Glyph
coordinates indicate a baseline, not an exact selection rectangle. Native
Ghostty+tmux graphics limitations from [AUDIT-2026-10-19.md](AUDIT-2026-10-19.md)
remain historical test findings. Both Typst and PDF mode now negotiate native
graphics by default, including Ghostty+tmux; safe mode is explicit opt-in.

See [TYPST-VERIFICATION.md](TYPST-VERIFICATION.md) for actual tests, measurements
and remaining gaps. Additional automated check: `python3 scripts/typst-smoke.py`.

## PDF hyperlinks

Move the mouse over a PDF link to preview its destination in the cyan status
bar. Left-click follows it; moving alone never opens a browser. Internal links
support explicit page references and named destinations (`XYZ`, `Fit`, `FitH`,
`FitV`, `FitR`). `b` restores the previous page, zoom and scroll (64-entry history,
cleared on document reload). Fit/zoom requests respect Leafview's 50–300% limits.
External `http`/`https` links open asynchronously using `xdg-open` (`xdg-utils`
package). JavaScript, Launch, file URLs and other PDF actions cannot execute.

All annotations are indexed once per document snapshot, so each link's target
page is known without re-parsing. Hovering an internal link for ~150 ms warms its
destination page into the background raster cache (one in-flight job, cancelled
on leave/shutdown), making the click follow the link without waiting for Poppler.

Hit testing uses terminal **cells**, not mouse pixels: the cell center must be
inside one link with at least half its area covered. Ambiguous cells, tiny links,
padded image edges and stale/loading images cannot be activated. Zoom in if a
small reference cannot be selected. Rotation, nonzero MediaBox/CropBox origins,
scroll and image centering are included in the transform. The viewer still
renders the MediaBox, matching its existing Poppler configuration; CropBox is
not silently substituted. Nonrectangular QuadPoints links are currently skipped.

Hover changes only the status bar: no Poppler render, image encoding or overlay
is performed. This avoids corrupting Kitty placeholders in tmux. Pixel mouse mode
1016 and a hand cursor are not implemented. Mouse capture is restored on exit.
Tmux normally forwards application-requested mouse events; depending on your
configuration you may need to enable mouse forwarding yourself. Leafview does
not modify tmux mouse settings. Hold your terminal/tmux selection modifier when
you need text selection rather than application mouse input.

Annotations are parsed once per immutable document snapshot in a cancellable,
resource-limited helper process (lopdf, no Tokio/GLib). Only extracted geometry
and actions survive parsing, with caps of 100,000 links, 4096 annotations/page and
32 MiB serialized metadata. Invalid/encrypted/oversized metadata disables links
without disabling PDF rendering. Eager extraction can add document-load latency
on very large PDFs; it never runs on mouse movement. Malformed destinations,
FitB-family, remote GoTo and nonrectangular actions are unsupported, not guessed.

## Crash protection and graphics mode

LeafView negotiates native graphics by default, **including Ghostty inside
tmux**. There is no emulator blacklist or forced resolution downgrade for this
combination. Kitty uses lossless PNG transport at the existing raster resolution.
Halfblocks is used only when explicitly requested or graphics negotiation fails.

Earlier tests observed graphics transfer corruption under intensive tmux pane
operations (see AUDIT-2026-10-19.md); the cause is not established and is not a
reason to assume this combination always crashes. Removing the automatic
restriction does not constitute proof that every terminal/protocol issue is fixed.

Override the automatic selection when needed:

```sh
LEAFVIEW_GRAPHICS=safe leafview document.pdf    # explicitly request low-resolution halfblocks
LEAFVIEW_GRAPHICS=native leafview document.pdf  # negotiate native graphics (same as auto)
LEAFVIEW_DEBUG=1 leafview document.pdf         # inspect selected backend
```

`LEAFVIEW_GRAPHICS=auto` (default) queries terminal capabilities without a
Ghostty/tmux exception. Explicit safe mode deliberately skips graphics
capability negotiation. Navigation/render requests are coalesced to at most
one every 75 ms during key repeats; the final viewport is always scheduled.
Ctrl-C is handled as an ordinary clean exit, like `q` or Esc. A system-level
SIGKILL, OOM kill, or a crash inside Ghostty itself cannot be handled by an
application's cleanup code.

## Terminals and tmux

Ghostty 1.3 development build with tmux 3.6 has been visually tested, including
split panes, navigation, Typst reload and detach/attach with focus events.
Kitty itself, Sixel and iTerm2 have **NOT TESTED** status; see VERIFICATION.md.

Capability negotiation is delegated to ratatui-image 11.0.8. `TMUX` alone is not
proof of graphics support. The library may enable `allow-passthrough` on the
current pane during detection. Kitty Unicode placeholders constrain images to
the pane. Leafview reuses its own process-specific image ID and deletes only that
image on normal exit; it does not delete other panes' graphics.

For automatic repaint after attaching a **new outer terminal**, enable focus
forwarding in your tmux configuration:

```tmux
set -g focus-events on
```

Leafview requests focus events and retransmits its page when focus returns.
Without forwarded focus/resize events, tmux can replay placeholders into a new
terminal that has never received the image data: press `r` to restore it.
This is a protocol/lifecycle limitation, not evidence that the PDF is damaged.

Do not force a backend just because a tmux session exists. If detection cannot
establish graphics support, halfblocks provide a lower-resolution fallback.

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

See [VERIFICATION.md](VERIFICATION.md) for actual results, remaining gaps and a
repeatable real-terminal checklist. `scripts/publish-private.sh` is an optional
maintainer helper, never required for installation or daily use; it only creates
private repositories and refuses to modify an existing remote repository.
The Leafview repository itself is now public by the author's choice.

## Troubleshooting

- Missing command: install `poppler-utils util-linux`; ensure `pdfinfo`,
  `pdftoppm`, `prlimit` are on PATH.
- Poppler failure: PDF may be incomplete, damaged, encrypted, oversized or exceed
  the renderer deadline/resource cap. Try `pdfinfo document.pdf` outside Leafview.
- Reload error during compilation: old PDF stays visible; save again or press `r`.
- Blank image after attaching elsewhere: enable tmux focus events, select the pane
  or press `r`. Check outer-terminal graphics support and tmux passthrough policy.
- Low-resolution colored blocks: capability negotiation selected halfblocks.
- SIGKILL cannot run cleanup in any application. If externally killed, use
  `reset` if necessary; normal `q`/Esc exit restores the terminal.

## License

MIT. Dependencies retain their own licenses. Poppler is GPL-licensed and executed
as an installed external CLI process.
