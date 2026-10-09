import { computed, reactive, watch } from "vue";
import { audioDir, desktopDir, documentDir, downloadDir, homeDir, pictureDir, videoDir } from "@tauri-apps/api/path";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { insertFavorites, readFavorites, type Favorite } from "./favorites";
import { sortEntries } from "./fileTypes";
import { normalizePath, parentPath } from "./paths";
import type { Drive, Entry, SortKey } from "./types";

export interface Place {
  label: string;
  path: string;
  icon: "home" | "desktop" | "downloads" | "documents" | "pictures" | "music" | "videos";
}

interface Settings {
  view: "list" | "grid";
  sortKey: SortKey;
  ascending: boolean;
  showHidden: boolean;
  showDetails: boolean;
  favorites: Favorite[];
  lastPath: string;
}

const SETTINGS_KEY = "settings";
const defaults: Settings = {
  view: "list",
  sortKey: "name",
  ascending: true,
  showHidden: false,
  showDetails: true,
  favorites: [],
  lastPath: "",
};

function loadSettings(): Settings {
  try {
    const saved = JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? "{}");
    return { ...defaults, ...saved, favorites: readFavorites(saved.favorites) };
  } catch {
    return { ...defaults };
  }
}

export const settings = reactive<Settings>(loadSettings());

watch(settings, () => {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // Not persisted – still applies for this session.
  }
});

/** `path === ""` is "Dieser PC" (drives and known folders). */
export const state = reactive({
  path: "",
  entries: [] as Entry[],
  loading: false,
  history: [""] as string[],
  historyIndex: 0,
  /** Selected entries by path. */
  selected: new Set<string>(),
  /** Keyboard cursor and start of Shift ranges. */
  focused: "",
  filter: "",
  /** Path of the entry being renamed inline. */
  renaming: "",
  /** Entries cut to the clipboard – shown dimmed until pasted. */
  cut: new Set<string>(),
  /** Paths waiting for confirmation of permanent deletion. */
  confirmDelete: [] as string[],
  drives: [] as Drive[],
  places: [] as Place[],
});

/** Entries as shown: hidden ones filtered, search applied, sorted. */
export const visible = computed(() => {
  const query = state.filter.trim().toLowerCase();
  const list = state.entries.filter(
    (e) => (settings.showHidden || !(e.hidden || e.system)) && (!query || e.name.toLowerCase().includes(query)),
  );
  return sortEntries(list, settings.sortKey, settings.ascending);
});

export const selectedEntries = computed(() => visible.value.filter((e) => state.selected.has(e.path)));

export const canGoBack = computed(() => state.historyIndex > 0);
export const canGoForward = computed(() => state.historyIndex < state.history.length - 1);

// ---------------------------------------------------------------------------
// Navigation

/** Increases with every load – late answers for an older folder are dropped. */
let loadId = 0;

async function load(path: string): Promise<Entry[] | null> {
  const id = ++loadId;
  state.loading = true;
  try {
    const entries = path ? await api.listDir(path) : [];
    if (!path) loadDrives();
    return id === loadId ? entries : null;
  } finally {
    if (id === loadId) state.loading = false;
  }
}

/**
 * Opens a folder. `select` marks an entry afterwards (e.g. the folder we came from when going up).
 * Returns false if the folder could not be read.
 */
export async function navigate(target: string, options: { push?: boolean; select?: string } = {}): Promise<boolean> {
  const path = normalizePath(target);
  let entries: Entry[] | null;
  try {
    entries = await load(path);
  } catch (e) {
    toastError("Ordner kann nicht geöffnet werden", e);
    return false;
  }
  if (!entries) return false;

  if (options.push !== false && path !== state.path) {
    state.history.splice(state.historyIndex + 1);
    state.history.push(path);
    state.historyIndex = state.history.length - 1;
  }
  state.path = path;
  state.entries = entries;
  state.filter = "";
  state.renaming = "";
  settings.lastPath = path;
  const keep = options.select && entries.some((e) => e.path === options.select) ? options.select : "";
  state.selected = new Set(keep ? [keep] : []);
  state.focused = keep;
  return true;
}

export function goBack() {
  if (!canGoBack.value) return;
  const from = state.path;
  navigate(state.history[state.historyIndex - 1], { push: false, select: from }).then(
    (ok) => ok && state.historyIndex--,
  );
}

export function goForward() {
  if (!canGoForward.value) return;
  navigate(state.history[state.historyIndex + 1], { push: false }).then((ok) => ok && state.historyIndex++);
}

export function goUp() {
  if (!state.path) return;
  navigate(parentPath(state.path), { select: state.path });
}

/** Reloads the current folder and keeps the selection where possible. */
export async function refresh(options: { quiet?: boolean; select?: string[] } = {}) {
  let entries: Entry[] | null;
  try {
    entries = await load(state.path);
  } catch (e) {
    if (!options.quiet) toastError("Ordner konnte nicht neu geladen werden", e);
    return;
  }
  if (!entries) return;
  state.entries = entries;
  const exists = new Set(entries.map((e) => e.path));
  const wanted = options.select ?? [...state.selected];
  state.selected = new Set(wanted.filter((p) => exists.has(p)));
  if (options.select?.length) state.focused = options.select[0];
  else if (!exists.has(state.focused)) state.focused = "";
  for (const p of state.cut) if (!exists.has(p)) state.cut.delete(p);
}

// ---------------------------------------------------------------------------
// Selection

export function selectOnly(path: string) {
  state.selected = new Set(path ? [path] : []);
  state.focused = path;
}

export function toggleSelected(path: string) {
  if (state.selected.has(path)) state.selected.delete(path);
  else state.selected.add(path);
  state.focused = path;
}

/** Selects everything between the focused entry and `path` (Shift+click / Shift+arrows). */
export function selectRange(path: string, anchor = state.focused) {
  const list = visible.value;
  let a = list.findIndex((e) => e.path === anchor);
  if (a < 0) a = Math.max(0, list.findIndex((e) => e.path === state.focused));
  const b = list.findIndex((e) => e.path === path);
  if (b < 0) return;
  const [from, to] = a <= b ? [a, b] : [b, a];
  state.selected = new Set(list.slice(from, to + 1).map((e) => e.path));
}

export function selectAll() {
  state.selected = new Set(visible.value.map((e) => e.path));
}

// ---------------------------------------------------------------------------
// Sidebar

export async function loadDrives() {
  try {
    state.drives = await api.listDrives();
  } catch (e) {
    toastError("Laufwerke konnten nicht gelesen werden", e);
  }
}

async function loadPlaces() {
  const known: [Place["icon"], string, () => Promise<string>][] = [
    ["home", "Benutzerordner", homeDir],
    ["desktop", "Desktop", desktopDir],
    ["downloads", "Downloads", downloadDir],
    ["documents", "Dokumente", documentDir],
    ["pictures", "Bilder", pictureDir],
    ["music", "Musik", audioDir],
    ["videos", "Videos", videoDir],
  ];
  const results = await Promise.allSettled(known.map(([, , dir]) => dir()));
  state.places = known.flatMap(([icon, label], i) => {
    const result = results[i];
    return result.status === "fulfilled" ? [{ icon, label, path: normalizePath(result.value) }] : [];
  });
}

export function isFavorite(path: string) {
  return settings.favorites.some((f) => f.path.toLowerCase() === path.toLowerCase());
}

export function toggleFavorite(path: string, isDir = true) {
  if (isFavorite(path)) settings.favorites = settings.favorites.filter((f) => f.path.toLowerCase() !== path.toLowerCase());
  else settings.favorites.push({ path, isDir });
}

/** Pins files or folders at `index` of the favorites – or moves them there if they are pinned already. */
export async function pinFavorites(paths: string[], index: number) {
  const items = await Promise.all(
    paths.map(async (path): Promise<Favorite | null> => {
      const pinned = settings.favorites.find((f) => f.path.toLowerCase() === path.toLowerCase());
      if (pinned) return pinned;
      try {
        const entry = await api.stat(path);
        return { path: entry.path, isDir: entry.isDir };
      } catch (e) {
        toastError(`„${path}“ kann nicht angeheftet werden`, e);
        return null;
      }
    }),
  );
  settings.favorites = insertFavorites(
    settings.favorites,
    items.filter((f): f is Favorite => f !== null),
    index,
  );
}

export function sortBy(key: SortKey) {
  if (settings.sortKey === key) settings.ascending = !settings.ascending;
  else {
    settings.sortKey = key;
    // Newest and largest first feels natural for these columns.
    settings.ascending = key === "name" || key === "type";
  }
}

export async function initStore() {
  await loadPlaces();
  const ok = settings.lastPath && (await navigate(settings.lastPath, { push: false }));
  if (ok) state.history = [state.path];
  else await navigate("", { push: false });
  // Changes made outside the app show up when coming back to the window.
  window.addEventListener("focus", () => refresh({ quiet: true }));
}
