# Getting started

[Documentation](README.md) · [Project](../README.md)

## Read a PDF

```sh
leafview document.pdf
```

Documents open in Fit Page mode. Use `n` / `p` for pages, digits + Enter to
jump, `z` for Fit Width and `0` for Fit Page. `hjkl` pans; `+` / `-` zooms.
Wheel up/down zooms around the pointer, rather than scrolling vertically.
The PDF reloads when its file changes. `r` retries a reload.

## Browse documents

```sh
leafview ./documents
# Or browse the current directory:
leafview
```

The welcome screen does not automatically open a document. Select a PDF or
Typst file and press Enter. `F2` toggles the browser; `Tab` cycles focus.
Only PDF/Typst files appear in the tree. `F3` opens a picker for the active
pane; the source picker also includes common text/code formats.

## Edit Typst beside the preview

Build with `typst,editor`, then:

```sh
leafview main.typ
# Entry in a subdirectory with imports from the project:
leafview documents/introduction.typ --root .
```

1. Press `F4` to show the source pane. Use `F6` to switch source/PDF focus.
2. Press `i` to insert text. Escape returns to Normal mode.
3. Type `:w`, then Enter. The saved source triggers an incremental rebuild.
4. Use `Ctrl+Space` for completion with an external language server installed.
5. Press `F2` for the tree; drag dividers to adjust the layout.

Format-on-save is enabled for recognized source formats and needs an external
formatter. Use `:set noformatonsave` when no formatter is installed, or `:format`
to run it explicitly. Completion browsing does not insert until acceptance by
default. `F1` opens hover documentation; `F7` cycles available tool windows,
`F8` closes a tool.

The preview is private by default: it does not overwrite a PDF beside your
source. `--write-pdf` opts into export; a saved per-file export setting is also
respected. See [Typst](typst.md) for configuration and source-map limitations.

## Try the showcase safely

The checked-in [showcase scripts](../scripts/showcase/README.md) create an
independent temporary HOME and neutral documents. They never clear your recent
files or reuse your application configuration.

## Exit

`Ctrl+C` exits. Unsaved source buffers require a second confirmation. `q` is
not a global quit shortcut. Normal exits restore terminal modes; SIGKILL cannot
run cleanup.
