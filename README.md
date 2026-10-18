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
| `h` / `l`, Left / Right, `p` / `n` | Previous / next page |
| PageUp / PageDown, Space | Previous / next page (Space: next) |
| `j` / `k`, Down / Up | Vertical scroll |
| `H` / `L` | Horizontal scroll |
| `+` / `-` (also `=` / `_`) | Zoom, 50–300% of pane width |
| `z` | Reset zoom and scroll |
| `g` / `G`, Home / End | First / last page |
| Digits + Enter | Jump to page (one-based) |
| Backspace / Esc while entering digits | Edit / cancel page input |
| `r` | Reload PDF |
| `b` | Back to previous internal-link location |
| `q`, Esc with no page input | Quit |

One status bar shows the requested page, zoom, loading or error state. While
loading, **the previous image remains visible** and is explicitly labelled as
previous: do not mistake it for the requested page.

## PDF hyperlinks

Move the mouse over a PDF link to preview its destination in the cyan status
bar. Left-click follows it; moving alone never opens a browser. Internal links
support explicit page references and named destinations (`XYZ`, `Fit`, `FitH`,
`FitV`, `FitR`). `b` restores the previous page, zoom and scroll (64-entry history,
cleared on document reload). Fit/zoom requests respect Leafview's 50–300% limits.
External `http`/`https` links open asynchronously using `xdg-open` (`xdg-utils`
package). JavaScript, Launch, file URLs and other PDF actions cannot execute.

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
  Structured scopes prevent detached jobs or an accumulating prefetch backlog.
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
