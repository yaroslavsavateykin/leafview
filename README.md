# Leafview

**Leafview** is a minimal terminal PDF viewer written in Rust, designed for everyday use in `tmux` while editing documents (especially Typst). No browser or separate GUI window is required.

It uses **Poppler** for PDF rasterization, **Rayon** for background prefetch of adjacent pages, and **ratatui-image** for graphics-protocol handling (Kitty, Sixel, iTerm2 and halfblocks fallback).

## Install (Ubuntu 24.04+)

```bash
sudo apt install poppler-utils coreutils
# Install stable Rust: https://rustup.rs/
cargo install --path .
```

Or compile from source:

```bash
cargo build --release
./target/release/leafview ~/Documents/document.pdf
```

Run directly:

```bash
leafview paper.pdf
```

### Tmux

Use a modern graphics-capable terminal (Kitty or Ghostty recommended). Recent tmux and `ratatui-image` can negotiate Kitty graphics passthrough. If the terminal does not support inline images, halfblock fallback is available, but is slower and lower fidelity.

Use `tmux -V` to check your version. Graphics support depends on **both** tmux and the outer terminal. `ratatui-image` may enable graphics passthrough per pane while querying terminal capabilities. Do not globally enable passthrough just to run Leafview without understanding the security implications.

## Keys

| Key | Function |
| --- | --- |
| `h` / `l`, `←` / `→`, `p` / `n` | Previous / next page |
| `PageUp` / `PageDown`, `Space` | Previous / next page |
| `j` / `k`, `↓` / `↑` | Scroll down / up |
| `H` / `L` | Scroll left / right |
| `+` / `-` | Zoom in / out |
| `z` | Reset zoom and scroll |
| `g` / `G` | First / last page |
| `123` then Enter | Go to page 123 |
| `r` | Reload file |
| `q` / `Esc` | Quit |

The status line stays at the bottom. Automatic reload watches the containing directory, so it also detects Typst's atomic replacement of PDFs. It retains the previously opened PDF until the replacement is successfully validated.

## Design

- UI never calls Poppler synchronously while processing navigation.
- A one-slot mailbox replaces obsolete render requests instead of growing a job queue.
- Rendering jobs carry generations; outdated pages cannot replace newer results.
- Cached page rasters are keyed by document revision, page number, and raster width.
- The four-entry raster LRU makes scrolling on a page cheap after its initial render.
- A two-thread Rayon pool asynchronously pre-renders neighboring pages, using separate Poppler subprocesses (no shared native PDF context).
- PNG decoding and crop generation occur off the UI thread.
- `ratatui-image::ThreadProtocol` performs image encoding/resizing in a separate thread.
- Raster width is capped to avoid unbounded allocations. Very large images are rejected.

## Testing

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Manual tmux validation:

1. Open Kitty or Ghostty and run `tmux`.
2. Open `leafview test.pdf` in one pane.
3. Rapidly navigate pages, zoom and scroll.
4. Split the pane; resize panes repeatedly.
5. Detach and reattach tmux. Exit with `q` and verify terminal cleanup.
6. Regenerate the PDF with Typst while Leafview remains open.

## Limitations (v0.1)

- Requires Linux/GNU `timeout` and Poppler command-line tools, not a pure-Rust PDF engine.
- `tmux` graphics fidelity, positioning, and cleanup depend on the terminal and library versions; this must be tested on real terminals.
- No links, search, text selection, or document outlines yet.
- Rasterization has an upper bound of 12 seconds per page, so very complex pages can fail with an error rather than freeze indefinitely.
- This initial source release is not claimed to have passed an actual Cargo build unless CI says so.

## License

MIT. Dependencies have their own licenses; Poppler is GPL-licensed. Leafview executes the installed Poppler CLI as a separate process.
