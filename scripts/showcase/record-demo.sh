#!/usr/bin/env bash
# All GUI processes live on a fresh Xvfb display; no desktop or global config.
set -euo pipefail
REPO=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
: "${KITTY_BIN:=$(command -v kitty || true)}"
: "${TYPST_BIN:=$(command -v typst || true)}"
: "${SHOWCASE_PYTHON:=python3}"
: "${LEAFVIEW_BIN:=$REPO/target/release/leafview}"
for tool in Xvfb ffmpeg pdfinfo pdftoppm prlimit tinymist; do command -v "$tool" >/dev/null || { echo "Missing $tool" >&2; exit 1; }; done
[[ -x "$KITTY_BIN" && -x "$TYPST_BIN" && -x "$LEAFVIEW_BIN" ]] || { echo 'Set KITTY_BIN, TYPST_BIN and LEAFVIEW_BIN to existing executables' >&2; exit 1; }
"$SHOWCASE_PYTHON" -c 'import Xlib, PIL'
ROOT=$("$REPO/scripts/showcase/prepare-demo.sh")
source "$ROOT/environment.sh"
# -displayfd asks Xvfb to choose an unused display; never reuse the desktop.
Xvfb -displayfd 3 -screen 0 2560x1440x24 -nolisten tcp 3>"$ROOT/display" >"$ROOT/xvfb.log" 2>&1 & XVFB_PID=$!
KITTY_PID= WATCH_PID=
cleanup() {
  [[ -z "$KITTY_PID" ]] || kill "$KITTY_PID" 2>/dev/null || true
  [[ -z "$WATCH_PID" ]] || kill "$WATCH_PID" 2>/dev/null || true
  kill "$XVFB_PID" 2>/dev/null || true
  wait 2>/dev/null || true
  printf 'Disposable workspace retained for inspection: %s\n' "$ROOT"
}
trap cleanup EXIT INT TERM
"$SHOWCASE_PYTHON" -c 'import pathlib,sys,time; p=pathlib.Path(sys.argv[1]); end=time.monotonic()+10
while (not p.exists() or not p.stat().st_size) and time.monotonic()<end: time.sleep(.1)
assert p.exists() and p.stat().st_size, "Xvfb failed"' "$ROOT/display"
SCREEN=":$(<"$ROOT/display")"
cd "$ROOT/demo-workspace"
"$TYPST_BIN" watch main.typ documents/sample.pdf >"$ROOT/watch.log" 2>&1 & WATCH_PID=$!
# Wait for first successful output without reading any user workspace.
"$SHOWCASE_PYTHON" -c 'import pathlib,sys,time; p=pathlib.Path(sys.argv[1]); end=time.monotonic()+30
while not p.exists() and time.monotonic()<end: time.sleep(.1)
assert p.exists(), "Fixture compilation failed"' "$ROOT/demo-workspace/documents/sample.pdf"
# A neutral shell lets capture type the real default launch command.
mkdir -p "$ROOT/bin"
ln -s "$LEAFVIEW_BIN" "$ROOT/bin/leafview"
export PATH="$ROOT/bin:$PATH" PS1='demo-workspace $ ' HISTFILE=/dev/null
DISPLAY="$SCREEN" LIBGL_ALWAYS_SOFTWARE=1 "$KITTY_BIN" --config "$ROOT/kitty.conf" --directory "$PWD" bash --noprofile --norc -i >"$ROOT/kitty.log" 2>&1 & KITTY_PID=$!
# Finite initial render wait; recording/scene timing is managed by capture.py.
"$SHOWCASE_PYTHON" -c 'import time; time.sleep(4)'
cd "$REPO"
"$SHOWCASE_PYTHON" scripts/showcase/capture.py --root "$ROOT" --display "$SCREEN" --output docs/assets
ffmpeg -y -loglevel error -ss 38 -t 18 -i docs/assets/demo/leafview-demo.mp4 -vf 'fps=8,scale=960:-1:flags=lanczos' -c:v libwebp_anim -quality 65 -loop 0 docs/assets/demo/leafview-preview.webp
ffmpeg -y -loglevel error -ss 38 -t 18 -i docs/assets/demo/leafview-demo.mp4 -filter_complex '[0:v]fps=8,scale=1280:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle' -loop 0 docs/assets/demo/leafview-preview.gif
ffprobe -v error -show_entries stream=width,height,r_frame_rate -show_entries format=duration,size -of json docs/assets/demo/leafview-demo.mp4
