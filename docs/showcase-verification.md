# Showcase verification

[Documentation](README.md) · [Project](../README.md)

This report records this documentation/media refresh, not a clean bill of health
for every application feature. No Rust implementation or tests were changed.

## Media and isolation

- Seven real PNG screenshots, including welcome and settings: **2560×1440**,
  captured directly from X11 pixels.
- Full MP4: **2560×1440, 30 FPS, H.264/yuv420p, 90.2 seconds, 3.25 MB**,
  continuous walkthrough without cuts between actions.
- Leading animated GIF: 18 seconds, 1280×720, 8 FPS.
- Animated WebP alternative: 18 seconds, 960 pixels wide, 8 FPS.
- Header badges are local SVGs. All README images and videos live under
  `docs/assets/` and use repository-relative links, without external image hosting.
- Kitty 0.49.2, Xvfb, software GL; native Kitty PDF graphics, not halfblocks.
- Typst compiler 0.15.1; external Tinymist 0.15.10 for the genuine popup.
- Release `typst,editor` executable used in the final reproducible recording.
- All source, plot and table content is synthetic, checked in under
  `scripts/showcase/workspace/`. Separate HOME/config/state/cache and dedicated
  virtual displays; no desktop capture or user configuration changes.
- Script-generated sample PDF, neutral file names and no network/SSH session in
  the scene. Real edit → `:w` → title update and compiler pass counter observed.
- PNGs and decoded video storyboard inspected. MP4 fully decoded with ffmpeg
  without errors; ffprobe confirms dimensions/rate/duration. WebP frames verified
  with Pillow. No user paths, personal documents, tokens or private hosts visible
  in the reviewed media. Temporary paths in command entry were replaced by
  relative fixture paths before the final recording.

The PDF-only screenshot shows `sample.pdf` opened as an ordinary PDF before
switching to Typst. The walkthrough starts by typing `leafview` without flags,
uses directory navigation, turns PDF pages, fits width and zooms in/out, then
opens Typst, edits the title letter by letter, saves, shows completion and visits
compilation settings tabs. No diagnostics screenshot is included.

The default welcome screen contains the disposable `/tmp/leafview-showcase.*`
fixture path. It is intentionally real UI, not a redacted or composited screen;
no personal home directory appears.

## Build and runtime checks

Passed:

- `cargo build --locked` (PDF-only; existing unused-method warning)
- `cargo build --locked --features typst,editor -j 2`
- `cargo build --locked --release --features typst,editor -j 2`
- `cargo check --locked --all-features -j 2`
- `cargo fmt --all -- --check`
- An isolated install using `cargo install --locked --debug --path .
  --features typst,editor -j 2 --root /tmp/leafview-showcase-install`
- `cargo test --locked --all-features --test typst`: 2 passed
- Editor, completion and keybinding PTY smoke scripts, including real Tinymist
- Reproducible release showcase script: real launch, editing, saving, tabs,
  completion, PDF navigation and restored final view
- Fixture Typstyle check: wrap-text, line width 70, indent width 2
- Standalone fixture watcher compilation without font warnings

The documented release build was executed. Apt installation and a fresh remote
clone were not executed: existing Poppler/util-linux and this checkout were used.
No system installation or replacement of the user's LeafView executable occurred.

## Failing checks (not hidden or skipped)

- All-features unit suite: **97 passed, 6 failed, 10 ignored**. Five failures are
  PDF annotation/link geometry assertions. The Neovim comparison fails because
  `nvim` is not installed on PATH in this environment.
- Default unit suite: **51 passed, 6 failed, 3 ignored**. PDF link/geometry
  assertions and a stale-encoder timing assertion fail.
- CLI integration: 2 passed, 1 link-extraction fixture assertion failed.
- `scripts/pty-smoke.py`: zoom/reset assertion failed.
- `scripts/settings-smoke.py`: expects a neighboring `main.pdf` which is absent
  under private-preview defaults; test fails while reading that file.
- All-feature Clippy with `-D warnings` fails on existing editor implementation
  warnings (including completion argument count and identical conditional blocks).
- Latest public CI run inspected during the refresh was a failure:
  [Rust run 41145703248](https://github.com/yaroslavsavateykin/leafview/actions/runs/41145703248).
  No build-status badge was added. Existing CI covers default/Typst, not editor.

Some related failures appear in historical reports; this task did not build a
separate baseline to prove the origin of every failure. These are current observed
failures, not regressions attributed to the documentation changes.

## Documentation and publication

- Markdown lint configured for GitHub header HTML and prose-length lines.
- Local Markdown/image/file targets checked, including archive relocation.
- README rendered locally in Chromium with GitHub Markdown CSS; image layout
  inspected. External badges initially timed out; they were subsequently replaced
  by local SVG assets to remove third-party image requests.
- Original README and six reports retained in `docs/archive/`; no useful report
  removed. Root now keeps user-facing project files rather than status reports.
- `git diff --check`, shell syntax and Python syntax checked.

**Not verified:** the final hosted GitHub rendering of this branch, remote media
playback after push, live SSH, real tmux lifecycle, other graphics protocols,
and every terminal/font combination. Nothing was pushed, deployed or uploaded
to a release. Relative media links resolve to checked-in files once published;
GitHub uses a linked animated preview rather than an HTML video embed.
