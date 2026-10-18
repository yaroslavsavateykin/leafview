# Typst Live Preview verification — 2026-10-19

## Trackpad/wheel navigation update

Added ScrollUp/Down pointer-anchored zoom (5 percentage points, rounded to avoid
float drift) and ScrollLeft/Right page navigation (250 ms turn throttle). Anchor
math accounts for image centering, zoom raster cap, current offsets and page
aspect; bounds still clamp. Existing keyboard scrolling unchanged. Unit test
checks normalized page-point stability and centered-page zoom. PTY integration
sends actual SGR 64/66/67 reports, checks ten wheel ticks reach exactly 150%,
horizontal right/left reach pages 2/1, then source follow and remaining live tests.
Default unit tests, feature tests, Clippy, release and both PTY suites passed;
feature version installed. Physical touchpad gesture delivery in Ghostty/tmux
has NOT been tested here; terminal event forwarding is required. This is small
stepped zoom, not raw touch/pinch or pixel-smooth animation.

## Native graphics default update

Removed automatic Ghostty+tmux Halfblocks selection; safe/halfblocks are now
explicit opt-in. Updated status, regression test and README. Historical mentions
of automatic protection below are superseded. Ran check, Typst tests/Clippy,
release, PDF and Typst PTY scripts successfully. Real separate Ghostty/tmux with
`LEAFVIEW_GRAPHICS=auto`, user's letuchka.typ and --root ../..: confirmed Kitty
placeholders, no Halfblocks status, zoom/scroll/page keys and normal shell return;
no Kitty/checksum errors logged in this short run. Installed with Typst feature.
This short smoke does not certify long-run absence of all protocol errors.

## Optional Cargo feature update

Typst is now opt-in: `cargo install --locked --features typst --path . -j 2`.
Default builds exclude all four optional Typst dependencies, compiler/live modules,
and compiler integration tests. A PDF-only binary rejects `leafview typst ...`
with the exact installation hint before terminal entry. CI now tests both feature
configurations. README updated accordingly; the historical install command below
predates this change.

Rechecked check/test/clippy for both configurations, PDF-only release + PTY smoke,
Typst release + live PTY smoke (save-to-display 0.653 s), feature-disabled CLI
regression test. Installed successfully using `--features typst`.
No new real-terminal test for this packaging-only update; prior visual results
below apply to the same compiler/navigation implementation.

`typst watch` is not a source-map protocol: status/dependencies don't tell the
changed PDF page or coordinates. The retained compiler world remains necessary
for exact same-generation source mapping; no second watch compiler was added.

## Compile flags update

Added `src/typst_options.rs`: options before/after input, --key=value, root,
sys.inputs, fonts, package/cache paths, certificates, timestamp, jobs, features,
PDF standards/tags/pretty, page ranges, PDF format. Options serialized to the
same compiler helper; page selection adjusts source-map page numbers. Help lists
all compile/watch flags and explicitly rejected export/server side effects;
this is NOT full CLI compatibility. Diagnostic/color accepted but status remains
short/sanitized, PPI is PNG-only. No global environment mutation.

Tests pass for default and typst builds, Clippy both, release + both PTY scripts.
New compiler test verifies sys.inputs in actual PDF text, selected 2 PDF pages,
source coordinates renumbered, PDF standard/pretty/no-tags/fixed date/font options.
Parser test covers relative root, spaces, equals syntax, repeated-value input,
unknown/non-PDF rejection. The user's exact relative-root command was launched
in a disposable real Ghostty/tmux from the supplied project directory: compiled
letuchka.typ successfully, page 1/2, initial compile 126 ms, clean q/shell return.
User source not edited. Installed again with --features typst.

Remaining CLI differences are explicitly documented in README: deps/timings/open
and HTML server/export are rejected rather than implemented; helper startup
validation failures still have generic resource/internal error feedback.

## Implementation and choice

Base: clean main/origin/main at 1fdbb0d, fetched before work. Separate branch
`feature/typst-live-preview`; no main merge or push.

Installed CLI is Typst 0.15.1. Mason Tinymist also embeds 0.15.1 but isn't on
PATH. Inspected installed CLI compile/watch help, local typst/typst-syntax/
typst-layout/typst-kit/typst-pdf 0.15.1 sources, official typst-ide jump.rs
(v0.15.0), and current Tinymist/typst-preview source (research checkout in /tmp).
Official `jump_from_cursor<D: JumpInDocument>(document, source, cursor)` finds a
Text/MathText syntax leaf then searches layout glyph spans. `Source::find(span)`
provides its source range; PagedDocument pages provide glyph coordinates, group
transforms and page dimensions. These actual APIs compile against pinned 0.15.1.

Options considered:
- `typst watch`: excellent incremental compilation, no CLI source-map export.
- Tinymist preview/LSP: incremental mapping available, but coordinating separately
  requested PDF export and source query needs revision barriers; dependency on a
  changing external protocol and installed server. Not used.
- Rust compiler helper (chosen): pinned APIs, one retained FileStore/world,
  comemo eviction, PDF and glyph map from one PagedDocument. Compiler memory and
  failure are isolated from UI. No OCR, image comparison or modified source
  instrumentation. No extra PDF renderer. Text glyph span offsets are used;
  MathText maps at token granularity (including multi-digit math tokens).

Tradeoff: heavyweight dependencies, ~47 MiB stripped release executable,
~605 MiB debug executable locally. First full release build took multiple
minutes; incremental release link with thin LTO ~2 min. Dependency lock expanded;
only Typst family versions explicitly pinned, no intentional downgrade.

## Files

- Cargo.toml/Cargo.lock: pinned compiler API dependencies.
- src/typst_compiler.rs: private owned compiler process, font/world setup,
  dependency/source snapshots, glyph mapping, PDF export, atomic metadata reply.
- src/typst_live.rs: worker orchestration, process limits/deadline/kill+wait,
  dependency-parent watcher, debounce, generations, source diff/mapping selection.
- src/main.rs: CLI, async startup with blank placeholder, document publication,
  existing Destination-based scroll navigation, sync keys/status, optional trace.
- tests/fixtures/typst-live/: 10-page include/math/table fixture.
- tests/typst.rs: same-process incremental compiler/PDF/map/error tests.
- scripts/typst-smoke.py: PTY live editing integration.
- README.md and this report.

src/kitty.rs, renderer/cache/workers/process implementation unchanged. Both
modes share existing graphics selection and safety policy. No Tectonic or
unused backend abstraction introduced. Sources/PDFs in user projects not edited.

## Lifecycle and bounds

One running compile; latest pending generation, 200 ms filesystem debounce.
Stale results rejected; changes during initial source loading trigger retry.
Compiler FileStore reset retains source identity for incremental compilation.
Only dependency parents watched non-recursively, updated after compiles including
failed dependency attempts. PDF publication happens only after successful PDF
export and immutable Document snapshot. Private temp output never touches the
user's main.pdf. Helpers killed/reaped by owned worker shutdown; sources capped
16 MiB, map 500000 entries / 96 MiB serialized, compiler address space 3 GiB,
wall deadline 60 seconds. Worker results bounded to one. Compilations are not
preempted on every save; latest work waits for running job to finish/deadline.

Diff is linear UTF-8 prefix/suffix comparison selecting the last differing
character (deterministic last-hunk anchor), not a full edit-script algorithm.
Deletion selects a surviving nearby glyph. Repeated spans across pages and
unmapped style/resource edits preserve position. It cannot reconstruct actual
keystroke order within a single save. Includes use compiler dependencies.
Manual navigation never repeatedly reapplies old follow; s toggles, f follows.
Zoom unchanged; existing Destination conversion and crop bounds do final clamp.

## Executed checks

All PASS:
- cargo fmt --all -- --check
- cargo check --locked
- cargo test --locked (25 unit + 3 CLI + 1 compiler integration passed;
  one optional profiling test ignored)
- cargo clippy --locked --all-targets -- -D warnings
- cargo build --locked --release
- python3 scripts/pty-smoke.py (ordinary PDF regression and signal cleanup)
- python3 scripts/typst-smoke.py
- cargo test --locked --release (earlier implementation iteration)
- git diff --check

Fixture formatted and checked with Mason typstyle using --wrap-text
--line-width 70 --indent-width 2. Owned CLI typst watch compiled fixture
successfully after formatting (10 ms). Page 7 raster inspected via pdftoppm.

Compiler test: 10-page include on page 7; mapped formula has baseline coordinates
inside expected page region; added leading page shifts map to page 8; malformed
source emits error and no new PDF; repair compiles generation 4. UTF-8 insertion,
deletion, multiple changes, unchanged text diff unit cases.
PTY: paths with spaces, included formula save, zoom 150%, nonzero vertical crop,
manual page navigation persists, syntax error retains app, 40 rapid atomic saves,
repair, sync-off retains page 1, f follows page 7, q restores exact termios.
Final save-to-accepted-image observed 0.652 s (includes 200 ms debounce, compile,
Document/Poppler, encoding, UI polling). This is a single fixture measurement,
not a latency guarantee. Real 10-page incremental compiles in status: 1–3 ms;
initial compile 7–20 ms. Total compile time varies with fonts/project complexity.

## Real Ghostty/tmux/Neovim

Separate Ghostty 1.3.0-dev (X11 for screenshot), isolated tmux 3.6 socket
leafview-typst-real; allow-passthrough and focus-events enabled only there.
Neovim -u NONE editing chapters/theory.typ next to native LeafView. Real Neovim
:write and atomic-save events, not only simulated PTY events.

Confirmed page 7 follow, manual g then next edit returns to 7; syntax error
shows `Typst: unclosed delimiter`, fixing save recovers. At 150% changed c^34
to c^35: accepted geometry page=6 (zero-based), raster=2184x4368,
crop=(711,1470,1456,1692). XGetImage window screenshot inspected: crisp E=mc^35
visible near upper third, pane confinement intact. Table partly cropped at this
horizontal zoom is expected: target formula is brought into view, not entire
page. Screenshot /tmp/leafview-typst-150.png; full-page view at 50% also inspected
(/tmp/leafview-typst-window.png). No Kitty/checksum errors in this test's log.
This short successful test does NOT resolve previous Ghostty/tmux graphics
corruption under intensive pane resizing; AUDIT-2026-10-19.md still applies.
All own compiler processes gone after q; existing user processes untouched.

Installed via cargo install --locked --path . --offline -j 2. `type -a leafview`
resolves /home/yaroslav/.cargo/bin/leafview, SHA-256 matches target/release/leafview:
eecc5610e41062497c2f01f1040fce4921b5c047fe3de85dc71dc9efb8d26726.
Final installed-command Ghostty/tmux launch shows Typst ready page 1/10, normal
conservative graphics policy; q returns shell and prints TTY_AFTER_EXIT.

## Remaining limitations / not claimed

- No long-duration memory soak, broad real-document corpus or complete automated
  11-scenario acceptance suite; deletion/multiple changes have unit rather than
  real Neovim end-to-end coverage. Repeated open/close leak soak not completed.
- Generated expressions/shapes/images and arbitrary styling may not map; repeated
  glyphs/locations are conservatively skipped. No exact selection rectangle.
- Event ordering across different dependency directories is backend delivery order,
  not a timestamp proof of editor keystroke chronology. Events are bounded;
  overflow forces recompile with deterministic fallback ordering.
- Resource changes trigger builds but do not provide source-text follow. Newly
  introduced unreadable/missing nested dependency directory may require r after
  creation if its parent could not initially be watched.
- Root is entry parent or TYPST_ROOT, not automatic discovery of arbitrary project
  manifests. CLI inputs/package overrides, PDF standards/custom timestamps and
  warning diagnostics UI not yet exposed. Helper errors are shortened; stderr
  from compiler startup is suppressed. Network package fetch obeys helper timeout.
- Source byte validation after compile is in the worker; a filesystem edit racing
  publication is additionally caught by watcher generation, but no filesystem
  transaction across all input files is promised.
- Optional LEAFVIEW_TRACE writes accepted geometry JSON lines; diagnostic only,
  file append on UI thread and unbounded log if explicitly enabled. Off by default.
- SIGKILL/OOM cannot run RAII; compiler kill-on-parent-SIGKILL isn't implemented.
  Normal q and handled signals join workers and kill/reap compiler.
