# Installation

[Documentation](README.md) · [Project](../README.md)

## Requirements

Linux, Git and a current stable Rust toolchain. The renderer calls `pdfinfo`,
`pdftoppm` (Poppler) and `prlimit` (util-linux). On Ubuntu 24.04+:

```sh
sudo apt update
sudo apt install poppler-utils util-linux
git clone https://github.com/yaroslavsavateykin/leafview.git
cd leafview
cargo install --locked --path .
```

Install Rust through [rustup](https://rustup.rs/) if needed. Older distribution
Rust packages may not support the dependencies in Cargo.lock. Cargo also fetches
the pinned Helix Git dependency when building the editor.

## Choose a build

```sh
# PDF viewer; no compiler or editor
cargo build --locked --release
# PDF + bundled Typst compiler
cargo build --locked --release --features typst -j 2
# Full document workspace
cargo build --locked --release --features typst,editor -j 2
./target/release/leafview --help
```

Use `cargo install --locked --path . --features typst,editor -j 2` to install
the workspace in Cargo's bin directory. Ensure that directory is on PATH.
The Typst dependency graph is substantial; limiting jobs reduces simultaneous
compiler memory use, but is not a memory guarantee.

| Feature | Adds |
| --- | --- |
| `typst` | Embedded Typst 0.15.1 compiler and source-to-preview mappings |
| `editor` | Helix Core buffer engine and native source editor |
| `tinymist` | Compatibility feature alias enabling `editor`; no bundled server |
| `typstyle` | Compatibility feature alias enabling `editor`; no bundled formatter |

Default features are empty. `editor` alone can edit source beside a PDF;
Typst compilation still requires `typst`.

## Optional external tools

For Typst editing, install [Tinymist](https://github.com/Myriad-Dreamin/tinymist)
and [Typstyle](https://github.com/typstyle-rs/typstyle) and put their executables
on PATH. Choose versions compatible with your document's Typst version.
The embedded compiler does not need a separately installed `typst` CLI.

Other editor integrations use rust-analyzer/rustfmt, basedpyright-langserver/black,
clangd/clang-format and typescript-language-server/prettier. See the
[editor guide](editor.md). HTTP(S) PDF links need `xdg-open` from `xdg-utils`.

## Terminal requirements

For sharp pages, use a terminal supporting the negotiated native graphics
protocol. The showcase uses Kitty with inline PNG/Unicode placements. Halfblocks
work as an explicit low-resolution fallback. Terminal, tmux and SSH details are
in [configuration](configuration.md); do not force Kitty on an unsupported terminal.
