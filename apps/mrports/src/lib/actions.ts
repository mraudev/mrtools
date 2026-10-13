import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import type { Row } from "./rows";
import { refresh, state } from "./store";

export async function openInBrowser(port: number) {
  try {
    await openUrl(`http://localhost:${port}`);
  } catch (e) {
    toastError("Browser konnte nicht geöffnet werden", e);
  }
}

/** Asks first (see `ConfirmKill`). */
export function askKill(row: Row) {
  state.confirmKill = row;
}

export async function kill(row: Row) {
  state.confirmKill = null;
  try {
    await api.kill(row.pid);
    toast("success", `${row.process.name} beendet`, `Port ${row.port} ist jetzt frei.`);
  } catch (e) {
    const hint = state.elevated ? "" : " – als Administrator neu starten hilft bei Prozessen anderer Benutzer";
    toastError(`${row.process.name} konnte nicht beendet werden${hint}`, e);
  }
  // The port can take a moment to disappear from the table.
  setTimeout(refresh, 300);
}

export async function showFolder(row: Row) {
  const path = row.process.cwd || row.process.exe;
  if (!path) return toast("info", "Kein Ordner bekannt", "Als Administrator neu starten zeigt mehr.");
  try {
    await revealItemInDir(path);
  } catch (e) {
    toastError("Explorer konnte nicht geöffnet werden", e);
  }
}

export async function copy(text: string, what: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast("success", `${what} kopiert`, text.length < 120 ? text : undefined);
  } catch (e) {
    toastError("Kopieren fehlgeschlagen", e);
  }
}

export async function restartAsAdmin() {
  try {
    await api.restartAsAdmin();
  } catch (e) {
    toastError("Neustart als Administrator fehlgeschlagen", e);
  }
}
