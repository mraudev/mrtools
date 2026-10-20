import { reactive } from "vue";

export type Seite = "kunden" | "rechnungsbuch" | "auswertung" | "artikel" | "einstellungen";

/** Was gerade angezeigt wird. Ein geöffneter Beleg oder Brief liegt über der Seite. */
export const nav = reactive({
  seite: "kunden" as Seite,
  kundeId: 0,
  projektId: 0,
  belegId: 0,
  briefId: 0,
  /** Wird erhöht, wenn sich Daten geändert haben, die Listen neu laden sollen. */
  version: 0,
  /** Ungespeicherte Änderungen im offenen Editor (fragt beim Verlassen nach). */
  geaendert: false,
});

export function neuLaden() {
  nav.version++;
}

export async function verlassenOk(): Promise<boolean> {
  if (!nav.geaendert) return true;
  const ok = await frage("Ungespeicherte Änderungen", "Die Änderungen wurden noch nicht gespeichert. Trotzdem schließen?", "Verwerfen");
  if (ok) nav.geaendert = false;
  return ok;
}

export async function oeffneBeleg(id: number) {
  if (nav.belegId === id) return;
  if (!(await verlassenOk())) return;
  nav.briefId = 0;
  nav.belegId = id;
}

export async function oeffneKunde(kundeId: number, projektId = 0) {
  if (!(await verlassenOk())) return;
  nav.belegId = 0;
  nav.briefId = 0;
  nav.seite = "kunden";
  nav.kundeId = kundeId;
  nav.projektId = projektId;
}

export async function zeigeSeite(seite: Seite) {
  if (!(await verlassenOk())) return;
  nav.belegId = 0;
  nav.briefId = 0;
  nav.seite = seite;
}

// ------------------------------------------------------------ Rückfragen

export const dialog = reactive({
  offen: false,
  titel: "",
  text: "",
  ok: "OK",
  gefaehrlich: false,
  antwort: null as ((ok: boolean) => void) | null,
});

/** Rückfrage mit „OK“/„Abbrechen“. */
export function frage(titel: string, text: string, ok = "OK", gefaehrlich = true): Promise<boolean> {
  return new Promise((resolve) => {
    Object.assign(dialog, { offen: true, titel, text, ok, gefaehrlich, antwort: resolve });
  });
}
