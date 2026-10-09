import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { deletePermanently } from "./deletion";
import { startTransfer } from "./transfer";
import { isInside, parentPath } from "./paths";
import { navigate, refresh, selectedEntries, selectOnly, state } from "./store";
import type { Entry } from "./types";

async function attempt(title: string, action: () => Promise<unknown>): Promise<boolean> {
  try {
    await action();
    return true;
  } catch (e) {
    toastError(title, e);
    return false;
  }
}

/** Folders are opened in the app, files with their default program. */
export function openEntry(entry: Pick<Entry, "path" | "isDir">) {
  if (entry.isDir) navigate(entry.path);
  else attempt("Datei konnte nicht geöffnet werden", () => api.openPath(entry.path));
}

export function openSelection() {
  const entries = selectedEntries.value;
  // Several folders: only the first one can be shown.
  const folder = entries.find((e) => e.isDir);
  if (folder) return openEntry(folder);
  entries.slice(0, 20).forEach(openEntry);
}

export const openWith = (path: string) => attempt("„Öffnen mit“ fehlgeschlagen", () => api.openWith(path));
export const showProperties = (path: string) => attempt("Eigenschaften nicht verfügbar", () => api.properties(path));
export const openTerminal = (dir: string) => attempt("Terminal konnte nicht gestartet werden", () => api.openTerminal(dir));
export const showInExplorer = (path: string) => attempt("Explorer konnte nicht geöffnet werden", () => revealItemInDir(path));

export async function copyPaths(paths: string[]) {
  const text = paths.join("\r\n");
  if (await attempt("Kopieren fehlgeschlagen", () => navigator.clipboard.writeText(text))) {
    toast("success", paths.length === 1 ? "Pfad kopiert" : `${paths.length} Pfade kopiert`, paths.length === 1 ? text : undefined);
  }
}

// ---------------------------------------------------------------------------
// Clipboard (shared with Explorer)

export async function copyToClipboard(paths: string[], cut: boolean) {
  if (!paths.length) return;
  if (await attempt("Zwischenablage nicht verfügbar", () => api.clipboardSet(paths, cut))) {
    state.cut = new Set(cut ? paths : []);
  }
}

/** Pastes files from the clipboard into `target` (default: the current folder). */
export async function paste(target = state.path) {
  if (!target) return;
  let clip;
  try {
    clip = await api.clipboardGet();
  } catch (e) {
    return toastError("Zwischenablage nicht verfügbar", e);
  }
  if (!clip.paths.length) return toast("info", "Keine Dateien in der Zwischenablage");
  if (clip.paths.some((p) => isInside(target, p))) {
    return toast("error", "Ein Ordner kann nicht in sich selbst eingefügt werden");
  }
  // Moving into the same folder changes nothing.
  if (clip.cut && clip.paths.every((p) => parentPath(p).toLowerCase() === target.toLowerCase())) return;
  // After cut & paste the clipboard is emptied, like in Explorer.
  const afterMove = () => {
    state.cut.clear();
    api.clipboardClear().catch(() => {});
  };
  await startTransfer(clip.cut ? "move" : "copy", clip.paths, target, clip.cut ? afterMove : undefined);
}

// ---------------------------------------------------------------------------
// Changes

/** Into the recycle bin (via Windows), or permanently (fast, in Rust) – asks first unless `confirmed`. */
export async function deletePaths(paths: string[], permanently = false, confirmed = false) {
  if (!paths.length) return;
  if (permanently && !confirmed) {
    state.confirmDelete = paths;
    return;
  }
  state.confirmDelete = [];
  if (permanently) return deletePermanently(paths);
  await attempt("Löschen fehlgeschlagen", () => api.recycle(paths));
  await refresh();
}

export function deleteSelection(permanently = false) {
  deletePaths(
    selectedEntries.value.map((e) => e.path),
    permanently,
  );
}

export function startRename(path: string) {
  selectOnly(path);
  state.renaming = path;
}

export async function commitRename(path: string, name: string) {
  state.renaming = "";
  const old = path.slice(path.lastIndexOf("\\") + 1);
  if (!name.trim() || name === old) return;
  try {
    const renamed = await api.rename(path, name);
    await refresh({ select: [renamed] });
  } catch (e) {
    toastError("Umbenennen fehlgeschlagen", e);
  }
}

/** Creates "Neuer Ordner" / "Neues Textdokument.txt" and starts renaming it. */
export async function createNew(folder: boolean) {
  if (!state.path) return;
  state.filter = "";
  try {
    const path = await api.create(state.path, folder ? "Neuer Ordner" : "Neues Textdokument.txt", folder);
    await refresh({ select: [path] });
    state.renaming = path;
  } catch (e) {
    toastError(folder ? "Ordner konnte nicht angelegt werden" : "Datei konnte nicht angelegt werden", e);
  }
}
