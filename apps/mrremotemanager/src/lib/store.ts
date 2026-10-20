import { computed, reactive, shallowRef, watch } from "vue";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { filterConnections, groupConnections, isActive, kindLabels, normalize, probeTarget, type Sort } from "./list";
import type { Active, Connection, ProbeStatus, Tools } from "./types";

/** How often the open connections are read again. */
const ACTIVE_INTERVAL = 3000;
/** How often the saved hosts are checked for reachability. */
const PROBE_INTERVAL = 30000;

function loadSort(): Sort {
  try {
    return localStorage.getItem("sort") === "recent" ? "recent" : "name";
  } catch {
    return "name";
  }
}

export const state = reactive({
  connections: [] as Connection[],
  query: "",
  sort: loadSort(),
  /** Id of the selected connection. */
  selected: "",
  loaded: false,
  /** Copy being edited in `EditDialog`; `isNew` when it is not in the list yet. */
  draft: null as Connection | null,
  isNew: false,
  /** Connection to delete, waiting for confirmation. */
  confirmDelete: null as Connection | null,
  /** Open session to end, waiting for confirmation. */
  confirmEnd: null as Active | null,
  /** Whether `ImportDialog` is open. */
  importing: false,
});

watch(
  () => state.sort,
  (sort) => {
    try {
      localStorage.setItem("sort", sort);
    } catch {
      // Not persisted – still applies for this session.
    }
  },
);

export const groups = computed(() => groupConnections(filterConnections(state.connections, state.query), state.sort));

/** All visible connections in display order (for keyboard navigation). */
export const visible = computed(() => groups.value.flatMap((g) => g.items));

export const selected = computed(() => state.connections.find((c) => c.id === state.selected));

/** Existing group names, for suggestions in the edit dialog. */
export const groupNames = computed(() =>
  [...new Set(state.connections.map((c) => c.group.trim()).filter(Boolean))].sort((a, b) => a.localeCompare(b, "de")),
);

/** Remote connections open right now. */
export const active = shallowRef<Active[]>([]);

/** Reachability and IP addresses by connection id. */
export const probes = reactive(new Map<string, ProbeStatus>());

/** Whether a Remotedesktop password is stored, by host. */
export const passwords = reactive(new Map<string, boolean>());

export const tools = shallowRef<Tools>({ vnc: false, putty: false, winscp: false });

/** The open sessions of a saved connection. */
export function sessionsOf(c: Connection): Active[] {
  const ips = probes.get(c.id)?.ips ?? [];
  return active.value.filter((a) => isActive(c, a, ips));
}

/** Ids of the saved connections that are open right now. */
export const connectedIds = computed(() => new Set(state.connections.filter((c) => sessionsOf(c).length).map((c) => c.id)));

/** Open connections that are not saved yet (matching the search). */
export const unsaved = computed(() => {
  const words = state.query.toLowerCase().split(/\s+/).filter(Boolean);
  return active.value
    .filter((a) => !state.connections.some((c) => isActive(c, a, probes.get(c.id)?.ips)))
    .filter((a) => {
      const text = [a.host, a.address, a.user, a.process, kindLabels[a.kind]].join(" ").toLowerCase();
      return words.every((w) => text.includes(w));
    });
});

let activeBusy = false;

export async function refreshActive() {
  if (activeBusy) return;
  activeBusy = true;
  try {
    active.value = await api.active();
  } catch {
    // Without the open connections the list still works.
  } finally {
    activeBusy = false;
  }
}

let probeBusy = false;

export async function refreshProbes() {
  if (probeBusy) return;
  probeBusy = true;
  try {
    const targets = state.connections.flatMap((c) => {
      const target = probeTarget(c);
      return target ? [{ id: c.id, ...target }] : [];
    });
    const result = await api.probe(targets);
    probes.clear();
    for (const [id, status] of Object.entries(result)) probes.set(id, status);
  } catch {
    // Without the check the list still works.
  } finally {
    probeBusy = false;
  }
}

export async function refreshPasswords() {
  const hosts = [...new Set(state.connections.filter((c) => c.kind === "rdp" && c.host.trim()).map((c) => c.host.trim()))];
  try {
    const result = await api.rdpPasswords(hosts);
    passwords.clear();
    for (const [host, stored] of Object.entries(result)) passwords.set(host, stored);
  } catch {
    // Unknown – the edit dialog then simply offers to store one.
  }
}

export async function persist() {
  try {
    await api.save(state.connections);
  } catch (e) {
    toastError("Verbindungen konnten nicht gespeichert werden", e);
  }
}

let probeTimer: ReturnType<typeof setTimeout> | undefined;

/** Checks again shortly after hosts were added or changed. */
export function probeSoon() {
  clearTimeout(probeTimer);
  probeTimer = setTimeout(refreshProbes, 500);
}

export async function initStore() {
  try {
    state.connections = normalize(await api.load());
  } catch (e) {
    toastError("Verbindungen konnten nicht geladen werden", e);
  }
  state.loaded = true;
  api.tools().then((t) => (tools.value = t), () => {});
  refreshPasswords();
  refreshProbes();
  await refreshActive();
  setInterval(() => {
    if (!document.hidden) refreshActive();
  }, ACTIVE_INTERVAL);
  setInterval(() => {
    if (!document.hidden) refreshProbes();
  }, PROBE_INTERVAL);
}
