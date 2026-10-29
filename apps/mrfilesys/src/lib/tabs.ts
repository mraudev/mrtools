import { reactive, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { baseName, normalizePath } from "./paths";
import { navigate, settings, state } from "./store";

/** A folder tab. The active tab's folder, history and selection live in `state` while it is shown. */
export interface Tab {
  id: number;
  path: string;
  history: string[];
  historyIndex: number;
  selected: string[];
  focused: string;
}

let nextId = 1;
const blank = (path: string): Tab => ({ id: nextId++, path, history: [path], historyIndex: 0, selected: [], focused: "" });

export const tabs = reactive({ list: [] as Tab[], active: 0 });

const activeIndex = () => tabs.list.findIndex((t) => t.id === tabs.active);

export const tabPath = (tab: Tab) => (tab.id === tabs.active ? state.path : tab.path);
export const tabTitle = (tab: Tab) => (tabPath(tab) ? baseName(tabPath(tab)) : "Dieser PC");

/** Shows the folder of `tab` – a folder that is gone falls back to "Dieser PC". */
async function show(tab: Tab) {
  state.history = [...tab.history];
  state.historyIndex = tab.historyIndex;
  if (!(await navigate(tab.path, { push: false }))) await navigate("", { push: false });
  if (tabs.active !== tab.id) return;
  const exists = new Set(state.entries.map((e) => e.path));
  state.selected = new Set(tab.selected.filter((p) => exists.has(p)));
  state.focused = exists.has(tab.focused) ? tab.focused : "";
}

export async function activateTab(id: number) {
  const tab = tabs.list.find((t) => t.id === id);
  const current = tabs.list[activeIndex()];
  if (!tab || tab === current) return;
  if (current) {
    Object.assign(current, {
      path: state.path,
      history: [...state.history],
      historyIndex: state.historyIndex,
      selected: [...state.selected],
      focused: state.focused,
    });
  }
  tabs.active = id;
  await show(tab);
}

/** Opens `path` in a new tab next to the active one – in the background or switching to it. */
export function openTab(path: string, background = false) {
  const tab = blank(normalizePath(path));
  tabs.list.splice(activeIndex() + 1, 0, tab);
  if (!background) activateTab(tab.id);
}

/** Middle click on a folder (an element with `data-drop` inside the handling one) opens it in a background tab. */
export function openOnMiddleClick(event: MouseEvent) {
  const folder = (event.target as HTMLElement).closest<HTMLElement>("[data-drop]");
  if (event.button === 1 && folder?.dataset.drop && folder !== event.currentTarget) openTab(folder.dataset.drop, true);
}

/** Switches to a tab showing `path`, or opens one (a folder opened from Explorer). */
export function revealTab(path: string) {
  const wanted = normalizePath(path).toLowerCase();
  const tab = tabs.list.find((t) => tabPath(t).toLowerCase() === wanted);
  if (tab) activateTab(tab.id);
  else openTab(path);
}

/** The last tab stays open. */
export function closeTab(id: number) {
  if (tabs.list.length < 2) return;
  const index = tabs.list.findIndex((t) => t.id === id);
  if (index < 0) return;
  if (id === tabs.active) activateTab(tabs.list[index + 1]?.id ?? tabs.list[index - 1].id);
  tabs.list.splice(index, 1);
}

export function cycleTab(step: number) {
  const n = tabs.list.length;
  activateTab(tabs.list[(activeIndex() + step + n) % n].id);
}

/** Ctrl+1 … Ctrl+8, Ctrl+9 is the last tab. */
export function selectTab(number: number) {
  const tab = number === 9 ? tabs.list.at(-1) : tabs.list[number - 1];
  if (tab) activateTab(tab.id);
}

export async function initTabs() {
  const paths = settings.tabs.length ? settings.tabs : [settings.lastPath];
  tabs.list = paths.map(blank);
  const first = tabs.list[Math.min(Math.max(settings.activeTab, 0), tabs.list.length - 1)];
  tabs.active = first.id;
  await show(first);

  watch(
    () => [tabs.list.map(tabPath), activeIndex()] as const,
    ([paths, index]) => {
      settings.tabs = paths;
      settings.activeTab = index;
    },
  );

  // Started with a folder ("mrfilesys hier öffnen"), or a second start handed one over.
  const launched = await invoke<string | null>("launch_path").catch(() => null);
  if (launched) revealTab(launched);
  await listen<string>("open-path", ({ payload }) => revealTab(payload));
}
