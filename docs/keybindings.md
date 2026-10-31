# Keyboard and mouse reference

[Documentation](README.md) · [Project](../README.md)

## Keys (document focus)

| Key | Action |
| --- | --- |
| `h` / `l`, Left / Right | Horizontal scroll within the page |
| `j` / `k`, Down / Up | Vertical scroll within the page |
| Shift + arrows or `H` / `J` / `K` / `L` | Previous page: Left / Up / H / K; next page: Right / Down / L / J |
| `p` / `n`, PageUp / PageDown, Space | Previous / next page (Space: next) |
| Wheel/trackpad up / down | Zoom in / out by 5 percentage points around the pointer |
| Wheel/trackpad right / left | Next / previous page (at most one turn per 250 ms) |
| `+` / `-` (also `=` / `_`) | Zoom by 5 percentage points, manual zoom relative to pane width |
| `0` / `z` | Fit page / fit width and reset pan |
| `g` / `G`, Home / End | First / last page |
| Digits + Enter | Jump to page (one-based) |
| Backspace / Esc while entering digits | Edit / cancel page input |
| `r` | Reload PDF |
| `b` | Back to previous internal-link location |
| `Esc` (no page input) / `F2` | Show / hide sidebar |
| `Ctrl+C` | Quit (`q` does not quit) |

Trackpad gestures work when the terminal/tmux forwards SGR wheel events;
LeafView cannot read raw touchpad/pinch gestures. Zoom uses small discrete steps
coalesced by the existing renderer, not pixel-level animation. At page edges or
the 3400-pixel raster cap the pointer anchor is clamped. Arrows and `hjkl` scroll
within the page; hold Shift to turn pages instead. Horizontal gesture direction follows
the ScrollLeft/ScrollRight events supplied by your natural-scroll settings.

The PDF frame title shows the filename, requested page, zoom and loading or error state. While
loading, **the previous image remains visible** and is explicitly labelled as
previous: do not mistake it for the requested page.

## Source, browser and completion controls

See the [editor guide](editor.md) for modal editing, file tabs and LSP controls,
and [PDF browser controls](pdf-viewer.md) for tree navigation. Mouse wheel over
source scrolls source; over PDF it zooms. Hover never changes keyboard focus.
F7/F8 focus/close LSP tool windows, not source/PDF panes.

## Command palette and custom mappings

F9 opens the searchable command palette; F10 opens Keyboard Shortcuts. The
latter displays effective bindings and their origins. Default preset is
`astronvim`; available preset names are `vim`, `neovim`, `astronvim`,
`leafview-default` and `custom`. This does not imply full editor parity.

Bindings are layered: global `$XDG_CONFIG_HOME/leafview/keybindings.toml`
(or `LEAFVIEW_KEYBINDINGS`), workspace `.leafview/keybindings.toml`, then
project `.leafview/project-keybindings.toml`. Workspace discovery uses the
nearest Git ancestor, or the current root when none exists. One override replaces
one key, not the entire key map. For example:

```toml
[keybindings]
preset = "astronvim"
leader = "Space"
timeout_ms = 500

[keybindings.bindings.PDF]
"n" = "pdf.next_page"
"p" = "pdf.previous_page"
```

Use the UI to inspect command IDs and reload mappings. Context and modal state
matter: source Insert input must not leak into PDF/browser commands.
