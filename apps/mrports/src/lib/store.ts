import { computed, reactive, shallowRef, watch } from "vue";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { buildRows, type Mode, type Row, type Sort } from "./rows";
import type { Snapshot } from "./types";

/** How often the list is read again. */
const INTERVAL = 2000;

interface Settings {
  mode: Mode;
  sort: Sort;
}

function loadSettings(): Settings {
  const defaults: Settings = { mode: "listening", sort: { key: "port", desc: false } };
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem("settings") ?? "{}") };
  } catch {
    return defaults;
  }
}

export const state = reactive({
  ...loadSettings(),
  query: "",
  paused: false,
  /** Key of the selected row. */
  selected: "",
  loaded: false,
  elevated: false,
  /** Row whose process should be ended, waiting for confirmation. */
  confirmKill: null as Row | null,
});

watch(
  () => ({ mode: state.mode, sort: state.sort }),
  (value) => {
    try {
      localStorage.setItem("settings", JSON.stringify(value));
    } catch {
      // Not persisted – still applies for this session.
    }
  },
  { deep: true },
);

export const snapshot = shallowRef<Snapshot>({ sockets: [], processes: [] });

export const rows = computed(() =>
  buildRows(snapshot.value.sockets, snapshot.value.processes, state.mode, state.query, state.sort),
);

export const selectedRow = computed(() => rows.value.find((r) => r.key === state.selected));

/** Exe path → PNG data URL (null: no icon). */
export const icons = reactive(new Map<string, string | null>());

async function loadIcons() {
  const missing = [...new Set(snapshot.value.processes.map((p) => p.exe))].filter((p) => p && !icons.has(p));
  if (!missing.length) return;
  missing.forEach((p) => icons.set(p, null));
  try {
    const loaded = await api.icons(missing);
    for (const [path, icon] of Object.entries(loaded)) icons.set(path, icon);
  } catch {
    // Without icons the list still works.
  }
}

let busy = false;

export async function refresh() {
  if (busy) return;
  busy = true;
  try {
    snapshot.value = await api.snapshot();
    state.loaded = true;
    loadIcons();
  } catch (e) {
    toastError("Ports konnten nicht gelesen werden", e);
  } finally {
    busy = false;
  }
}

export function sortBy(key: Sort["key"]) {
  state.sort = state.sort.key === key ? { key, desc: !state.sort.desc } : { key, desc: false };
}

export async function initStore() {
  state.elevated = await api.isElevated().catch(() => false);
  await refresh();
  setInterval(() => {
    if (!state.paused && !document.hidden) refresh();
  }, INTERVAL);
}
