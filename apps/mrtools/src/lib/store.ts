import { computed, reactive } from "vue";
import { api } from "./api";
import { toastError } from "@mrtools/ui/lib/toast";
import type { App, ReleaseState } from "./types";

const ROOT_KEY = "root";

export const state = reactive({
  /** Folder whose subfolders are the apps. */
  root: "",
  apps: [] as App[],
  /** Latest GitHub release per app, see `releaseKey`. */
  releases: {} as Record<string, ReleaseState>,
  query: "",
  loaded: false,
  loading: false,
  error: "",
});

export const visibleApps = computed(() => {
  const q = state.query.trim().toLowerCase();
  if (!q) return state.apps;
  return state.apps.filter((a) =>
    [a.name, a.folder, a.description ?? "", a.github ?? ""].some((s) => s.toLowerCase().includes(q)),
  );
});

/** Apps of a monorepo share the repository but not their releases. */
export function releaseKey(app: App): string {
  return `${app.github}#${app.tagPrefix ?? ""}`;
}

async function loadReleases() {
  const apps = state.apps.filter((a) => a.github);
  const keys = [...new Set(apps.map(releaseKey))];
  for (const key of keys) state.releases[key] = undefined;
  await Promise.all(
    keys.map(async (key) => {
      const app = apps.find((a) => releaseKey(a) === key)!;
      try {
        state.releases[key] = await api.latestRelease(app.github!, app.tagPrefix);
      } catch (e) {
        state.releases[key] = { error: String(e) };
      }
    }),
  );
}

export async function refresh() {
  if (state.loading) return;
  state.loading = true;
  try {
    state.apps = await api.scan(state.root);
    state.error = "";
  } catch (e) {
    state.apps = [];
    state.error = String(e);
  } finally {
    state.loading = false;
    state.loaded = true;
  }
  loadReleases();
}

export async function setRoot(root: string) {
  state.root = root;
  try {
    localStorage.setItem(ROOT_KEY, root);
  } catch {
    // Not persisted – it still applies for this session.
  }
  await refresh();
}

export async function initStore() {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(ROOT_KEY);
  } catch {
    // Fall back to the default folder.
  }
  try {
    state.root = stored || (await api.defaultRoot());
  } catch (e) {
    toastError("Standardordner nicht ermittelbar", e);
  }
  await refresh();
}
