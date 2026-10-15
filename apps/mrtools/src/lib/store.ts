import { computed, reactive, watch } from "vue";
import { api } from "./api";
import { groupApps, sortApps } from "./apps";
import type { App, Release } from "./types";

export const state = reactive({
  apps: [] as App[],
  /** Latest release per app folder. */
  releases: {} as Record<string, Release>,
  releasesLoading: false,
  releasesError: "",
  /** Folders whose installer is running. */
  installing: new Set<string>(),
  /** Folders whose uninstaller is running. */
  uninstalling: new Set<string>(),
  query: "",
  loaded: false,
  /** Show the apps grouped by category (trial). */
  grouped: loadGrouped(),
});

function loadGrouped(): boolean {
  try {
    return localStorage.getItem("grouped") === "1";
  } catch {
    return false;
  }
}

watch(
  () => state.grouped,
  (grouped) => {
    try {
      localStorage.setItem("grouped", grouped ? "1" : "0");
    } catch {
      // Not persisted – still applies for this session.
    }
  },
);

/** Filtered by the search; apps with an update first, then installed ones, then the rest. */
export const visibleApps = computed(() => {
  const q = state.query.trim().toLowerCase();
  const apps = q ? state.apps.filter((a) => [a.name, a.description ?? ""].some((s) => s.toLowerCase().includes(q))) : state.apps;
  return sortApps(apps, state.releases);
});

export const categories = computed(() => groupApps(visibleApps.value, state.releases));

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
