/** Formatierung und Eingabe deutscher Zahlen und Daten. */

const betragFmt = new Intl.NumberFormat("de-DE", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const mengeFmt = new Intl.NumberFormat("de-DE", { minimumFractionDigits: 2, maximumFractionDigits: 3 });

export function betrag(n: number | null | undefined): string {
  return betragFmt.format(n ?? 0);
}

export function euro(n: number | null | undefined, waehrung = "EUR"): string {
  return `${betrag(n)} ${waehrung === "EUR" ? "€" : waehrung}`;
}

export function menge(n: number | null | undefined): string {
  return mengeFmt.format(n ?? 0);
}

/** Zahl ohne überflüssige Nachkommastellen (für Prozentsätze). */
export function zahl(n: number | null | undefined, max = 3): string {
  return new Intl.NumberFormat("de-DE", { maximumFractionDigits: max }).format(n ?? 0);
}

/** Rechnet „12+3*2“ (ohne eval, das die CSP verbietet). Gibt `null` bei ungültiger Eingabe. */
export function rechne(norm: string): number | null {
  let i = 0;
  const s = norm.replace(/\s+/g, "");
  const zahlen = (): number | null => {
    if (s[i] === "(") {
      i++;
      const w = summe();
      if (s[i] !== ")") return null;
      i++;
      return w;
    }
    if (s[i] === "-") {
      i++;
      const w = zahlen();
      return w === null ? null : -w;
    }
    const m = /^\d*\.?\d+/.exec(s.slice(i));
    if (!m) return null;
    i += m[0].length;
    return Number(m[0]);
  };
  const produkt = (): number | null => {
    let w = zahlen();
    while (w !== null && (s[i] === "*" || s[i] === "/")) {
      const op = s[i++];
      const r = zahlen();
      if (r === null) return null;
      w = op === "*" ? w * r : r === 0 ? null : w / r;
    }
    return w;
  };
  const summe = (): number | null => {
    let w = produkt();
    while (w !== null && (s[i] === "+" || s[i] === "-")) {
      const op = s[i++];
      const r = produkt();
      if (r === null) return null;
      w = op === "+" ? w + r : w - r;
    }
    return w;
  };
  const w = summe();
  return w !== null && i === s.length && Number.isFinite(w) ? Math.round(w * 1e6) / 1e6 : null;
}

/** „1.234,5“ oder „1234.5“ → 1234.5; leer/ungültig → 0. Einfache Rechnungen wie „12+3“ sind erlaubt. */
export function parseZahl(text: string): number {
  const t = text.trim();
  if (!t) return 0;
  // Tausenderpunkte nur entfernen, wenn ein Komma das Dezimaltrennzeichen ist
  const norm = t.includes(",") ? t.replace(/\.(?=\d{3}(\D|$))/g, "").replace(/,/g, ".") : t;
  return rechne(norm) ?? 0;
}

/** „2024-06-21“ → „21.06.2024“ */
export function datum(iso: string | null | undefined): string {
  if (!iso) return "";
  const [j, m, t] = iso.slice(0, 10).split("-");
  return t ? `${t}.${m}.${j}` : iso;
}

export function heute(): string {
  const d = new Date();
  const z = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${z(d.getMonth() + 1)}-${z(d.getDate())}`;
}

/** Erste Zeile eines mehrzeiligen Textes. */
export function kurztext(text: string): string {
  return text.split("\n")[0] ?? "";
}

/** HTML des Editors als Klartext (für Vorschauen). */
export function htmlText(html: string): string {
  return html
    .replace(/<br\s*\/?>/gi, "\n")
    .replace(/<\/p>/gi, "\n")
    .replace(/<[^>]+>/g, "")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .trim();
}

export const artTitel: Record<string, string> = {
  angebot: "Angebot",
  auftrag: "Auftragsbestätigung",
  lieferschein: "Lieferschein",
  abschlag: "Abschlagsrechnung",
  schluss: "Schlussrechnung",
  rechnung: "Rechnung",
  gutschrift: "Gutschrift",
  aufstellung: "Aufstellung",
};

export const statusTitel: Record<string, string> = {
  entwurf: "Entwurf",
  gedruckt: "Gedruckt",
  festgeschrieben: "Festgeschrieben",
  storniert: "Storniert",
};

/** Rechnungsarten (Forderungen im Rechnungsbuch). */
export function istRechnung(art: string): boolean {
  return ["abschlag", "schluss", "rechnung", "gutschrift"].includes(art);
}
