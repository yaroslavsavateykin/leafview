#set page(width: 300pt, height: 600pt)
#for i in range(1, 7) {
  heading("Page " + str(i))
  [Ordinary text.
    $ a^2 + b^2 = c^2 $]
  pagebreak()
}
#include "chapters/theory.typ"
#pagebreak()
#for i in range(8, 11) {
  heading("Page " + str(i))
  [Final text.]
  if i < 10 { pagebreak() }
}
