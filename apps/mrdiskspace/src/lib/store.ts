import { computed, reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { toastError } from "@mrtools/ui/lib/toast";
import type { Drive, Entry, NodeView, ScanDone, ScanProgress } from "./types";

export type Phase = "start" | "scanning" | "result";

/**
 * Nodes of the scan tree are addressed by a key: the child indices from the root joined by "/"
 * ("" is the root, "0/3" the fourth child of the first child). The Rust side resolves them.
 */
export const state = reactive({
  phase: "start" as Phase,
  drives: [] as Drive[],
  /** Path of the running (or last started) scan. */
  scanPath: "",
  scanId: 0,
  progress: null as ScanProgress | null,
  /** A finished scan exists – cancelling a new scan returns to it. */
  hasResult: false,
  elapsedMs: 0,
  /** Loaded folders by key. */
  nodes: new Map<string, NodeView>(),
  expanded: new Set<string>(),
  selected: "",
});

export const indexOf = (key: string) => (key === "" ? [] : key.split("/").map(Number));
export const childKey = (key: string, i: number) => (key === "" ? String(i) : `${key}/${i}`);
export const parentKey = (key: string) => (key.includes("/") ? key.slice(0, key.lastIndexOf("/")) : "");

export const root = computed(() => state.nodes.get(""));

export function entryOf(key: string): Entry | undefined {
  if (key === "") return root.value;
  const index = indexOf(key);
  return state.nodes.get(parentKey(key))?.children[index[index.length - 1]];
}

/** Full file system path of a loaded entry. */
export function pathOf(key: string): string {
  if (key === "") return root.value?.path ?? "";
  const parent = state.nodes.get(parentKey(key))?.path ?? "";
  const name = entryOf(key)?.name ?? "";
  return parent.endsWith("\\") ? parent + name : `${parent}\\${name}`;
}

export async function loadNode(key: string): Promise<NodeView> {
  const cached = state.nodes.get(key);
  if (cached) return cached;
  const node = await api.getNode(indexOf(key));
  state.nodes.set(key, node);
  return node;
}

// ---------------------------------------------------------------------------
// Tree rows

export interface Row {
  key: string;
  depth: number;
  entry: Entry;
  parentSize: number;
  expanded: boolean;
}

export interface MoreRow {
  key: string;
  depth: number;
  more: number;
}

/** Visible rows of the tree in display order. */
export const rows = computed(() => {
  const out: (Row | MoreRow)[] = [];
  const walk = (key: string, entry: Entry, depth: number, parentSize: number) => {
    const expanded = state.expanded.has(key);
    out.push({ key, depth, entry, parentSize, expanded });
    const node = expanded ? state.nodes.get(key) : undefined;
    if (!node) return;
    node.children.forEach((child, i) => walk(childKey(key, i), child, depth + 1, node.size));
    if (node.omitted) out.push({ key: `${key}/more`, depth: depth + 1, more: node.omitted });
  };
  if (root.value) walk("", root.value, 0, root.value.size);
  return out;
});

/** The folder shown in the detail panel: the selection, or its parent for a file. */
export const detailKey = computed(() => {
  const entry = entryOf(state.selected);
  return !entry || entry.isDir ? state.selected : parentKey(state.selected);
});

export async function select(key: string) {
  state.selected = key;
  if (entryOf(key)?.isDir) await loadNode(key).catch((e) => toastError("Ordner konnte nicht geladen werden", e));
}

export async function toggle(key: string) {
  if (state.expanded.has(key)) {
    state.expanded.delete(key);
    return;
  }
  try {
    await loadNode(key);
    state.expanded.add(key);
  } catch (e) {
    toastError("Ordner konnte nicht geladen werden", e);
  }
}

/** Expands all ancestors of `key` and selects it. */
export async function reveal(key: string) {
  const index = indexOf(key);
  try {
    for (let i = 0; i < index.length; i++) {
      const ancestor = index.slice(0, i).join("/");
      await loadNode(ancestor);
      state.expanded.add(ancestor);
    }
  } catch (e) {
    toastError("Eintrag konnte nicht angezeigt werden", e);
    return;
  }
  await select(key);
}

// ---------------------------------------------------------------------------
// Scanning

export async function loadDrives() {
  try {
    state.drives = await api.listDrives();
  } catch (e) {
    toastError("Laufwerke konnten nicht gelesen werden", e);
  }
}

/** Done events that arrived before `startScan` returned the id (tiny folders). */
const earlyDone = new Map<number, ScanDone>();

async function finishScan(done: ScanDone) {
  if (done.cancelled) {
    state.phase = state.hasResult ? "result" : "start";
    return;
  }
  try {
    const node = await api.getNode([]);
    state.nodes.clear();
    state.expanded.clear();
    state.nodes.set("", node);
    state.expanded.add("");
    state.selected = "";
    state.hasResult = true;
    state.elapsedMs = done.elapsedMs;
    state.phase = "result";
  } catch (e) {
    toastError("Ergebnis konnte nicht geladen werden", e);
    state.phase = state.hasResult ? "result" : "start";
  }
}

export async function startScan(path: string) {
  state.scanPath = path;
  state.progress = null;
  state.phase = "scanning";
  try {
    state.scanId = await api.startScan(path);
  } catch (e) {
    toastError("Scan konnte nicht gestartet werden", e);
    state.phase = state.hasResult ? "result" : "start";
    return;
  }
  const done = earlyDone.get(state.scanId);
  earlyDone.clear();
  if (done) finishScan(done);
}

export function cancelScan() {
  api.cancelScan().catch((e) => toastError("Abbrechen fehlgeschlagen", e));
}

export function rescan() {
  const path = root.value?.path ?? state.scanPath;
  if (path) startScan(path);
}

export function goHome() {
  if (state.phase === "scanning") cancelScan();
  state.phase = "start";
  loadDrives();
}

export async function initStore() {
  await listen<ScanProgress>("scan-progress", ({ payload }) => {
    if (payload.id === state.scanId) state.progress = payload;
  });
  await listen<ScanDone>("scan-done", ({ payload }) => {
    if (payload.id === state.scanId) finishScan(payload);
    else if (payload.id > state.scanId) earlyDone.set(payload.id, payload);
  });
  await loadDrives();
}
