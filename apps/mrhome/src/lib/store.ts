import { reactive, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api, LOGGED_OUT } from "./api";
import { applyManual, budgetAllowsAuto, clampTarget, isDue } from "./logic";
import type { DeviceLogin, Home, HomeState, Quota, Room, Termination } from "./types";
import { ui } from "./ui";

/** Changes by hand are sent after this pause – several clicks on +/− become one call. */
const SEND_DELAY = 1500;

interface Settings {
  home: Home | null;
  /** Minutes between automatic refreshes while the window is visible; 0: never. */
  autoRefresh: number;
  termination: Termination;
}

function loadSettings(): Settings {
  const defaults: Settings = { home: null, autoRefresh: 30, termination: { kind: "nextBlock" } };
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem("settings") ?? "{}") };
  } catch {
    return defaults;
  }
}

export const settings = reactive<Settings>(loadSettings());

watch(settings, () => {
  try {
    localStorage.setItem("settings", JSON.stringify(settings));
  } catch {
    // Not persisted – still applies for this session.
  }
});

export const state = reactive({
  phase: "checking" as "checking" | "login" | "ready",
  login: null as DeviceLogin | null,
  loggingIn: false,
  homes: [] as Home[],
  rooms: [] as Room[],
  homeState: null as HomeState | null,
  quota: null as Quota | null,
  loading: false,
  /** ms since 1970 of the last successful refresh. */
  lastUpdate: 0,
  /** Rooms with a change waiting to be sent. */
  pending: new Set<number>(),
});

/** Handles the backend's errors: a lost login leads back to the login view. */
function fail(title: string, e: unknown) {
  if (String(e) === LOGGED_OUT) {
    state.phase = "login";
    state.rooms = [];
    return;
  }
  toastError(title, e);
}

// ---------------------------------------------------------------------------
// Login

export async function startLogin() {
  state.loggingIn = true;
  try {
    const login = await api.loginStart();
    state.login = login;
    await openUrl(login.verificationUri).catch(() => {});
    await api.loginFinish(login);
    state.login = null;
    toast("success", "Mit tado verbunden");
    await load();
  } catch (e) {
    state.login = null;
    toastError("Anmeldung fehlgeschlagen", e);
  } finally {
    state.loggingIn = false;
  }
}

export async function logout() {
  await api.logout().catch((e) => toastError("Abmelden fehlgeschlagen", e));
  settings.home = null;
  state.rooms = [];
  state.homeState = null;
  state.phase = "login";
}

// ---------------------------------------------------------------------------
// Loading – every call costs one of the day's requests.

/** Rooms only (one call). */
export async function refresh() {
  const home = settings.home;
  if (!home || state.loading) return;
  state.loading = true;
  try {
    const rooms = await api.rooms(home.id);
    // A change waiting to be sent stays visible instead of jumping back.
    state.rooms = rooms.map((r) => (state.pending.has(r.id) ? (state.rooms.find((o) => o.id === r.id) ?? r) : r));
    state.lastUpdate = Date.now();
  } catch (e) {
    fail("Räume konnten nicht geladen werden", e);
  } finally {
    state.loading = false;
  }
}

/** Home (first time only), rooms and presence. */
export async function load() {
  try {
    if (!settings.home) {
      state.homes = await api.homes();
      settings.home = state.homes[0] ?? null;
      if (!settings.home) return toast("error", "Kein Zuhause im tado-Konto gefunden");
    }
    state.phase = "ready";
    await refresh();
    state.homeState = await api.homeState(settings.home.id);
  } catch (e) {
    fail("tado konnte nicht geladen werden", e);
  }
}

export async function chooseHome(home: Home) {
  settings.home = home;
  state.rooms = [];
  await load();
}

// ---------------------------------------------------------------------------
// Changes

const timers = new Map<number, ReturnType<typeof setTimeout>>();

function replaceRoom(room: Room) {
  state.rooms = state.rooms.map((r) => (r.id === room.id ? room : r));
}

/** Sets a room's temperature (`null`: off). Sent after a short pause. */
export function setTarget(room: Room, temperature: number | null) {
  const value = temperature === null ? null : clampTarget(temperature);
  replaceRoom(applyManual(room, value, settings.termination, Date.now()));
  state.pending.add(room.id);
  clearTimeout(timers.get(room.id));
  timers.set(
    room.id,
    setTimeout(async () => {
      timers.delete(room.id);
      const home = settings.home;
      if (!home) return;
      try {
        await api.setRoom(home.id, room.id, value, settings.termination);
      } catch (e) {
        fail(`${room.name} konnte nicht geändert werden`, e);
      } finally {
        state.pending.delete(room.id);
      }
    }, SEND_DELAY),
  );
}

async function run(title: string, action: () => Promise<void>, reload = true) {
  try {
    await action();
    if (reload) await refresh();
  } catch (e) {
    fail(title, e);
  }
}

export function resumeRoom(room: Room) {
  run(`${room.name} konnte nicht zurückgesetzt werden`, () => api.resumeRoom(settings.home!.id, room.id));
}

export function endOpenWindow(room: Room) {
  run("Fenster-offen-Modus konnte nicht beendet werden", () => api.endOpenWindow(settings.home!.id, room.id));
}

export function quickAction(action: "boost" | "allOff" | "resumeSchedule") {
  const labels = { boost: "Boost (30 min) überall", allOff: "Heizung überall aus", resumeSchedule: "Überall wieder Zeitplan" };
  run("Aktion fehlgeschlagen", async () => {
    await api.quickAction(settings.home!.id, action);
    toast("success", labels[action]);
  });
}

/** `HOME`/`AWAY` by hand, `null`: automatic (geofencing). */
export function setPresence(presence: "HOME" | "AWAY" | null) {
  const previous = state.homeState;
  state.homeState = presence ? { presence, locked: true } : { presence: previous?.presence ?? "HOME", locked: false };
  run(
    "Zuhause/Abwesend konnte nicht geändert werden",
    async () => {
      try {
        await api.setPresence(settings.home!.id, presence);
      } catch (e) {
        state.homeState = previous;
        throw e;
      }
    },
    false,
  );
}

// ---------------------------------------------------------------------------
// Start

export async function initStore() {
  await listen<Quota>("quota", ({ payload }) => (state.quota = payload));
  try {
    const status = await api.authStatus();
    state.quota = status.quota;
    if (!status.loggedIn) {
      state.phase = "login";
      return;
    }
  } catch (e) {
    toastError("Anmeldestatus konnte nicht gelesen werden", e);
    state.phase = "login";
    return;
  }
  await load();

  // Automatic refreshing: only while visible and while the day's budget allows it.
  const autoRefresh = () => {
    if (state.phase !== "ready" || ui.tab !== "heating" || document.hidden || !budgetAllowsAuto(state.quota)) return;
    if (isDue(state.lastUpdate, Date.now(), settings.autoRefresh)) refresh();
  };
  setInterval(autoRefresh, 30_000);
  document.addEventListener("visibilitychange", autoRefresh);
  watch(() => ui.tab, autoRefresh);
}
