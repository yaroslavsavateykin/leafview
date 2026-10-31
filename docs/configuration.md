# Configuration

[Documentation](README.md) · [Project](../README.md)

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
LEAFVIEW_DEBUG=1 LEAFVIEW_LOG=/tmp/leafview.log leafview document.pdf # log selected backend
```

`LEAFVIEW_GRAPHICS=auto` (default) queries terminal capabilities without a
Ghostty/tmux exception. Explicit safe mode deliberately skips graphics
capability negotiation. Navigation/render requests are coalesced to at most
one every 25 ms during key repeats; the final viewport is always scheduled.
Ctrl-C is handled as an ordinary clean exit. A system-level
SIGKILL, OOM kill, or a crash inside Ghostty itself cannot be handled by an
application's cleanup code.

Loading, compilation, link-hover hints and short notifications appear in the
bottom footer, never appended to the document title. Borderless mode reserves a
footer row outside the PDF image. Compile diagnostics appear once in the error
panel. Optional `LEAFVIEW_DEBUG`/`LEAFVIEW_PERF` diagnostics are written only to
`LEAFVIEW_LOG` when that path is set, never over the terminal UI.

## SSH and Kitty graphics

Run Leafview **on the server**, from a local terminal supporting Kitty graphics
with Unicode placeholders (for example Kitty). Both queries and lossless PNG
images travel inline through the SSH terminal stream (`t=d`); no shared filesystem,
X forwarding, remote control socket or copying PDFs to the client is required.
The server still needs Leafview, Poppler and util-linux installed.

```sh
ssh -t server 'leafview /path/to/document.pdf'
# If capability negotiation is blocked but the client is known to support Kitty:
ssh -t server 'LEAFVIEW_GRAPHICS=kitty leafview /path/to/document.pdf'
# A slow connection can use a longer capability-query timeout (100..15000 ms):
ssh -t server 'LEAFVIEW_QUERY_TIMEOUT_MS=5000 leafview /path/to/document.pdf'
```

SSH sessions detected via `SSH_CONNECTION` or `SSH_TTY` use a 3-second query
budget; local sessions use 1 second. `kitty` explicitly forces the graphics
backend, but still queries cell size; prefer automatic negotiation. A PTY is
required (`ssh -t`), and the SSH session must preserve escape sequences and UTF-8.
A terminal implementing only part of the Kitty protocol may not support the
Unicode placements used here. Do not force Kitty on unsupported clients.

With tmux on the server, use a recent tmux and configure:

```tmux
set -g allow-passthrough on
set -g focus-events on
```

The first display transmits PNG data; revisiting cached views uses terminal-side
images without retransmitting them. Network throughput still bounds cold-page
latency. Reattaching from another client invalidates upload state on focus;
press `r` if focus forwarding is unavailable. Cleanup deletes only Leafview's
32 image slots. The protocol uses no server-local image filenames.

Verification: `python3 scripts/ssh-kitty-smoke.py` emulates a Kitty client over a
PTY with 1.2-second query latency, direct/tmux framing, inline PNG transport and
focus-triggered re-upload. This is **not** a real SSH/network or visual terminal
test; those remain to be checked on the target client/server.

## Terminals and tmux

The showcase was captured in Kitty on an isolated Xvfb display with native
inline PNG graphics. This verifies that capture configuration, not every
terminal/version, SSH network path or tmux lifecycle. Historical Ghostty/tmux
checks and corruption observations are retained in [the archive](archive/README.md).
Sixel/iTerm2 paths have not been visually verified by the showcase.

Capability negotiation is delegated to ratatui-image 11.0.8. `TMUX` alone is not
proof of graphics support. The library may enable `allow-passthrough` on the
current pane during detection. Kitty Unicode placeholders constrain images to
the pane. Leafview uses 32 process-owned image slots and deletes only those slots on
normal exit; it does not delete other panes' graphics.

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

## Start screen and recent workspaces

Launching `leafview` without a document (or with a directory) shows “Ничего не
выбрано”, not a blank PDF page. Select a file in the tree/Recent and press Enter,
or press 1–9 on the welcome screen to reopen a recent workspace. Workspace history
stores the browser folder and document in `leafview/workspaces.json` alongside
`recent.json` under the XDG state directory. Old Recent entries remain usable.
No document is automatically opened or exported on this screen.

## Configuration files

- Compiler flags: `$XDG_CONFIG_HOME/leafview/config.json` and the nearest
  ancestor `.leafview.json`; see [Typst settings](typst.md#editable-compilation-flags).
- Completion: `$XDG_CONFIG_HOME/leafview/completion.json` or
  `LEAFVIEW_COMPLETION_CONFIG`; [example](../examples/completion.json).
- Keyboard mappings: `$XDG_CONFIG_HOME/leafview/keybindings.toml` and project
  layers; see [keybindings](keybindings.md#command-palette-and-custom-mappings).
- Recent documents and workspaces: `$XDG_STATE_HOME/leafview/`, falling back to
  `~/.local/state/leafview/`. Paths, not document contents, are stored.

An unset XDG config directory falls back to `~/.config`. LeafView does not
provide a global theme configuration; editor/completion styling and terminal
colors are distinct. The showcase terminal config does not alter application UI.
