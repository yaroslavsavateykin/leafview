# Verification status

Prepared 2026-10-18.

## Verified in authoring environment

- `Cargo.toml` parses as TOML.
- Test PDF fixture has three pages according to `pdfinfo`.
- `pdftoppm -f 2 -l 2 -singlefile -scale-to-x 800 -scale-to-y -1 -png tests/fixtures/sample.pdf` produces a valid 800 × 1133 PNG on Linux.

## Not verified in authoring environment

- Rust compilation, formatting, Cargo unit/integration tests and clippy: Rust/Cargo are not installed and outbound network is unavailable.
- Tmux graphical display, placeholder positioning, switching panes, detach/attach.
- Ghostty and Kitty image integration.

Do not assume these tests pass merely because code or CI exists. Run `cargo fmt`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release` in a Rust-enabled environment. The `scripts/publish-private.sh` helper refuses to create a remote GitHub repository unless build and tests pass.
