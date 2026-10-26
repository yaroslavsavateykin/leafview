# Tectonic integration verification

## Environment and actual commands

Linux; Tectonic 0.17.0; Poppler; util-linux; Rust stable; tmux.
The system had no SyncTeX executable. `texlive-binaries` and `libsynctex2`
were downloaded and extracted **only under /tmp** (no sudo/system install).

The default Tectonic bundle request failed with `error decoding response body`
and an upstream CLI panic. To exercise real TeX rather than mock the compiler,
Debian TeX packages were extracted under `/tmp/leafview-texbundle`; a flat local
bundle and isolated `/tmp/leafview-texconfig/tectonic/config.toml` were prepared.
`XDG_CONFIG_HOME=/tmp/leafview-texconfig` selected that bundle. This is an
environment workaround, not a repository dependency or a change to user config.
Real Tectonic ran TeX, BibTeX and xdvipdfmx; actual SyncTeX 1.5 found the included
paragraph on page 2. The second PDF page was rendered with Poppler and inspected:
section, paragraphs, image and formatted bibliography were visible.

Passed:

- `cargo fmt --check`
- `cargo check`, `cargo check --features typst`
- `cargo test`: 32 unit + 4 CLI tests; one pre-existing benchmark ignored
- `cargo test --features typst`: 33 unit + 3 CLI + 2 Typst integration tests;
  one pre-existing benchmark ignored
- `cargo clippy --all-targets -- -D warnings`
- `cargo clippy --all-targets --features typst -- -D warnings`
- `cargo build --release --features typst`
- `python3 scripts/pty-smoke.py`
- `python3 scripts/typst-smoke.py`, also `--no-pdf`
- `python3 scripts/tectonic-smoke.py`, also `--no-pdf`, with actual SyncTeX
- `python3 scripts/tectonic-smoke.py` with SyncTeX absent from PATH
- Tectonic smoke run in a dedicated `tmux -L leafview-tectonic-test` session
- `git diff --check`

Tectonic smoke checks the initial automatic build, ordinary text edits, added and
removed paragraphs, included source, bibliography edit, actual image-pixel
replacement, diagnostics with included filename, last-good PDF retention,
recovery after 30 fast atomic saves, zoom retention, sync toggle/manual follow,
forced rebuild, terminal restoration and exit. Both the project directory and
entry filename contain spaces. SyncTeX's mapped page and stable zoom are checked
through the existing render trace. Resource-only rebuilds retain page/zoom/crop.
`--no-pdf` retains a pre-existing sentinel file across all these operations.
No remaining Tectonic or Typst helper processes were found after the smoke runs.
Unit tests cover prompt cancellation, compiler cwd preservation, escaped Make
syntax, Tectonic's actual unescaped/rebased dependency format, ambiguous/nonfinite
SyncTeX output, last-hunk UTF-8 source diff and stale atomic publication.

## Limits / not claimed

These are headless PTY tests, including one inside tmux, **not** a visual check
of native Kitty/Sixel/iTerm2 transport in a desktop emulator. Native terminal
flicker/overlay behavior still needs a manual terminal session. The PDF render
itself was visually inspected.

SyncTeX accuracy is limited by its line/box mappings. Multiple distinct results
are rejected. Preamble edits in the entry document and deletion-only edits do not
jump; a preamble imported through a separate file cannot always be classified
without parsing TeX. Resource-only edits do not jump. Exotic generated-source
macros may yield mappings that the external SyncTeX tool cannot disambiguate.
Missing mappings do not prevent preview. Rotations/origins use the existing
unit-tested viewer transform; a rotated LaTeX fixture was not tested end-to-end.

Dependencies are discovered from compiler rules and, on failures, log filenames.
A never-successful project with a failing include whose filename cannot be
recovered from the log may require saving the entry file or pressing `r` after
fixing that include. Previously discovered dependencies remain watched.

Limits are per process/per file, not a filesystem quota: no claim of total
project-disk isolation or of a security sandbox for arbitrary malicious TeX.
Tectonic shell escape is disabled. Compilation is limited to 60 seconds and
3 GiB; a cold remote bundle may need warming outside LeafView first.
Default remote bundle downloading was **not verified successfully** in this
network environment; the local bundle exercised the same installed engine.
