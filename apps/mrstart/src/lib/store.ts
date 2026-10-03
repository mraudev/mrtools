import { computed, reactive, watch } from "vue";
import { api } from "./api";
import { applyTheme, onSystemThemeChange } from "./theme";
import { toastError } from "./toast";
import {
  defaultSettings,
  emptyProject,
  SETTINGS,
  WATCHED,
  type Config,
  type Project,
  type View,
} from "./types";

export const store = reactive({
  config: { projects: [], settings: defaultSettings() } as Config,
  loaded: false,
  /** Set if the config file exists but could not be read; saving is disabled then. */
  loadError: "",
  view: "" as View,
  filter: "",
  watched: [] as Project[],
  /** Incremented to make tiles and lists reload their file system / git state. */
  refreshTick: 0,
});

// ---------------------------------------------------------------------------
// Loading & saving

function normalizeProject(raw: any): Project {
  const commands = Array.isArray(raw?.commands) ? raw.commands : [];
  return {
    id: String(raw?.id ?? crypto.randomUUID()),
    name: String(raw?.name ?? ""),
    category: String(raw?.category ?? ""),
    path: String(raw?.path ?? ""),
    version: raw?.version == null ? "" : String(raw.version),
    apps: Array.isArray(raw?.apps) ? raw.apps.map(String) : [],
    commands: commands.map((c: any) => ({
      caption: String(c?.caption ?? ""),
      command: String(c?.command ?? ""),
    })),
  };
}

function normalizeConfig(raw: any): Config {
  return {
    projects: Array.isArray(raw?.projects) ? raw.projects.map(normalizeProject) : [],
    settings: { ...defaultSettings(), ...(raw?.settings ?? {}) },
  };
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;
function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    api.saveConfig(store.config).catch((e) => toastError("Einstellungen konnten nicht gespeichert werden", e));
  }, 400);
}

export async function loadWatched() {
  const { watchedDirectories, defaultApps } = store.config.settings;
  if (watchedDirectories.length === 0) {
    store.watched = [];
    return;
  }
  try {
    const entries = await api.watchedProjects(watchedDirectories, defaultApps);
    store.watched = entries.map((e) => ({
      ...emptyProject(WATCHED),
      id: `watched:${e.path}`,
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
    () => [settings().watchedDirectories, settings().defaultApps, store.refreshTick],
    loadWatched,
    { deep: true, immediate: true },
  );
  store.loaded = true;
}

export function refresh() {
  store.refreshTick++;
}

// ---------------------------------------------------------------------------
// Tabs & project lists

const collator = new Intl.Collator("de", { numeric: true, sensitivity: "base" });

/** Highest version first (as radstart did), then by name. */
function sortProjects(list: Project[]) {
  return [...list].sort(
    (a, b) => collator.compare(b.version, a.version) || collator.compare(a.name, b.name),
  );
}

export const categoryView = (category: string): View => `cat:${category}`;

export const categories = computed(() =>
  [...new Set(store.config.projects.map((p) => p.category.trim()).filter(Boolean))].sort(
    collator.compare,
  ),
);

export interface Tab {
  id: View;
  label: string;
  count: number;
}

export const tabs = computed<Tab[]>(() => {
  const result: Tab[] = categories.value.map((category) => ({
    id: categoryView(category),
    label: category,
    count: store.config.projects.filter((p) => p.category.trim() === category).length,
  }));
  if (store.config.settings.watchedDirectories.length > 0) {
    result.push({ id: WATCHED, label: "Überwacht", count: store.watched.length });
  }
  return result;
});

/** The view actually shown: falls back to the first tab if the selected one vanished. */
export const activeView = computed<View>(() => {
  if (store.view === SETTINGS) return SETTINGS;
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
    const configuredPaths = new Set(store.config.projects.map((p) => p.path.toLowerCase()));
    const all = [
      ...store.config.projects,
      ...store.watched.filter((p) => !configuredPaths.has(p.path.toLowerCase())),
    ];
    return sortProjects(
      all.filter((p) =>
        [p.name, p.path, p.version, p.category].some((s) => s.toLowerCase().includes(query)),
      ),
    );
  }
  const view = activeView.value;
  if (view === WATCHED) return store.watched;
  if (view.startsWith("cat:")) {
    const category = view.slice(4);
    return sortProjects(store.config.projects.filter((p) => p.category.trim() === category));
  }
  return [];
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
