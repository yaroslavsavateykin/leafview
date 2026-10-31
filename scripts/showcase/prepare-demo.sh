#!/usr/bin/env bash
# Creates a new disposable workspace. Never reuses or clears an existing HOME.
set -euo pipefail
ROOT=$(mktemp -d /tmp/leafview-showcase.XXXXXX)
export HOME="$ROOT/home" XDG_CONFIG_HOME="$ROOT/config" XDG_STATE_HOME="$ROOT/state" XDG_CACHE_HOME="$ROOT/cache"
mkdir -p "$HOME" "$XDG_CONFIG_HOME/leafview" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"
HERE=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cp -R "$HERE/workspace" "$ROOT/demo-workspace"
# Shared imports live above documents/. This is a fixture-only project setting.
printf '%s\n' '{"root":"."}' > "$ROOT/demo-workspace/.leafview.json"
cat > "$XDG_CONFIG_HOME/leafview/completion.json" <<'JSON'
{"auto_insert":false,"sources":["lsp","buffer","path","snippets"]}
JSON
cat > "$ROOT/kitty.conf" <<'CONF'
background #1e1e2e
foreground #cdd6f4
cursor #f5e0dc
font_family DejaVu Sans Mono
font_size 17
window_padding_width 12
hide_window_decorations yes
confirm_os_window_close 0
enable_audio_bell no
remember_window_size no
initial_window_width 2560
initial_window_height 1440
linux_display_server x11
allow_remote_control socket-only
CONF
printf 'export HOME=%q XDG_CONFIG_HOME=%q XDG_STATE_HOME=%q XDG_CACHE_HOME=%q\n' "$HOME" "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME" > "$ROOT/environment.sh"
printf '%s\n' "$ROOT"
