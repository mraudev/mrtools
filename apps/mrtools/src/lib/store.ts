import { computed, reactive } from "vue";
import { api } from "./api";
import type { App, Release } from "./types";

export const state = reactive({
  apps: [] as App[],
  /** Latest release per app folder. */
  releases: {} as Record<string, Release>,
  releasesLoading: false,
  releasesError: "",
  /** Folders whose installer is running. */
  installing: new Set<string>(),
  query: "",
  loaded: false,
});

export const visibleApps = computed(() => {
  const q = state.query.trim().toLowerCase();
  if (!q) return state.apps;
  return state.apps.filter((a) => [a.name, a.description ?? ""].some((s) => s.toLowerCase().includes(q)));
});

/** Re-reads which apps are installed (e.g. after an installer ran). */
export async function loadInstalled() {
  state.apps = await api.apps();
  state.loaded = true;
}

async function loadReleases() {
  state.releasesLoading = true;
  try {
    state.releases = await api.releases();
    state.releasesError = "";
  } catch (e) {
    state.releasesError = String(e);
  } finally {
    state.releasesLoading = false;
  }
}

export async function refresh() {
  await Promise.all([loadInstalled(), loadReleases()]);
}
