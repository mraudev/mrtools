import { computed, reactive, shallowRef, watch } from "vue";
import { api } from "./api";
import { buildRows, type Sort } from "./rows";
import { toastError } from "@mrtools/ui/lib/toast";
import type { ProcInfo } from "./types";

/** Samples kept for the charts. */
export const HISTORY = 60;
/** How long new and ended processes stay highlighted. */
const HIGHLIGHT_MS = 2500;

export const INTERVALS = [500, 1000, 2000, 5000];

interface Settings {
  interval: number;
  tree: boolean;
  sort: Sort;
}

function loadSettings(): Settings {
  const defaults: Settings = { interval: 1000, tree: false, sort: { key: "cpu", desc: true } };
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem("settings") ?? "{}") };
  } catch {
    return defaults;
  }
}

const settings = loadSettings();

export const state = reactive({
  ...settings,
  paused: false,
  query: "",
  selected: null as number | null,
  /** Collapsed pids in the tree view. */
  collapsed: new Set<number>(),
  loaded: false,
  elevated: false,
  cpu: 0,
  cpuCount: 1,
  memoryUsed: 0,
  memoryTotal: 0,
  /** Bumped on every snapshot so history-based views recompute. */
  tick: 0,
});

watch(
  () => ({ interval: state.interval, tree: state.tree, sort: state.sort }),
  (value) => {
    try {
      localStorage.setItem("settings", JSON.stringify(value));
    } catch {
      // Not persisted – still applies for this session.
    }
  },
  { deep: true },
);

/** Current processes; disk values are already bytes per second. */
export const procs = shallowRef<ProcInfo[]>([]);
export const byPid = computed(() => new Map(procs.value.map((p) => [p.pid, p])));

/** Exe path → PNG data URL (null: no icon). */
export const icons = reactive(new Map<string, string | null>());

/** Processes that exited recently, shown faded until `until`. */
const ended = new Map<number, { proc: ProcInfo; until: number }>();
/** When a process first appeared (only for processes started while mrprocs runs). */
const appeared = new Map<number, number>();

export const isNew = (pid: number) => (appeared.get(pid) ?? 0) > Date.now() - HIGHLIGHT_MS;
export const isEnded = (pid: number) => ended.has(pid);

/** Chart history per pid, keyed with the start time so a reused pid starts fresh. */
const history = new Map<number, { start: number; cpu: number[]; memory: number[] }>();
const systemHistory = { cpu: [] as number[], memory: [] as number[] };

/** Copies, so views recompute on every snapshot. */
export function historyOf(pid: number) {
  void state.tick;
  const h = history.get(pid);
  return h && { cpu: h.cpu.slice(), memory: h.memory.slice() };
}

export function systemHistoryCopy() {
  void state.tick;
  return { cpu: systemHistory.cpu.slice(), memory: systemHistory.memory.slice() };
}

function push(list: number[], value: number) {
  list.push(value);
  if (list.length > HISTORY) list.shift();
}

/** All rows including recently ended processes. */
export const rows = computed(() => {
  void state.tick;
  const all = ended.size ? [...procs.value, ...[...ended.values()].map((e) => e.proc)] : procs.value;
  return buildRows(all, { query: state.query, sort: state.sort, tree: state.tree, collapsed: state.collapsed });
});

export const selectedProc = computed(() => {
  void state.tick;
  if (state.selected === null) return undefined;
  return byPid.value.get(state.selected) ?? ended.get(state.selected)?.proc;
});

// ---------------------------------------------------------------------------
// Refresh loop

let timer: ReturnType<typeof setTimeout> | undefined;
let running = false;

function apply(list: ProcInfo[], interval: number) {
  const now = Date.now();
  const previous = byPid.value;
  const first = !state.loaded;

  for (const p of list) {
    // The first sample has no rates yet.
    p.diskRead = interval > 0 ? p.diskRead / interval : 0;
    p.diskWrite = interval > 0 ? p.diskWrite / interval : 0;
    const old = previous.get(p.pid);
    if (!first && (!old || old.startTime !== p.startTime)) appeared.set(p.pid, now);

    let h = history.get(p.pid);
    if (!h || h.start !== p.startTime) {
      h = { start: p.startTime, cpu: [], memory: [] };
      history.set(p.pid, h);
    }
    push(h.cpu, p.cpu);
    push(h.memory, p.memory);
  }

  const current = new Set(list.map((p) => p.pid));
  for (const [pid, p] of previous) {
    if (!current.has(pid)) ended.set(pid, { proc: { ...p, cpu: 0, diskRead: 0, diskWrite: 0 }, until: now + HIGHLIGHT_MS });
  }
  for (const [pid, e] of ended) {
    if (e.until < now || current.has(pid)) ended.delete(pid);
  }
  for (const [pid, t] of appeared) if (t < now - HIGHLIGHT_MS) appeared.delete(pid);
  for (const pid of history.keys()) if (!current.has(pid)) history.delete(pid);
  if (state.selected !== null && !current.has(state.selected) && !ended.has(state.selected)) state.selected = null;

  procs.value = list;
  state.loaded = true;
}

async function loadIcons(list: ProcInfo[]) {
  const missing = [...new Set(list.map((p) => p.exe).filter((exe) => exe && !icons.has(exe)))];
  if (!missing.length) return;
  for (const exe of missing) icons.set(exe, null);
  try {
    const result = await api.icons(missing);
    for (const [exe, url] of Object.entries(result)) icons.set(exe, url);
  } catch {
    // Icons are decoration – the generic one stays.
  }
}

export async function refresh() {
  if (running) return;
  running = true;
  clearTimeout(timer);
  try {
    const snap = await api.snapshot();
    apply(snap.processes, snap.interval);
    state.cpu = snap.cpu;
    state.cpuCount = snap.cpuCount;
    state.memoryUsed = snap.memoryUsed;
    state.memoryTotal = snap.memoryTotal;
    // The first snapshot has no CPU baseline yet.
    if (snap.interval > 0) {
      push(systemHistory.cpu, snap.cpu);
      push(systemHistory.memory, snap.memoryUsed);
    }
    state.tick++;
    loadIcons(snap.processes);
  } catch (e) {
    toastError("Prozesse konnten nicht gelesen werden", e);
  } finally {
    running = false;
    schedule();
  }
}

function schedule() {
  clearTimeout(timer);
  if (!state.paused) timer = setTimeout(refresh, state.interval);
}

watch(() => [state.paused, state.interval], schedule);

export async function initStore() {
  api.isElevated().then((v) => (state.elevated = v));
  await refresh();
  // The first snapshot has no CPU values yet: take the second one soon.
  clearTimeout(timer);
  timer = setTimeout(refresh, 300);
}

// ---------------------------------------------------------------------------
// Tree and selection

export function toggleCollapsed(pid: number) {
  if (state.collapsed.has(pid)) state.collapsed.delete(pid);
  else state.collapsed.add(pid);
}

export function setSort(key: Sort["key"]) {
  if (state.sort.key === key) state.sort = { key, desc: !state.sort.desc };
  // Numbers start with the largest, text alphabetically.
  else state.sort = { key, desc: !["name", "user"].includes(key) };
}

/** Selects `pid` and makes it visible (clears a search that hides it, expands its ancestors). */
export function reveal(pid: number) {
  const p = byPid.value.get(pid);
  if (!p) return;
  if (!rows.value.some((r) => r.proc.pid === pid)) {
    state.query = "";
    // Bounded: a pid reuse can form a cycle.
    let parent = p.parent;
    for (let i = 0; parent && i < 64; i++) {
      state.collapsed.delete(parent);
      parent = byPid.value.get(parent)?.parent ?? 0;
    }
  }
  state.selected = pid;
}
