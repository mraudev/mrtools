import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { address, displayName, fromActive, mergeBackup, newConnection } from "./list";
import { passwords, persist, probeSoon, refreshActive, refreshPasswords, selected, state } from "./store";
import type { Active, Connection } from "./types";

export async function connect(c: Connection) {
  try {
    await api.connect(c);
    c.lastUsed = Date.now();
    persist();
    // The new session shows up in the list without waiting for the next refresh.
    setTimeout(refreshActive, 1500);
  } catch (e) {
    toastError(`${displayName(c)} konnte nicht geöffnet werden`, e);
  }
}

export async function openSftp(c: Connection) {
  try {
    await api.openSftp(c);
  } catch (e) {
    toastError("WinSCP konnte nicht geöffnet werden", e);
  }
}

/** Opens the edit dialog for a new connection, in the group of the selected one. */
export function create() {
  state.draft = newConnection(selected.value?.kind, selected.value?.group);
  state.isNew = true;
}

/** Opens the edit dialog prefilled from an open connection. */
export function saveActive(a: Active) {
  state.draft = fromActive(a);
  state.isNew = true;
}

export function edit(c: Connection) {
  state.draft = { ...c };
  state.isNew = false;
}

export function duplicate(c: Connection) {
  state.draft = { ...c, id: crypto.randomUUID(), name: `${displayName(c)} (Kopie)`, favorite: false, lastUsed: null };
  state.isNew = true;
}

export function toggleFavorite(c: Connection) {
  c.favorite = !c.favorite;
  persist();
}

/**
 * Stores the draft of `EditDialog`. A new Remotedesktop password goes to the credential store
 * first; if that fails, the dialog stays open.
 */
export async function saveDraft(password: string): Promise<boolean> {
  const draft = state.draft;
  if (!draft) return false;
  const clean = {
    ...draft,
    name: draft.name.trim(),
    host: draft.host.trim(),
    user: draft.user.trim(),
    group: draft.group.trim(),
    gateway: draft.gateway.trim(),
    jumpHost: draft.jumpHost.trim(),
    vmName: draft.vmName.trim(),
  };
  if (clean.kind === "rdp" && password) {
    try {
      await api.setRdpPassword(clean.host, clean.user, password);
    } catch (e) {
      toastError("Passwort konnte nicht gespeichert werden", e);
      return false;
    }
  }
  const index = state.connections.findIndex((c) => c.id === clean.id);
  if (index >= 0) state.connections[index] = clean;
  else state.connections.push(clean);
  state.selected = clean.id;
  state.draft = null;
  persist();
  refreshPasswords();
  probeSoon();
  return true;
}

export async function removePassword(host: string) {
  try {
    await api.deleteRdpPassword(host);
    passwords.set(host, false);
    toast("success", "Passwort entfernt", host);
  } catch (e) {
    toastError("Passwort konnte nicht entfernt werden", e);
  }
}

/** Asks first (see `ConfirmDelete`). */
export function askDelete(c: Connection) {
  state.confirmDelete = c;
}

export async function remove(c: Connection, deletePassword: boolean) {
  state.confirmDelete = null;
  state.connections = state.connections.filter((x) => x.id !== c.id);
  if (state.selected === c.id) state.selected = "";
  persist();
  if (deletePassword) await removePassword(c.host);
  toast("success", `${displayName(c)} gelöscht`);
}

export async function copyAddress(c: Connection) {
  try {
    await navigator.clipboard.writeText(address(c));
    toast("success", "Adresse kopiert", address(c));
  } catch (e) {
    toastError("Kopieren fehlgeschlagen", e);
  }
}

export async function focusSession(a: Active) {
  try {
    await api.focusSession(a.pid);
  } catch (e) {
    toastError("Fenster konnte nicht geholt werden", e);
  }
}

/** Asks first (see `ConfirmEnd`). */
export function askEndSession(a: Active) {
  state.confirmEnd = a;
}

export async function endSession(a: Active) {
  state.confirmEnd = null;
  try {
    await api.endSession(a.pid);
    toast("success", "Sitzung beendet", `${a.process} · ${a.host}`);
  } catch (e) {
    toastError("Sitzung konnte nicht beendet werden", e);
  }
  setTimeout(refreshActive, 300);
}

const jsonFilter = [{ name: "JSON", extensions: ["json"] }];

export async function exportBackup() {
  try {
    const path = await saveDialog({ defaultPath: "mrremotemanager.json", filters: jsonFilter });
    if (!path) return;
    await api.exportTo(path, state.connections);
    toast("success", `${state.connections.length} Verbindungen exportiert`, path);
  } catch (e) {
    toastError("Export fehlgeschlagen", e);
  }
}

export async function importBackup() {
  try {
    const path = await openDialog({ multiple: false, filters: jsonFilter });
    if (typeof path !== "string") return;
    const added = mergeBackup(state.connections, await api.importFrom(path));
    if (added) {
      persist();
      refreshPasswords();
      probeSoon();
    }
    toast("success", added ? `${added} Verbindungen importiert` : "Nichts Neues – alle Verbindungen sind schon da");
  } catch (e) {
    toastError("Import fehlgeschlagen", e);
  }
}

/** Adds the connections chosen in `ImportDialog`. */
export function addImported(list: Connection[]) {
  state.connections.push(...list);
  state.importing = false;
  persist();
  refreshPasswords();
  probeSoon();
  toast("success", `${list.length} ${list.length === 1 ? "Verbindung" : "Verbindungen"} importiert`);
}
