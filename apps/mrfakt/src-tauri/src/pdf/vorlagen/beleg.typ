// Angebot, Auftragsbestätigung, Lieferschein, Rechnung, Abschlags-/Schlussrechnung, Gutschrift.
#import "briefpapier.typ": *

#let d = json("daten.json")

#if d.at("zugferd", default: false) {
  pdf.attach("factur-x.xml", relationship: "alternative", mime-type: "text/xml", description: "Factur-X/ZUGFeRD-Rechnungsdaten")
}

#show: briefpapier.with(d, folgekopf: d.folgekopf, titel: d.titel)

#briefkopf(d, d.adresse, d.info)

#text(size: 14pt, weight: "bold", d.titel)
#if d.betreff != "" {
  v(-2mm)
  text(weight: "bold", d.betreff)
}
#if d.leistung != "" {
  v(-2mm)
  text(size: 9pt, [Leistungszeitraum: #d.leistung])
}

#richtext(d.vortext)

#let preise = d.preise
#let kopf(t) = text(size: 8.5pt, weight: "bold", t)

#if d.positionen.len() > 0 {
  set text(size: 9.5pt)
  let spalten = if preise { (13mm, 1fr, 17mm, 12mm, 19mm, 22mm) } else { (13mm, 1fr, 17mm, 14mm) }
  let n = spalten.len()
  let zellen = ()
  for p in d.positionen {
    let text-zelle = {
      if p.kurz != "" { zeilen(p.kurz) }
      if p.lang != "" { linebreak(); text(fill: rgb("#333333"), zeilen(p.lang)) }
    }
    if p.art == "G" {
      zellen.push(table.cell(colspan: n, inset: (top: 3mm, bottom: 1.5mm), text(size: 10.5pt, weight: "bold", zeilen(p.kurz))))
    } else if p.art == "T" {
      zellen.push(table.cell(colspan: n, inset: (top: 2.5mm, bottom: 1mm), strong(text-zelle)))
    } else if p.art == "K" {
      zellen.push([])
      zellen.push(table.cell(colspan: n - 1, text-zelle))
    } else if p.art == "TS" or p.art == "S" {
      zellen.push(table.cell(colspan: n - 1, align(right, strong(p.kurz))))
      zellen.push(table.cell(align: right, stroke: (top: 0.4pt), strong(p.gp)))
    } else {
      let markiert = p.art == "A" or p.art == "B"
      let stil(c) = if markiert { emph(c) } else { c }
      zellen.push(stil(p.posnr))
      zellen.push(stil({
        text-zelle
        if p.art == "A" { linebreak(); text(size: 8pt, fill: grau, [Alternativposition – nicht in der Summe enthalten]) }
        if p.art == "B" { linebreak(); text(size: 8pt, fill: grau, [Bedarfsposition – wird nur bei Bedarf ausgeführt]) }
        if p.rabatt != "" { linebreak(); text(size: 8pt, fill: grau, [abzüglich #p.rabatt % Rabatt]) }
      }))
      zellen.push(align(right, stil(p.menge)))
      zellen.push(stil(p.einheit))
      if preise {
        zellen.push(align(right, stil(p.ep)))
        zellen.push(align(right, stil(if markiert and p.gp != "" { "(" + p.gp + ")" } else { p.gp })))
      }
    }
  }
  let kopfzeile = (kopf[Pos.], kopf[Leistung], align(right, kopf[Menge]), kopf[Einh.])
  if preise { kopfzeile += (align(right, kopf[EP #d.waehrung]), align(right, kopf[GP #d.waehrung])) }
  table(
    columns: spalten,
    stroke: none,
    inset: (x: 1.2mm, y: 1.3mm),
    table.header(
      table.hline(stroke: 0.6pt),
      ..kopfzeile,
      table.hline(stroke: 0.4pt),
    ),
    ..zellen,
    table.hline(stroke: 0.6pt),
  )
}

#if preise and d.summen.len() > 0 {
  align(right, block(width: 100mm, breakable: false, {
    set text(size: 9.5pt)
    let zellen = ()
    for s in d.summen {
      let t = if s.stark { strong(s.text) } else { s.text }
      let b = if s.stark { strong(s.betrag) } else { s.betrag }
      if s.linie {
        zellen.push(grid.cell(stroke: (top: 0.5pt), inset: (top: 1.4mm), t))
        zellen.push(grid.cell(stroke: (top: 0.5pt), inset: (top: 1.4mm), align(right, b)))
      } else {
        zellen.push(t)
        zellen.push(align(right, b))
      }
    }
    grid(columns: (1fr, 30mm), row-gutter: 1.6mm, ..zellen)
  }))
}

#if d.hinweise.len() > 0 {
  v(2mm)
  set text(size: 9pt)
  richtext(d.hinweise)
}

#if d.zahlung.len() > 0 {
  v(2mm)
  richtext(d.zahlung)
}

#richtext(d.nachtext)

#if d.unterschrift {
  v(14mm)
  grid(
    columns: (70mm, 1fr, 70mm),
    [#line(length: 100%, stroke: 0.4pt) #text(size: 8pt, [Datum, Unterschrift Auftragnehmer])],
    [],
    [#line(length: 100%, stroke: 0.4pt) #text(size: 8pt, [Datum, Unterschrift Auftraggeber])],
  )
}
