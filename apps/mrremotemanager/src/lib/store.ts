import { computed, reactive, shallowRef } from "vue";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { filterConnections, groupConnections, isActive, kindLabels, normalize } from "./list";
import type { Active, Connection } from "./types";

/** How often the open connections are read again. */
const INTERVAL = 3000;

export const state = reactive({
  connections: [] as Connection[],
  query: "",
  /** Id of the selected connection. */
  selected: "",
  loaded: false,
  /** Copy being edited in `EditDialog`; `isNew` when it is not in the list yet. */
  draft: null as Connection | null,
  isNew: false,
  /** Connection to delete, waiting for confirmation. */
  confirmDelete: null as Connection | null,
});

export const groups = computed(() => groupConnections(filterConnections(state.connections, state.query)));

/** All visible connections in display order (for keyboard navigation). */
export const visible = computed(() => groups.value.flatMap((g) => g.items));

export const selected = computed(() => state.connections.find((c) => c.id === state.selected));

/** Remote connections open right now. */
export const active = shallowRef<Active[]>([]);

/** Ids of the saved connections that are open right now. */
export const connectedIds = computed(
  () => new Set(state.connections.filter((c) => active.value.some((a) => isActive(c, a))).map((c) => c.id)),
);

/** Open connections that are not saved yet (matching the search). */
export const unsaved = computed(() => {
  const words = state.query.toLowerCase().split(/\s+/).filter(Boolean);
  return active.value
    .filter((a) => !state.connections.some((c) => isActive(c, a)))
    .filter((a) => {
      const text = [a.host, a.address, a.user, a.process, kindLabels[a.kind]].join(" ").toLowerCase();
      return words.every((w) => text.includes(w));
    });
});

let busy = false;

async function refreshActive() {
  if (busy) return;
  busy = true;
  try {
    active.value = await api.active();
  } catch {
    // Without the open connections the list still works.
  } finally {
    busy = false;
  }
}

/** Existing group names, for suggestions in the edit dialog. */
export const groupNames = computed(() =>
  [...new Set(state.connections.map((c) => c.group.trim()).filter(Boolean))].sort((a, b) => a.localeCompare(b, "de")),
);

export async function persist() {
  try {
    await api.save(state.connections);
  } catch (e) {
    toastError("Verbindungen konnten nicht gespeichert werden", e);
  }
}

export async function initStore() {
  try {
    state.connections = normalize(await api.load());
  } catch (e) {
    toastError("Verbindungen konnten nicht geladen werden", e);
  }
  state.loaded = true;
  await refreshActive();
  setInterval(() => {
    if (!document.hidden) refreshActive();
  }, INTERVAL);
}
