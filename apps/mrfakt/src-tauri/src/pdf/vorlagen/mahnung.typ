// Zahlungserinnerung und Mahnungen.
#import "briefpapier.typ": *

#let d = json("daten.json")
#show: briefpapier.with(d, folgekopf: d.titel, titel: d.titel)

#briefkopf(d, d.adresse, d.info)

#text(size: 14pt, weight: "bold", d.titel)

#richtext(d.einleitung)

#{
  set text(size: 9.5pt)
  let zellen = ()
  for p in d.posten {
    zellen += (p.beleg, p.datum, align(right, p.betrag), align(right, p.bezahlt), align(right, p.offen))
  }
  if d.gebuehr != "" {
    zellen += ([Mahngebühr], [], [], [], align(right, d.gebuehr))
  }
  table(
    columns: (1fr, 24mm, 26mm, 26mm, 26mm),
    stroke: none,
    inset: (x: 1.2mm, y: 1.4mm),
    table.hline(stroke: 0.6pt),
    strong[Beleg], strong[Datum], align(right, strong[Betrag]), align(right, strong[bezahlt]), align(right, strong[offen]),
    table.hline(stroke: 0.4pt),
    ..zellen,
    table.hline(stroke: 0.6pt),
    table.cell(colspan: 4, strong[Zu zahlen #d.waehrung]), align(right, strong(d.summe)),
  )
}

#richtext(d.schluss)

#v(4mm)
#d.gruss
#v(10mm)
#d.unterschrift
