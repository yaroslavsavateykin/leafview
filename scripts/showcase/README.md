# Reproduce the LeafView showcase

Captures are real Kitty window pixels on a **dedicated Xvfb display**, not a
Ratatui mockup or escape-sequence animation. The screen is 2560×1440 and MP4 is
H.264/yuv420p, 30 FPS. The terminal uses a Mocha-like background and DejaVu Sans
Mono (widely available; no font installation or global config changes).

## Prerequisites

- Linux, Xvfb, Kitty, ffmpeg with x11grab/libx264/libwebp_anim
- Poppler and util-linux, a `typst,editor` LeafView build
- Typst CLI 0.15.1 to generate the standalone sample PDF
- Tinymist on PATH (capture tested with 0.15.10)
- Python with `python-xlib` and `Pillow`, e.g. in a disposable virtual environment
- DejaVu Sans / Sans Mono fonts

No dependencies are downloaded by the scripts. The capture used Kitty 0.49.2.
The installed Typst CLI is only for preparing fixtures, not LeafView compilation.

```sh
cargo build --locked --release --features typst,editor -j 2
python3 -m venv /tmp/leafview-capture-venv
/tmp/leafview-capture-venv/bin/pip install python-xlib Pillow
SHOWCASE_PYTHON=/tmp/leafview-capture-venv/bin/python \
  bash scripts/showcase/record-demo.sh
```

If tools are not on PATH, set `KITTY_BIN`, `TYPST_BIN`, `LEAFVIEW_BIN` to absolute
executables. Tinymist must be discoverable through PATH. Run from a checkout;
the script resolves the repository directory itself. It overwrites **only the
showcase media under `docs/assets`**, so retain old versions if you need a comparison.

## Safety model

`prepare-demo.sh` always creates a fresh `/tmp/leafview-showcase.XXXXXX`. It copies
only the checked-in synthetic `workspace/`, creates separate HOME,
XDG_CONFIG_HOME, XDG_STATE_HOME and XDG_CACHE_HOME, and writes a local Kitty
config. It never reads/copies your editor config, clears history or removes files.
No user documents, real SSH hosts or network connections are part of the scene.
The welcome screen shows only the generated `/tmp/leafview-showcase.XXXXXX`
workspace path, never a real home directory. Shell history is disabled.

`record-demo.sh` asks Xvfb for a free display using `-displayfd`. GUI capture and
input target that display only. A trap stops only process IDs started by this
script. Temporary workspaces are **retained**, not recursively removed. Inspect
them and remove the exact generated directory yourself when no longer needed.
There is deliberately no broad `cleanup.sh` or `pkill` command.

The helper `capture.py` rejects desktop displays `:0`, `:1` and the inherited
DISPLAY, and requires exactly one visible window. `xkeys.py` is a small optional
manual-inspection helper for a dedicated display; it is not used by the recorder.

## Scene sequence

1. Type `leafview` into a clean shell, letter by letter; show the default welcome screen.
2. Expand `documents/`, select `sample.pdf` and open it as an ordinary PDF.
3. Turn pages quickly, zoom in/out and return to Fit Page.
4. Return to the working tree, move between entries, collapse/expand a folder,
   then open `introduction.typ`.
5. Open source, resize the browser, type a title edit slowly and save with `:w`.
6. Wait for the real PDF update; type `#page`, browse completion and documentation.
7. Undo the unsaved scratch line, open compilation settings, inspect a field
   and visit its tabs without persisting a change.
8. Open a second source tab and return to the complete workspace.

The recording is continuous: no cuts between intermediate states. Text input is
sent one character at a time, with deliberate holds between actions. Project root
is configured only in the generated fixture `.leafview.json`; the visible launch
has no flags. No fake UI or synthesized PDF overlay is used.

Seven PNGs (including welcome and settings) and the MP4 are written to
`docs/assets/`. An 18-second, 1280-pixel-wide animated GIF leads the README;
a smaller 960-pixel-wide animated WebP is also available. Both are generated from the real recording. The full MP4 is
small enough to keep alongside the docs; no upload or release publication is done.
GitHub Markdown displays the GIF directly; the full MP4 is a separate text link,
not an unsupported embedded video tag.

## Updating the fixture

All content is invented: equations, a synthetic curve, illustrative table values
and neutral filenames. `main.typ` includes `documents/introduction.typ`;
`style/{macros,settings,template}.typ` keep styling local. Opening the entry needs
`--root .` because imports live above `documents/`.

Format all `.typ` files with Typstyle before recording:

```sh
find scripts/showcase/workspace -name '*.typ' -print0 | \
  xargs -0 typstyle --wrap-text --line-width 70 --indent-width 2 -i
```

The recorder runs one fixture watcher while recording. LeafView independently
compiles the opened entry and rebuilds on its own save watcher. No screenshots
are composited. Reinspect every image and sample the final video after UI changes;
timing-based automation is not a substitute for visual verification.
