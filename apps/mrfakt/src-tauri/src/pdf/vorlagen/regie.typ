// Regiebericht (Stundenlohn-/Tagesbericht).
#import "briefpapier.typ": *

#let d = json("daten.json")
#show: briefpapier.with(d, folgekopf: d.titel, titel: d.titel)

#briefkopf(d, d.adresse, d.info)

#text(size: 14pt, weight: "bold", d.titel)
#if d.betreff != "" {
  v(-2mm)
  strong(d.betreff)
}

#set text(size: 9.5pt)
#for r in d.berichte {
  block(above: 5mm, breakable: true, {
    strong[#r.datum #r.titel]
    v(-2mm)
    line(length: 100%, stroke: 0.4pt)
    let n = if d.preise { 6 } else { 4 }
    let zellen = ()
    for z in r.zeilen {
      if z.art == "T" {
        zellen += ([], table.cell(colspan: n - 1, zeilen(z.text)))
      } else {
        zellen += (z.posnr, z.text, align(right, z.menge), z.einheit)
        if d.preise { zellen += (align(right, z.ep), align(right, z.gp)) }
      }
    }
    let spalten = (16mm, 1fr, 20mm, 14mm)
    if d.preise { spalten += (20mm, 22mm) }
    table(columns: spalten, stroke: none, inset: (x: 1mm, y: 1mm), ..zellen)
  })
}

#v(16mm)
#grid(
  columns: (70mm, 1fr, 70mm),
  [#line(length: 100%, stroke: 0.4pt) #text(size: 8pt, [Datum, Unterschrift Auftragnehmer])],
  [],
  [#line(length: 100%, stroke: 0.4pt) #text(size: 8pt, [Datum, Unterschrift Auftraggeber])],
)
