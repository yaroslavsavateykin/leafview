# Typst live preview

[Documentation](README.md) · [Project](../README.md)

## Typst Live Preview

```sh
leafview main.typ
# Or: leafview *.typ (when exactly one .typ file matches)
leafview main.typ --input mode=teacher
leafview typst main.typ  # legacy syntax remains supported
leafview typst "./notes with spaces/lecture.typ"
leafview typst letuchka.typ --root ../..
leafview typst main.typ --input edition=teacher --font-path ./fonts -j 2
```

Typst support is opt-in (ordinary PDF builds do not include the compiler):

```sh
cargo install --locked --features typst --path . -j 2
```

For a PDF-only installation, use `cargo install --locked --path .`.

LeafView compiles the entry file in an owned background compiler process and
shows the PDF using the **same renderer and graphics policy** as PDF mode.
You do not need to run `typst watch`. Its CLI reports compilation status and
can list dependencies, but does not export source-to-PDF coordinates or the
changed layout region. LeafView instead reuses an incremental compiler world
and reads source spans from that same compilation's layout. No installed `typst`
CLI is needed: compilation uses the pinned **Typst Rust API 0.15.1** bundled
with the `typst` feature, not a potentially different CLI version. This adds substantial
build dependencies and a roughly 47 MiB release executable.

After a save, dependency events are debounced for 200 ms. One compiler is reused
for incremental compilation; at most one job is running and one latest request
is pending. Newer edits invalidate older results. PDF and source coordinates
always come from the same compiled layout. By default builds use a **private
preview snapshot**: opening or selecting a `.typ` does not create or overwrite
`main.pdf` beside the source. Run `leafview main.typ` (or `leafview *.typ` when
there is exactly one matching file). Use `leafview main.typ --write-pdf` to opt
into atomic PDF export, or enable Write PDF in compilation settings. A previously
saved Export PDF setting is still respected. Disable Export PDF in settings to
return to private preview. `--no-pdf` has been removed.
Failed or superseded builds do not replace exported files; own PDF writes do not
trigger a rebuild.
Compilation errors
keep the previous PDF visible; fix and save again, or press `r` to retry. An
initial error shows a blank placeholder until compilation succeeds.

Changes in included/imported files and compiler-known resources also trigger a
build. LeafView watches dependency **parent directories non-recursively**, so
Neovim atomic saves work without recursively watching your home directory. The
entry file is the root document; the project root defaults to its parent. Set
`--root /path/to/project` or `TYPST_ROOT` for a different root. Relative flag
paths are resolved from the launch directory, like the Typst CLI. Flags may
precede or follow the input; `--flag=value` is supported.

Supported Typst 0.15.1 compile settings: `--root`, repeatable `--input key=value`,
`--font-path` (repeatable/path-separated), `--ignore-system-fonts`,
`--ignore-embedded-fonts`, `--package-path`, `--package-cache-path`, `--cert`,
`--creation-timestamp`, `-j`/`--jobs`, `--features`, `--pdf-standard`,
`--no-pdf-tags`, `--pretty`, `--pages`, `-f pdf`/`--format pdf`.
Standard Typst environment defaults for roots/fonts/packages/cert/timestamp/features
are honored; explicit flags take precedence (font paths add to the configured
search paths). Page selection renumbers source mappings to the exported PDF.
`--diagnostic-format` and `--color` are accepted, but the viewer always displays
sanitized short status messages; `--ppi` has no effect on PDF (PNG export only).
Use `leafview typst --help` or `--version` for help/compiler version.

Not all CLI side effects fit a live PDF viewer: `--deps`, `--deps-format`,
`--make-deps`, `--timings`, `--open`, `--no-serve`, `--no-reload`, `--port`,
non-PDF output formats and a positional output path are **explicitly rejected**,
not forwarded or silently ignored. Use `typst compile/watch` for these exports
or server features. LeafView does not claim full Typst CLI flag compatibility.
`--write-pdf` and `--output PATH` are LeafView-specific options. Output paths
are relative to the source directory unless absolute. In compilation settings
(`o`), use **Export PDF** and **Output PDF path** to control publication and filename.

| Key in Typst mode | Action |
| --- | --- |
| `s` | Toggle automatic source synchronization (shown as `sync on/off`) |
| `f` | Follow the last mapped edit manually |
| `r` | Recompile |

The status shows `⏳` while compiling or preparing the image, then `✓` once
loading finishes. Compilation errors appear in a centered reddish bordered
panel with the source path, line/column, source excerpt/caret and available hints.
The last successful PDF stays in memory and remains behind the diagnostic panel. A successful
recompile automatically closes the panel and restores PDF display.

Source synchronization compares successful source snapshots, selects the last
changed character/token in the most recently changed source file, then uses
compiler glyph spans to find its new PDF coordinates. Multiple disjoint edits
in one save deterministically select the last change in file order; deletion
uses the nearest surviving mapped token. Repeated spans on different pages,
style-only changes without glyph correspondence and resource-only changes do
not force an arbitrary jump. Manual navigation stays put until a new mapped
source change; zoom is preserved, and a target already visible does not scroll.
Otherwise it is positioned near the upper/left third of the viewport.

Limits: one compile has a 60 s timeout and a 3 GiB address-space cap; source
snapshots are capped at 16 MiB, the map at 500,000 glyph entries. Mapping covers
text/math glyphs, not arbitrary shape/image source expressions. Highly dynamic
macros, generated content and multi-location spans can remain unmapped. Glyph
coordinates indicate a baseline, not an exact selection rectangle. Native
Ghostty+tmux graphics limitations from [AUDIT-2026-10-19.md](archive/AUDIT-2026-10-19.md)
remain historical test findings. Both Typst and PDF mode now negotiate native
graphics by default, including Ghostty+tmux; safe mode is explicit opt-in.

See [TYPST-VERIFICATION.md](archive/TYPST-VERIFICATION.md) for actual tests, measurements
and remaining gaps. Additional automated check: `python3 scripts/typst-smoke.py`.

### Editable compilation flags

Press **o** in Typst mode. The **Typst** tab contains every implemented compiler flag; **Document inputs**
is a separate key/value tab; **LeafView** contains compile-on-save, PDF publication
and source following.
Use ←/→ or h/l to switch tabs; Tab / Shift+Tab, ↑↓ or j/k to select,
Enter to edit/toggle. On the bottom button row, ←/→ and h/l move between
Apply, Save and Cancel instead of switching tabs. Captions explain each action.
Package/export flags are shown directly, with no Advanced section. Every field
has a description identifying the corresponding CLI flag.
Paths and numbers are plain text; font directories use semicolons, feature/standard
lists use commas. Empty = default. No JSON quotes are needed.

The **Document inputs** tab contains a key/value table: ↑↓ selects a row, Tab selects
key/value, Enter edits, **n** adds a pair, Delete removes a row outside text editing.
Keys must be nonempty and unique; values can be empty and retain exact whitespace.
**F2** accepts the table into the settings draft; Esc cancels the current edit,
then the panel. Outside text editing ←/→ switches tabs, preserving valid inputs.
Ctrl+S also accepts and saves directly from the table.

All text fields support ←/→ cursor movement, Home/End, Backspace, Delete and
**Ctrl+U** to clear. **Ctrl+S saves for this file**, including the current edit;
**Ctrl+A applies without saving**. A confirmation is shown in the PDF title.
Esc discards the panel. Mouse/PDF shortcuts cannot leak through the editor.

**Apply** changes this session only. **Save file** applies and remembers the flags
for the current input in the shared config, normally
`$XDG_CONFIG_HOME/leafview/config.json` (or $XDG_CONFIG_HOME/leafview/config.json).
There are no profiles, versions or separate per-document JSON files:

```json
{
  "files": {
    "/home/user/lecture.typ": {
      "inputs": {"mode": "teacher", "solutions": "true"},
      "jobs": 2,
      "write_pdf": true
    },
    "/home/user/exam.typ": {
      "inputs": {"solutions": "false"},
      "write_pdf": false
    }
  }
}
```

File keys saved by LeafView are canonical absolute paths. Relative file keys
resolve from the config directory. Paths inside file-specific settings resolve
from the input's directory; top-level defaults resolve from the config directory.
The editor and JSON retain paths exactly as entered (for example ../..); path
resolution happens only for compilation, never by rewriting the displayed text. Saving preserves entries for other files
and refuses to overwrite malformed JSON. Existing profile-based configs must
be converted to the flat format; they are never silently overwritten.

Supported fields: root, inputs (string map), fonts (path array), jobs (1–256),
package, cache, cert, timestamp, features, standards, pages, ignore_system,
ignore_embedded, pretty, no_tags, write_pdf, auto_compile, auto_sync.
The legacy no_pdf flag is accepted, but saved JSON uses write_pdf only.
Nullable fields accept null; an explicit inputs object replaces inherited inputs.

Top-level flags supply shared defaults. The nearest ancestor .leafview.json may
also contain the same flat format. Priority: defaults < global flags < project
flags < global file entry < project file entry < CLI < runtime.
Configuration is loaded on open and re-read when opening settings.

Changing compiler flags invalidates old generations and safely kills/reaps the
old helper before replacement. Unchanged compiler flags reuse the incremental
world. Last-good PDF, page, zoom and scroll survive failed changes. Disabling
auto_compile keeps watchers alive; r builds manually. No extra Rust dependencies
or independent typst watch process are used.

Real integration check: python3 scripts/settings-smoke.py covers runtime inputs,
jobs restarts, file-specific persistence, manual builds, invalid configuration,
last-good PDF and document switching/child cleanup. These are headless PTY tests,
not native Ghostty/tmux visual certification.
