#import "../style/template.typ": document
#show: document

#text(size: 10pt, fill: rgb("537ca6"))[FIELD NOTES / 01]
= Numerical Methods
#text(size: 16pt, fill: rgb("64748b"))[A short introduction]

#v(12pt)
A good approximation turns a difficult problem into a small,
repeatable calculation. This note explores one simple idea: follow the
local slope, then correct the estimate.

== From a curve to a step
For a smooth function $f$, the first-order expansion gives
$ f(x + h) approx f(x) + h f'(x). $
The remainder decreases as the step becomes smaller.

#figure(
  image("../figures/plot.svg", width: 100%),
  caption: [A smooth curve and its discrete samples.],
)

== Choose the right resolution
#table(
  columns: (1fr, 1fr, 1fr),
  inset: 8pt,
  stroke: (bottom: 0.5pt + rgb("dce4ed")),
  table.header([*Step*], [*Estimate*], [*Error*]),
  [0.100], [0.9048], [0.0048],
  [0.050], [0.9024], [0.0024],
  [0.025], [0.9012], [0.0012],
)

#v(8pt)
#block(fill: rgb("edf3fa"), inset: 12pt, radius: 4pt)[
  *Takeaway* / Halving the step reduces first-order error. Verify
  convergence before trusting a single result.
]

#pagebreak()
== A practical iteration
Start with an initial value and repeat a small update:
$ x_(n+1) = x_n - alpha nabla f(x_n). $

Use a modest learning rate $alpha$ and stop when successive estimates
differ by less than the chosen tolerance.

+ Choose an initial estimate.
+ Evaluate the local slope.
+ Update and check the residual.
+ Repeat until the result stabilizes.

== Keep the experiment reproducible
Record the step size, tolerance and stopping criterion. Compare at
least two resolutions, and retain a small table of results. All values
in this document are illustrative.
