# PDF fit and full-width graphics

Opening a new PDF or Typst document now defaults to Fit Page. It computes the
scale from both pixel dimensions of the PDF pane and page aspect ratio, so the
whole page is visible without stretching. Aspect comes from per-page PDF space,
with pdfinfo's page size as fallback when links are unavailable. Fit recalculates
when the pane is resized. Manual zoom switches out of fit mode.

PDF context bindings:

- `0`: `pdf.fit_page` — whole page, reset pan.
- `z`: `pdf.fit_width` — width, reset pan (may require vertical scroll).
- `+` / `-` and pointer zoom: manual scale.
- F9 palette exposes both fit actions; TOML can rebind them.

Percent is relative to pane width, not physical PDF print size. Thus Fit Width
is 100%; Fit Page may be smaller. Small fit scales are allowed below the old 50%
minimum. Changing files resets to Fit Page; recompiling retains the current mode.

Kitty rows now advertise their full terminal cell width to Ratatui, not width 1,
and disable autowrap while printing placeholders at the right edge. This fixes
incorrect diff/cursor assumptions in full-width output. Exact disappearance on
the user's live terminal has not been visually reproduced; PTY protocol tests
exercise direct/tmux transport but are not real Kitty pixel certification.

Checks: 7 view tests, Kitty transport regression, direct/tmux SSH smoke, editor,
completion and keybinding PTY smoke, two Typst integration tests, all-feature
check and combined build passed.
