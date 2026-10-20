// Briefpapier nach DIN 5008 (Form B): Briefkopf, Anschriftfeld, Infoblock, Fußzeile, Seitenzahlen.

#let grau = rgb("#555555")

#let zeilen(s) = {
  let teile = str(s).split("\n")
  teile.map(t => [#t]).join(linebreak())
}

// Absätze aus dem Editor: (liste, nr, teile: ((t, b, i, u), …))
#let richtext(absaetze) = {
  for a in absaetze {
    let inhalt = a.teile.map(t => {
      let c = zeilen(t.t)
      if t.b { c = strong(c) }
      if t.i { c = emph(c) }
      if t.u { c = underline(c) }
      c
    }).join()
    if inhalt == none { inhalt = [] }
    if a.liste == "ul" {
      block(above: 0.4em, below: 0.4em, grid(columns: (1.2em, 1fr), [•], inhalt))
    } else if a.liste == "ol" {
      block(above: 0.4em, below: 0.4em, grid(columns: (1.6em, 1fr), [#a.nr.], inhalt))
    } else if a.teile.len() == 0 {
      v(0.6em)
    } else {
      block(above: 0.5em, below: 0.5em, inhalt)
    }
  }
}

#let fusszeile(d) = {
  let f = d.firma
  if d.layout.fussbild != none {
    image(d.layout.fussbild, width: 100%)
  } else {
    set text(size: 7pt, fill: grau)
    let bank = f.banken.map(b => [#b.name #if b.iban != "" [· IBAN #b.iban] #if b.bic != "" [· BIC #b.bic]]).join(linebreak())
    let steuer = (
      if f.steuernummer != "" [Steuer-Nr. #f.steuernummer] else { none },
      if f.ustid != "" [USt-IdNr. #f.ustid] else { none },
      if f.registergericht != "" [#f.registergericht] else { none },
    ).filter(x => x != none).join(linebreak())
    line(length: 100%, stroke: 0.4pt + grau)
    v(-0.4em)
    if f.fusszeile != "" {
      align(center, zeilen(f.fusszeile))
    } else {
      grid(
        columns: (1fr, 1fr, 1fr),
        gutter: 4mm,
        [#f.name #if f.inhaber != "" [#linebreak()#f.inhaber] #linebreak() #f.strasse #linebreak() #f.plz #f.ort],
        [#if f.telefon != "" [Tel. #f.telefon #linebreak()] #if f.email != "" [#f.email #linebreak()] #if f.internet != "" [#f.internet]],
        [#bank #if steuer != none [#linebreak() #steuer]],
      )
    }
    if f.fusszeile != "" and (f.banken.len() > 0 or steuer != none) {
      align(center, [#bank #if steuer != none [ · #steuer]])
    }
  }
}

// Erste Seite: Briefkopf, Absenderzeile, Anschrift und Infoblock. `info` ist eine Liste von (Bezeichnung, Wert).
#let briefkopf(d, adresse, info) = {
  let f = d.firma
  let r = d.layout.rand
  // Briefkopf bis 45 mm vom oberen Blattrand
  block(height: 45mm - r.oben * 1mm - 2mm, width: 100%, {
    if d.layout.logo != none {
      place(top + right, image(d.layout.logo, height: 26mm, fit: "contain"))
    } else {
      place(top + right, align(right, {
        text(size: 18pt, weight: "bold", f.name)
        if f.inhaber != "" { linebreak(); text(size: 10pt, f.inhaber) }
        if f.zusatz != "" { linebreak(); text(size: 9pt, fill: grau, zeilen(f.zusatz)) }
      }))
    }
  })
  grid(
    columns: (85mm, 1fr),
    {
      block(height: 5mm, width: 85mm, {
        set text(size: 7pt, fill: grau)
        if f.absenderzeile != "" { underline(f.absenderzeile) } else {
          underline([#f.name · #f.strasse · #f.plz #f.ort])
        }
      })
      block(height: 40mm, width: 85mm, inset: (top: 2mm), {
        set par(leading: 0.55em)
        adresse.map(z => [#z]).join(linebreak())
      })
    },
    align(right, {
      if d.layout.seitenleiste != "" {
        block(text(size: 7.5pt, fill: grau, zeilen(d.layout.seitenleiste)))
        v(3mm)
      }
      set text(size: 9pt)
      grid(
        columns: (auto, auto),
        column-gutter: 4mm,
        row-gutter: 1.2mm,
        align: (right, left),
        ..info.map(x => (text(fill: grau, x.at(0)), x.at(1))).flatten(),
      )
    }),
  )
  v(6mm)
}

#let briefpapier(d, folgekopf: "", titel: "", body) = {
  let f = d.firma
  let r = d.layout.rand
  set document(title: titel, author: f.name)
  set text(font: ("Arial", "Liberation Sans", "Helvetica", "Libertinus Serif"), size: 10pt, lang: "de")
  set par(justify: false, leading: 0.6em)
  set page(
    paper: "a4",
    margin: (left: r.links * 1mm, right: r.rechts * 1mm, top: r.oben * 1mm, bottom: r.unten * 1mm + 22mm),
    header: context {
      if counter(page).get().first() > 1 {
        set text(size: 8pt, fill: grau)
        grid(columns: (1fr, auto), folgekopf, [Seite #counter(page).display()])
        v(-2mm)
        line(length: 100%, stroke: 0.4pt + grau)
      }
    },
    footer: context {
      set text(size: 8pt)
      let n = counter(page).final().first()
      if n > 1 { align(right, text(fill: grau, [Seite #counter(page).display() von #n])) }
      v(1mm)
      fusszeile(d)
    },
    footer-descent: 4mm,
  )
  body
}
