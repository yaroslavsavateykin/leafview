# Verification — 2026-10-18

## Interactive hyperlinks update

Implemented annotation-based URI and local GoTo links in `src/links.rs`, including
indirect objects, legacy named destinations and name trees. Poppler remains the
renderer. Chosen parser: lopdf 0.45 without default features. Poppler GLib was
considered but requires new GLib/Cairo/system-library dependencies; pdfinfo URL
listing does not provide the required annotation geometry/destinations. Serde and
serde_json carry bounded helper output, not PDF actions or executable code.

Parsing is eager once per immutable snapshot in a separate resource-limited
Leafview helper (`--extract-links`), called by the existing document worker.
Helper parsing is cancellable via process termination and has the same 12 s /
512 MiB process limits as rendering; metadata output is capped at 32 MiB.
This favors bounded shutdown and fault isolation over an in-process parser that
cannot be interrupted. The parsed PDF is discarded; compact metadata stays with
its snapshot. No mouse event parses a PDF. Large PDFs can hit metadata limits and
remain viewable with links disabled; lazy page extraction is not implemented.

Current regression run: `cargo fmt --all -- --check`, `cargo check --locked`,
`cargo test --locked`, Clippy `-D warnings`, release build, `git diff --check`:
PASS. **19 unit tests and 3 CLI integration tests passed**, one optional older
performance test remains ignored. New tests cover URI/GoTo/name-tree/legacy-name
extraction, unsafe actions, actual Typst PDF, PDF Y inversion, rotations 0/90/180/270,
nonzero MediaBox/CropBox, zoom 50/100/125/200/300%, horizontal/vertical/both-axis
scroll, centered image, different cell dimensions, ambiguous/tiny/outside regions,
stale image identity, destination offsets/fit and URL argument/sanitization rules.
A Poppler pixel test rasterizes red annotation rectangles and confirms their
transformed centers are actually red on all four rotations (not just a transform
round-trip test). Fixtures and deterministic generator are checked in; Typst
fixture source is included, Typst is not required for normal tests/CI.

Extended PTY test: PASS. SGR mouse motion previews DOI, 50 repeated moves within
one link cause **zero additional terminal output**, hover never opens a browser,
click passes the exact DOI to an injected PATH-local mock `xdg-open`, launcher
failure is reported without exit, internal click and `b` restore source location.
Also reruns rapid keyboard input, corrupt/atomic reload, 1×1 resize, termios
restoration and mouse capture disable. Latest run: viewer peak RSS 31056 KiB,
quit 0.022 s. The script clears captured output between assertions, so its final
byte count is not total session transmission volume.

### New real-terminal status (not inherited from earlier graphics tests)

Ghostty + isolated tmux 3.6 launched with the Typst hyperlink fixture. Pane capture
confirmed internal-link hover status (`→ Page 2 [click to follow · b back]`).
However XWayland pointer injection / screenshot capture in this environment did
not provide a consistent synchronized interactive view: screenshots lagged the
current tmux state. Therefore **real visual hyperlink click, browser opening,
horizontal/vertical split interaction, zoom/scroll clicking and detach/attach
hyperlink interaction are NOT VERIFIED**. The PTY tests above exercise application
mouse events but are not claimed as end-to-end physical mouse testing. Kitty,
Sixel and iTerm2 hyperlink behavior: NOT TESTED. Real browser opening: NOT TESTED
(the automated test intentionally uses a mock). Existing earlier graphics checks
below predate hyperlinks and do not certify the new interaction paths.

Hover uses cyan destination feedback in the status bar, no PDF-region overlay,
1016 pixel mode or hand cursor. Rectangular Link annotations only; QuadPoints,
FitB-family, GoToR, JavaScript and Launch are unsupported. Null XYZ coordinates
preserve corresponding scroll coordinates; zoom/fit are constrained by existing
50–300% and raster limits. History is cleared on successful reload.

Manual hyperlink check: open `tests/fixtures/typst-links.pdf` in the active pane;
hover DOI without clicking (browser must not open), click Go to target (page 2),
press `b`, click DOI and verify browser. Repeat after +/- and H/L/j/k, pane splits,
resize and fresh-terminal attach; verify blank/status/neighbor regions never
activate. Repeat with `tests/fixtures/links.pdf` to cover rotated pages and
unsupported-action feedback. No tmux mouse setting is changed by Leafview.


Source: supplied `leafview-rust-source.zip`, imported unchanged in commit
69e825b before modifications. Tests below were actually executed, not inferred
from successful compilation. No Fancy Cat source was modified or used as base.

## Automated checks: PASS

- `cargo fmt --all -- --check`
- `cargo check --locked`
- `cargo test --locked`: 12 unit tests + 2 CLI integration tests passed.
  One optional synthetic performance test is ignored in the default run and was
  executed separately with `--release --ignored --nocapture` (PASS).
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo build --locked --release`
- `cargo install --locked --path . --root /tmp/leafview-install-test`
- `git diff --check`, `bash -n scripts/publish-private.sh`
- Cargo.lock generated by Cargo and tracked. No dependency versions guessed or
  downgraded: Ratatui 0.30.2, ratatui-image 11.0.8, Crossterm 0.29.0, Rayon 1.12.0
  compiled together successfully (Rust 1.98.1).

Tests cover page bounds, zoom limits and NaN/Infinity rejection, saturating
scroll/crop geometry including empty dimensions, 1000 navigation/geometry
operations, byte-LRU eviction, snapshot preservation after damaged replacement,
missing PDF, cancellation and subprocess output limits, actual Poppler rendering,
1000 latest-wins renderer requests and joined shutdown.

`python3 scripts/pty-smoke.py`: PASS. Real pseudoterminal, halfblocks fallback;
over 1000 key events, corrupted PDF auto-reload, atomic replacement, twenty
resizes including 1×1, normal exit, exact termios restoration and alternate-screen
exit sequence. Final recorded run: peak viewer RSS 35824 KiB, quit 0.027 s,
218829 output bytes. This measures viewer RSS only, not aggregate subprocess or
outer-terminal memory. PTY assertions do not replace visual inspection.

## Real visual checks

Terminal: Ghostty 1.3.0-dev+0000000 under XWayland; tmux 3.6, isolated server
`tmux -L leafview-test -f /dev/null`. Window screenshots captured through X11
XGetImage and inspected directly during the session.

| Check | Result |
| --- | --- |
| Ghostty direct PDF graphics | PASS: actual fixture text/vector lines visible |
| Ghostty + tmux graphics | PASS: Kitty Unicode placeholders display PDF |
| Split pane confinement | PASS: PDF ends at divider; neighbor text unobscured |
| Page navigation / zoom / scroll in tmux | PASS: page 3, 125%, scrolled image |
| Pane resize | PASS: re-rendered at new pane width |
| Switch pane / focus return | PASS with focus-events enabled; no hang observed |
| Live Typst compilation | PASS: new `Reloaded live` heading visible without restart |
| Reload preserves page/zoom | PASS: status remained 3/3, 125% |
| Detach and attach in a NEW Ghostty window | PASS with `set -g focus-events on` and pane focus return |
| Quit / image cleanup | PASS: `LEAFVIEW_EXITED` text, no residual PDF, neighbor intact |
| Kitty emulator itself | NOT TESTED (not installed) |
| Sixel / iTerm2 | NOT TESTED |

Confirmed limitation: without forwarded focus events a new outer terminal may
show blank Kitty placeholders after attach. `r` restores the image. Enabling tmux
focus-events plus Leafview's focus tracking fixed this in the observed scenario.
Initial capability queries must occur while the Leafview pane is active; terminal
responses can otherwise be routed by tmux to another active pane. Do not launch
capability-querying programs in an inactive pane using external automation and
assume their negotiation result proves terminal support.

## Synthetic performance (release build)

Generated with `python3 scripts/make-bench-fixtures.py`; first page at 1000 px,
one run per case (not statistically rigorous). All five cases opened/rendered.

| PDF | Pages | Snapshot + pdfinfo | Render + PNG decode | Kitty encode, 1000×700 viewport |
| --- | ---: | ---: | ---: | ---: |
| Small text | 1 | 31 ms | 107 ms | 6.5 ms |
| Long text | 120 | 31 ms | 105 ms | 4.5 ms |
| Vector lines | 175 | 21 ms | 106 ms | 5.8 ms |
| Large raster (3000×3000 source) | 1 | 35 ms | 880 ms | 5.3 ms |
| Typst math | 3 | 21 ms | 158 ms | 5.6 ms |

1000 repeated cached lookups: 37–50 microseconds total, with pointer identity
assertions proving raster reuse. Crop copy: 1.8–3.1 ms. Scroll uses the same raster
cache key: no Poppler render is required on a hit. Actual application workload
cache hit percentage was NOT MEASURED. Render and PNG decode were measured
together, not separately. End-to-end key-to-photon latency and terminal
transmission time were NOT MEASURED. The event loop polls at up to 30 ms; that is
not a latency benchmark. No long-duration leak profiler/Valgrind or multi-hour
RSS soak was run: NOT TESTED. Bounded ownership and short stress tests are not
proof of universal leak/deadlock freedom.

## Three-page bidirectional prefetch update

Prefetch now prepares distance 1, then 2, then 3 on both sides with two workers.
The moving window discards obsolete revision/width/distant rasters; speculative
admission never evicts existing rasters and respects the 96 MiB budget. Fewer
pages may fit at high resolution. Added unit coverage for document-edge and
integer-overflow bounds and budget admission preserving the current page.

Re-ran format, check, tests (12 unit + 2 CLI passed, optional benchmark ignored),
Clippy with warnings denied, release build, diff whitespace check and PTY smoke:
PASS. Updated PTY run: peak viewer RSS 27388 KiB, quit 0.024 s. This short fixture
run does not establish the performance gain of six-page prefetch on large PDFs;
that gain and new real-terminal performance are NOT MEASURED. Remote CI for this
update is checked separately after pushing.

## Reproduce real-terminal checks

1. Build release and generate synthetic fixtures as above.
2. In Kitty or Ghostty start `tmux -L leafview-manual -f /dev/null`.
3. `tmux set -g focus-events on`; split; run Leafview in the active pane.
4. Open each generated PDF. Rapidly use h/l, j/k, H/L, +/-; try digits+Enter,
   g/G, PageUp/PageDown, Space, Esc cancellation. Verify displayed page text.
5. Repeatedly resize and switch panes. Confirm no overlap with neighbor text.
6. Run `typst watch /tmp/leafview-bench/math.typ`; edit/save while Leafview stays
   open. Check changed content and retained page/zoom. Test a shorter PDF too.
7. Detach, close the outer terminal, attach from a new terminal, return focus to
   Leafview. Confirm reconstruction; also test `r` recovery without focus events.
8. Quit during navigation/rendering. Check terminal echo/cursor, residual images,
   and no remaining Leafview/Poppler processes.

CI is configured on Ubuntu 24.04 for all Cargo checks and the separate PTY test.
Remote CI run [39149065261](https://github.com/yaroslavsavateykin/leafview/actions/runs/39149065261)
completed successfully on Ubuntu 24.04: format, check, test, clippy, release build
and PTY smoke test all passed (2m50s). Repository privacy confirmed through GitHub
API (`isPrivate: true`) at initial publication. The author subsequently explicitly
requested public publication; repository visibility is now intentionally public.
Subsequent commit updates checkout to its Node 24 release
and records this observed CI result.
