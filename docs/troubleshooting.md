# Troubleshooting

[Documentation](README.md) · [Project](../README.md)

## Common problems

- Missing command: install `poppler-utils util-linux`; ensure `pdfinfo`,
  `pdftoppm`, `prlimit` are on PATH.
- Poppler failure: PDF may be incomplete, damaged, encrypted, oversized or exceed
  the renderer deadline/resource cap. Try `pdfinfo document.pdf` outside Leafview.
- Reload error during compilation: old PDF stays visible; save again or press `r`.
- Blank image after attaching elsewhere: enable tmux focus events, select the pane
  or press `r`. Check outer-terminal graphics support and tmux passthrough policy.
- Low-resolution colored blocks: capability negotiation selected halfblocks.
- SIGKILL cannot run cleanup in any application. If externally killed, use
  `reset` if necessary; normal `Ctrl+C` exit restores the terminal.
