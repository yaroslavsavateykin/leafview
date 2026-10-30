# Conflicts

Resolution first selects the highest focused context, then exact mode or `any`.
Within that trie, higher priority wins; equal priority uses the later declaration.
Overrides are retained as provenance and visible in Keyboard Shortcuts.

Exact duplicate endpoints are conflicts/overrides. A command that is also a prefix
(e.g. `go` and `goo`) is valid and deferred until timeout; prefix overlap is shown
separately. Escape cancels without running the deferred command. Unknown prefix
continuations replay the pending strokes through the legacy handler, preserving
its native grammar. The implementation does not yet execute the shorter command
and reprocess a mismatching suffix as a fresh binding.

Tested: g/gg/go/goo, Space o/Space oa/Space on, leader, localleader, numeric count,
mode isolation, input fields, override and tombstone behavior.

Reference audit found **37 context/source-variant groups** in the union of two
runs. These are not all simultaneous conflicts: a buffer-local mapping, a loaded
lazy callback, a filetype or a different source line can replace an earlier map.
The JSON explicitly marks variants rather than pretending to recover overwritten
maps from nvim_get_keymap. 7 inactive templates are excluded from runtime counts.

Resolved routing conflicts: completion Ctrl-P vs file picker, Ctrl-J/K vs pane
navigation, editor text vs PDF/Explorer character bindings, palette/settings text
vs global Vim shortcuts. Native Vim fallback retains keys not yet migrated.

Current gaps: duplicate ancestor/child context interactions are not exhaustively
reported; deep prefix conflict reporting only visits directly adjacent endpoints;
terminal aliases are documented, not automatically normalized into all possible
protocol-equivalence groups. Recording checks central bindings but not every
hardcoded legacy Vim key. These limitations must be addressed before claiming
all conflicts can be detected.
