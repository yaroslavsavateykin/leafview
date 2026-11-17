# Blank Typst preview: upload lost behind overlays

## Root cause found

KittyImage::render previously embedded the PNG transfer into the first cell of
its Ratatui widget and immediately marked the shared image as uploaded. A later
widget (error panel, picker, WhichKey, settings or other overlay) can overwrite
that cell before the frame diff reaches the terminal. The transfer is then never
written, but cached replays skip it forever. Unicode placements reference an
image that the terminal has never received, producing an empty preview. This is
independent of Typst compiling a correct PDF.

## Fix

- KittyImage::render is now placement-only, with no upload-state side effects.
- KittyImage::upload writes PNG commands directly to the terminal before the
  frame is drawn and marks the slot/epoch as uploaded only after write + flush.
- Existing per-slot content identity handles LRU image-ID reuse and focus epoch
  invalidation. Failed IO leaves transfers retryable.
- Raster workers, compilation cache and last-good-PDF behavior are unchanged.

## Verification

- Regression: rendering an image then clearing/overwriting its entire buffer
  does not consume the first transfer. Upload still sends PNG.
- Retry after writer failure, cached replay, slot reuse and full-width placement
  checks pass in Kitty unit test.
- `scripts/typst-preview-smoke.py`: actual embedded Typst compiler, PDF rasterizer
  and Kitty transfer pipeline in a PTY. Two colored pages, fit/full-width toggles,
  33 atomic saves (including rapid burst), latest frame. One run decoded **18
  nonblank PNG transfers**, checking pixels rather than just protocol presence.
- `ssh-kitty-smoke.py --binary target/debug/leafview`: direct/tmux transfer and
  reconnect passed. This argument matters: the script defaults to release and
  must not silently validate an older build.

## Idle/active-slide recovery

The remote terminal cache is not durable: tmux can suppress a transfer while a
pane is hidden, and an outer terminal can discard graphics without delivering a
focus/resize event. A successful stdout flush is not a terminal acknowledgement.
Recovery is event-driven (focus, resize, manual reload), never periodic. A
previous 15-second renewal caused visible idle refreshes and has been removed.
New completed frames reset the frame diff so a reused image ID cannot suppress
placement restoration.

Focus return restores the existing PNG immediately and once more after 200 ms:
a tmux screen repaint may race the first transfer. This is a bounded,
event-triggered retry, not periodic refresh, and focus loss cancels it. Hidden
panes do not upload PNGs. The SSH PTY smoke checks both replays, cancellation,
and upload-free idle after recovery; it does not reproduce a real tmux/GPU race.

`scripts/typst-preview-smoke.py` checks 16 seconds without input/focus/resize:
no PNG uploads, screen clears or new render frames. It then checks 36 additional
active-slide edits (69 atomic saves total) through image-slot reuse.

Page navigation now retains the last completed image while the next raster and
PNG are prepared. Publication wraps clear, upload and placeholder drawing in a
synchronized terminal update (DEC mode 2026), avoiding a visible blank interval
on terminals supporting it. The SSH PTY smoke checks the ordering for a page
change in both direct and tmux framing. Unsupported terminals may still expose
intermediate transfer/draw states; this does not shorten PDF rasterization time.

These tests validate PNG content and protocol output, not GPU rendering in the
user's actual Kitty/tmux client. A genuinely blank source page remains valid and
is not automatically rejected or replaced by another page.
