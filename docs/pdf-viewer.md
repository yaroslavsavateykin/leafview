# PDF viewer

[Documentation](README.md) · [Project](../README.md)

## PDF library browser

Launch modes:

- `leafview ./papers/report.pdf` — document only, sidebar hidden.
- `leafview ./papers` — browse the specified directory with the sidebar open.
- `leafview` — browse PDFs in the current working directory.

Use `Esc` (or `F2`) to show the sidebar while reading a PDF. Directional focus never
opens a hidden pane.
The sidebar has two independent panes: the PDF directory tree above and Recent
below. Each keeps its own selection and scroll position. The focused pane has
a cyan border and a ● marker in its title; the PDF pane has the same indicator. Only
PDF and Typst files and directories appear; folders expand lazily, sorted before files.
Enter on `.typ` starts the same embedded Typst Preview with that project's config;
Enter on `.pdf` returns to PDF mode. The previous PDF remains until the new build
succeeds. Switching documents reaps the previous compiler and releases its watcher.
PDF files are warm amber; Typst files are soft blue. Folders containing openable
documents are turquoise; empty folders are muted gray. Unknown/unscanned folders
stay neutral. Selection and the current document's parent use a background tint,
without extra markers; directory expand/collapse arrows remain. Recent uses the
same file-type colors. Enter opens a selected file without leaving LeafView;
failed opens retain the current document. Recent history contains up to 20 paths
and is saved atomically to `$XDG_STATE_HOME/leafview/recent.json` (default:
`~/.local/state/leafview/recent.json`). No document contents are stored.

| Browser key | Action |
| --- | --- |
| `Ctrl+h` / `Ctrl+l` | Focus left sidebar / PDF |
| `Ctrl+j` / `Ctrl+k` | Focus Recent below / tree above (inside sidebar) |
| `Ctrl+←` / `Ctrl+→` | Move the vertical split left / right (1 column) |
| `Ctrl+↑` / `Ctrl+↓` | Move the tree/Recent split up / down (1 row, sidebar focus) |
| `Tab` | Cycle focus: tree → Recent → PDF → tree |
| Hover / left click | Select browser row / open file or toggle folder (hover does not change keyboard focus) |
| Drag pane border | Resize sidebar, tree/Recent or source/PDF split |
| `Esc` / `F2` | Show / hide sidebar |
| `↑` / `↓`, `j` / `k` | Select entry |
| `→` / `l` | Expand directory |
| `←` / `h` | Collapse directory or select parent |
| `Enter` | Open PDF / toggle directory |
| `Backspace` | Browse parent directory |
| `g` / `G`, Home / End | First / last entry |
| `r` | Refresh directory listing |

A pane must be activated by click, Tab or directional focus before it accepts
input. Hover and wheel events never move focus; the first click in an inactive
PDF activates it without following a hyperlink.

The sidebar hides automatically below 48 terminal columns. Symlinked directories
are not followed; listings are bounded to 5,000 visible entries and 24 levels.
Tree refresh is manual (`r`); the open document still reloads automatically on saves.
PDF switching is asynchronous and updates the file watcher. Typst preview starts
with the sidebar hidden; Esc opens it for switching to PDF or another Typst input.

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

Hover changes only the PDF frame title: no Poppler render, image encoding or overlay
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
