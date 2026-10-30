# Implemented defaults / Neovim correspondence

| Reference / LeafView keys | Command | Status |
|---|---|---|
| Space w | editor.save | AstroNvim match |
| Space c | editor.close | AstroNvim match; protects unsaved |
| ]b / [b | editor.next_buffer / previous_buffer | AstroNvim match |
| Space bp | editor.previous_buffer | existing previous-tab behavior (not exact MRU) |
| Space ff | files.find | existing directory picker, not recursive Telescope |
| Space fr | files.recent | focus existing recent files |
| Space e | explorer.toggle | supported |
| gd / K / gK | lsp.definition / hover / signature | supported |
| Space lR / lf / ld / lh | references / format / diagnostics / signature | supported subset |
| Ctrl-W h/j/k/l | window.left/down/up/right | existing pane manager |
| Ctrl-W v/s/c | vertical/horizontal/hide source | one source view + PDF, not arbitrary splits |
| F7 / F8 | popup.next / popup.close | LeafView-specific |
| F9 / F10 | commands.open / keybindings.open | LeafView-specific |
| Ctrl-PgDown / PgUp | editor.next_buffer / previous_buffer | LeafView-specific |
| Ctrl-Q | editor.close | differs from Astro force quit intentionally for safety |
| Ctrl-Space | completion.show | Insert mode |
| Tab/Down/Ctrl-N/Ctrl-J | completion.next | Completion context |
| Shift-Tab/Up/Ctrl-P/Ctrl-K | completion.previous | Completion context |
| Enter / Ctrl-E | completion.accept / hide | Completion context |
| PDF j/k, n/p, gg/G, +/-/r | scroll, pages, zoom, reload | PDF context |
| Explorer j/k/Enter | next/previous/open | Explorer context |

Normal Vim h/j/k/l, words, operators, text objects, registers, macros, visual,
search and command entry continue through existing Vim parser by default. Users
can shadow those keys with registry commands or tombstones. Not all grammar
productions are individually registered yet.

Unsupported reference categories (not assigned fake commands): embedded shell
terminals, Git UI, Marimo, OpenCode, DAP, notebook execution, most package/session
commands, rename/code actions and semantic LSP actions not present in LeafView.
See machine-readable inventory for each observed mapping.
