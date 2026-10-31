#import "settings.typ": settings
#let document(body) = {
  set page(
    width: settings.width,
    height: settings.height,
    margin: settings.margin,
    numbering: "01",
    footer: align(left)[#text(
      size: 8pt,
      fill: settings.accent,
    )[LEAFVIEW / NUMERICAL METHODS]],
  )
  set text(font: "Libertinus Serif", size: 11pt, fill: settings.ink)
  set par(justify: true, leading: 0.65em)
  set heading(numbering: none)
  show heading: set text(font: "DejaVu Sans", fill: settings.ink)
  body
}
