// Aufmaßblatt zu einem Beleg.
#import "briefpapier.typ": *

#let d = json("daten.json")
#show: briefpapier.with(d, folgekopf: d.titel, titel: d.titel)

#briefkopf(d, d.adresse, d.info)

#text(size: 14pt, weight: "bold", d.titel)
#if d.betreff != "" {
  v(-2mm)
  strong(d.betreff)
}

#set text(size: 9pt)
#for p in d.positionen {
  block(breakable: true, above: 4mm, {
    grid(columns: (14mm, 1fr, 34mm), strong(p.posnr), strong(p.kurz), align(right, strong[#p.summe #p.einheit]))
    if p.zeilen.len() > 0 {
      let zellen = ()
      for z in p.zeilen {
        if z.art == "K" {
          zellen += ([], table.cell(colspan: 6, emph(z.text)))
        } else if z.art == "R" {
          zellen += ([], table.cell(colspan: 6, text(fill: grau, [= #z.text])))
        } else {
          let erg = if z.abzug { text(fill: rgb("#b00000"), z.ergebnis) } else { z.ergebnis }
          zellen += (z.formel, z.text, align(right, z.anzahl), align(right, z.m1), align(right, z.m2), align(right, z.m3), align(right, erg))
        }
      }
      table(
        columns: (14mm, 1fr, 12mm, 18mm, 18mm, 18mm, 22mm),
        stroke: none,
        inset: (x: 1mm, y: 0.9mm),
        table.hline(stroke: 0.3pt),
        text(fill: grau)[Formel], text(fill: grau)[Bezeichnung], align(right, text(fill: grau)[Anz.]),
        align(right, text(fill: grau)[Maß 1]), align(right, text(fill: grau)[Maß 2]), align(right, text(fill: grau)[Maß 3]),
        align(right, text(fill: grau)[Ergebnis]),
        ..zellen,
        table.hline(stroke: 0.3pt),
      )
    }
  })
}
