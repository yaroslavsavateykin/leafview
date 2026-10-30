# Keyboard control in LeafView

Entry points: **F9 Command Palette**, **F10 Keyboard Shortcuts**.
The default preset is `astronvim`. These are application entry points, not claims
that the reference Neovim uses F9/F10 for the same commands.

- [Architecture](architecture.md)
- [Configuration](configuration.md)
- [Current bindings](leafview-bindings.md)
- [Neovim audit](neovim-inventory.md), [machine-readable inventory](neovim-inventory.json)
- [Conflicts and precedence](conflicts.md)
- [Migration report and limitations](migration-report.md)

This implementation adds a central, working command/keybinding layer while
retaining the existing compositional Vim parser. It does **not** claim every
legacy key path or every feature from the requested 17-stage specification has
already migrated. Read the migration report before relying on full parity.
