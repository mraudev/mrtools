import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { address, displayName, fromActive, newConnection } from "./list";
import { persist, selected, state } from "./store";
import type { Active, Connection } from "./types";

export async function connect(c: Connection) {
  try {
    await api.connect(c);
    c.lastUsed = Date.now();
    persist();
  } catch (e) {
    toastError(`${displayName(c)} konnte nicht geöffnet werden`, e);
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
  state.draft = { ...c, id: crypto.randomUUID(), name: `${displayName(c)} (Kopie)`, lastUsed: null };
  state.isNew = true;
}

/** Stores the draft of `EditDialog`. */
export function saveDraft() {
  const draft = state.draft;
  if (!draft) return;
  const clean = { ...draft, name: draft.name.trim(), host: draft.host.trim(), user: draft.user.trim(), group: draft.group.trim() };
  const index = state.connections.findIndex((c) => c.id === clean.id);
  if (index >= 0) state.connections[index] = clean;
  else state.connections.push(clean);
  state.selected = clean.id;
  state.draft = null;
  persist();
}

/** Asks first (see `ConfirmDelete`). */
export function askDelete(c: Connection) {
  state.confirmDelete = c;
}

export function remove(c: Connection) {
  state.confirmDelete = null;
  state.connections = state.connections.filter((x) => x.id !== c.id);
  if (state.selected === c.id) state.selected = "";
  persist();
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
