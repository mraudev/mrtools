// Etiketten (Adressen oder Artikel) auf A4-Bögen.
#let d = json("daten.json")
#set text(font: ("Arial", "Liberation Sans", "Libertinus Serif"), size: 9pt, lang: "de")
#set page(paper: "a4", margin: (x: 7mm, y: 12mm))
#let breite = (210mm - 14mm) / d.spalten
#grid(
  columns: (breite,) * d.spalten,
  rows: d.hoehe * 1mm,
  ..d.etiketten.map(e => block(width: 100%, height: 100%, inset: 4mm, clip: true, {
    if e.len() > 0 { strong(e.at(0)) }
    for z in e.slice(calc.min(1, e.len())) {
      linebreak()
      z
    }
  })),
)
