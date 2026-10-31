# Contributing

Small, focused changes are welcome. Please describe the behavior being changed,
how to reproduce it and which checks you ran. For renderer issues, include Linux,
terminal, graphics backend, tmux version (if used), dimensions and a synthetic PDF
or Typst fixture. Do not attach private documents, config files or recent history.

## Local checks

```sh
cargo fmt --all -- --check
cargo check --locked --all-features
cargo test --locked
cargo test --locked --all-features
cargo clippy --locked --all-features --all-targets -- -D warnings
cargo build --locked --release --features typst,editor -j 2
```

Some tests need external programs or generated fixtures; inspect failures rather
than treating an omitted integration test as a pass. Current showcase verification
results are in [docs/showcase-verification.md](docs/showcase-verification.md).
The existing CI matrix covers default and `typst` builds, not every editor feature.

See [development](docs/development.md) for PTY smoke tests and architecture.
Use [the showcase environment](scripts/showcase/README.md) for public screenshots;
never use personal projects or modify global terminal settings to capture them.

## Expectations

- Preserve dirty buffers and last-good documents on errors.
- Keep render/compiler work bounded, cancellable and owned through shutdown.
- Do not advertise unsupported terminal protocols or Vim/LSP parity.
- Document new keys and feature requirements; update media when UI changes.
- Keep original third-party license notices. LeafView is MIT; Helix components
  retain MPL-2.0 terms. Do not commit secrets, build output or personal state.
