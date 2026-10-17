// Brief an einen Kunden.
#import "briefpapier.typ": *

#let d = json("daten.json")
#show: briefpapier.with(d, folgekopf: d.folgekopf, titel: d.betreff)

#briefkopf(d, d.adresse, d.info)

#if d.betreff != "" {
  text(weight: "bold", d.betreff)
}

#richtext(d.text)

#if d.gruss != "" {
  v(4mm)
  d.gruss
  v(12mm)
  d.unterschrift
}
