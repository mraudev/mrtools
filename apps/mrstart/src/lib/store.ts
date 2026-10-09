import { computed, reactive, watch } from "vue";
import { api } from "./api";
import { applyTheme, onSystemThemeChange } from "./theme";
import { toastError } from "@mrtools/ui/lib/toast";
import { normalizeConfig, orderTabs, watchedCategory } from "./config";
import {
  defaultSettings,
  emptyProject,
  folderName,
  PULLS,
  SETTINGS,
  WATCHED_ID_PREFIX,
  type Config,
  type Project,
  type View,
} from "./types";

export const store = reactive({
  config: { projects: [], settings: defaultSettings() } as Config,
  loaded: false,
  /** Set if the config file exists but could not be read; saving is disabled then. */
  loadError: "",
  /** Start view: the pull request dashboard. */
  view: PULLS as View,
  filter: "",
  /** Keyboard selection among the visible tiles (arrow keys in the filter). */
  selected: 0,
  watched: [] as Project[],
  /** Incremented to make tiles and lists reload their file system / git state. */
  refreshTick: 0,
});

// ---------------------------------------------------------------------------
// Loading & saving

export { watchedCategory };

let saveTimer: ReturnType<typeof setTimeout> | undefined;
function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    api.saveConfig(store.config).catch((e) => toastError("Einstellungen konnten nicht gespeichert werden", e));
  }, 400);
}

export async function loadWatched() {
  const { watchedFolders, defaultApps } = store.config.settings;
  if (watchedFolders.length === 0) {
    store.watched = [];
    return;
  }
  try {
    const entries = await api.watchedProjects(
      watchedFolders.map((f) => f.path),
      defaultApps,
    );
    const categoryOf = new Map(watchedFolders.map((f) => [f.path, watchedCategory(f)]));
    store.watched = entries.map((e) => ({
      ...emptyProject(categoryOf.get(e.root) ?? folderName(e.root)),
      id: `${WATCHED_ID_PREFIX}${e.path}`,
      name: e.name,
      path: e.path,
      apps: e.apps,
    }));
  } catch (e) {
    toastError("Überwachte Ordner konnten nicht gelesen werden", e);
  }
}

export async function initStore() {
  try {
    store.config = normalizeConfig(await api.loadConfig());
  } catch (e) {
    store.loadError = String(e);
  }

  const settings = () => store.config.settings;
  applyTheme(settings());
  watch(() => [settings().theme, settings().accent], () => applyTheme(settings()));
  onSystemThemeChange(() => applyTheme(settings()));

  if (!store.loadError) {
    watch(() => store.config, scheduleSave, { deep: true });
  }
  watch(
    () => [settings().watchedFolders, settings().defaultApps, store.refreshTick],
    loadWatched,
    { deep: true, immediate: true },
  );
  store.loaded = true;
}

/** Replaces the whole configuration (import); it is validated first. */
export function replaceConfig(raw: unknown) {
  store.config = normalizeConfig(raw);
}

export function refresh() {
  store.refreshTick++;
}

// ---------------------------------------------------------------------------
// Tabs & project lists

const collator = new Intl.Collator("de", { numeric: true, sensitivity: "base" });

const samePath = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

export const isPinned = (project: Project) =>
  store.config.settings.pinnedPaths.some((p) => samePath(p, project.path));

export function togglePin(project: Project) {
  const pinned = store.config.settings.pinnedPaths;
  const index = pinned.findIndex((p) => samePath(p, project.path));
  if (index >= 0) pinned.splice(index, 1);
  else pinned.push(project.path);
}

/** Favorites first, then highest version (as radstart did), then by name. */
function sortProjects(list: Project[]) {
  return [...list].sort(
    (a, b) =>
      Number(isPinned(b)) - Number(isPinned(a)) ||
      collator.compare(b.version, a.version) ||
      collator.compare(a.name, b.name),
  );
}

export const categoryView = (category: string): View => `cat:${category}`;

/** Watches `path` in its own tab (named after the folder); returns that tab. */
export function addWatchedFolder(path: string): View {
  const folders = store.config.settings.watchedFolders;
  let folder = folders.find((f) => f.path.toLowerCase() === path.toLowerCase());
  if (!folder) {
    folder = { path, category: folderName(path) };
    folders.push(folder);
  }
  return categoryView(watchedCategory(folder));
}

/** Tabs come from project categories and from the tabs of watched folders,
 *  in the user's order. */
export const categories = computed(() =>
  orderTabs(
    [
      ...new Set(
        [
          ...store.config.projects.map((p) => p.category.trim()),
          ...store.config.settings.watchedFolders.map(watchedCategory),
        ].filter(Boolean),
      ),
    ],
    store.config.settings.tabOrder,
  ),
);

/** Drag & drop in the tab bar: puts `category` before `before`. */
export function moveTab(category: string, before: string) {
  if (category === before) return;
  const order = categories.value.filter((c) => c !== category);
  order.splice(order.indexOf(before), 0, category);
  store.config.settings.tabOrder = order;
}

/** Renames a tab: the category of its projects and watched folders. */
export function renameTab(from: string, to: string) {
  const name = to.trim();
  if (!name || name === from) return;
  for (const p of store.config.projects) if (p.category.trim() === from) p.category = name;
  for (const f of store.config.settings.watchedFolders) if (watchedCategory(f) === from) f.category = name;
  store.config.settings.tabOrder = store.config.settings.tabOrder.map((c) => (c === from ? name : c));
  if (store.view === categoryView(from)) store.view = categoryView(name);
}

/** A watched subfolder that is also configured as a project is shown only once. */
function watchedWithoutConfigured() {
  const configured = new Set(store.config.projects.map((p) => p.path.toLowerCase()));
  return store.watched.filter((p) => !configured.has(p.path.toLowerCase()));
}

function projectsIn(category: string): Project[] {
  return sortProjects([
    ...store.config.projects.filter((p) => p.category.trim() === category),
    ...watchedWithoutConfigured().filter((p) => p.category === category),
  ]);
}

export interface Tab {
  id: View;
  label: string;
  count: number;
  /** The tab shows (also) the subfolders of a watched folder. */
  watched: boolean;
}

export const tabs = computed<Tab[]>(() => {
  const watchedTabs = new Set(store.config.settings.watchedFolders.map(watchedCategory));
  return categories.value.map((category) => ({
    id: categoryView(category),
    label: category,
    count: projectsIn(category).length,
    watched: watchedTabs.has(category),
  }));
});

/** The view actually shown: falls back to the first tab if the selected one vanished. */
export const activeView = computed<View>(() => {
  if (store.view === SETTINGS || store.view === PULLS) return store.view;
  if (tabs.value.some((t) => t.id === store.view)) return store.view;
  return tabs.value[0]?.id ?? "";
});

export function currentCategory(): string {
  const view = activeView.value;
  return view.startsWith("cat:") ? view.slice(4) : (categories.value[0] ?? "development");
}

/** Projects of the active tab, or matches across all tabs while filtering. */
export const visibleProjects = computed<Project[]>(() => {
  const query = store.filter.trim().toLowerCase();
  if (query) {
    return sortProjects(
      [...store.config.projects, ...watchedWithoutConfigured()].filter((p) =>
        [p.name, p.path, p.version, p.category].some((s) => s.toLowerCase().includes(query)),
      ),
    );
  }
  const view = activeView.value;
  return view.startsWith("cat:") ? projectsIn(view.slice(4)) : [];
});

// ---------------------------------------------------------------------------
// Project editing

export const editor = reactive({
  open: false,
  isNew: false,
  draft: emptyProject(),
});

export function openProjectEditor(project?: Project) {
  editor.isNew = !project;
  editor.draft = project ? JSON.parse(JSON.stringify(project)) : emptyProject(currentCategory());
  editor.open = true;
}

export function saveProject(project: Project) {
  const projects = store.config.projects;
  const index = projects.findIndex((p) => p.id === project.id);
  if (index >= 0) projects.splice(index, 1, project);
  else projects.push(project);
}

export function deleteProject(id: string) {
  store.config.projects = store.config.projects.filter((p) => p.id !== id);
}
