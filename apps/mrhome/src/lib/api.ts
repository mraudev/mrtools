import { invoke } from "@tauri-apps/api/core";
import type {
  DeviceLogin,
  FoundBridge,
  GoveeChange,
  GoveeDevice,
  Home,
  HomeState,
  HueChange,
  HueState,
  Quota,
  Room,
  Termination,
} from "./types";

/** Error text of the backend when there is no valid login. */
export const LOGGED_OUT = "LOGGED_OUT";

/** Typed wrappers around the Rust commands in `src-tauri/src` (tado, Hue, Govee). */
export const api = {
  authStatus: () => invoke<{ loggedIn: boolean; quota: Quota | null }>("auth_status"),
  loginStart: () => invoke<DeviceLogin>("login_start"),
  loginFinish: (login: DeviceLogin) => invoke<void>("login_finish", { login }),
  logout: () => invoke<void>("logout"),
  homes: () => invoke<Home[]>("homes"),
  rooms: (home: number) => invoke<Room[]>("rooms", { home }),
  setRoom: (home: number, room: number, temperature: number | null, termination: Termination) =>
    invoke<void>("set_room", { home, room, temperature, termination }),
  resumeRoom: (home: number, room: number) => invoke<void>("resume_room", { home, room }),
  endOpenWindow: (home: number, room: number) => invoke<void>("end_open_window", { home, room }),
  quickAction: (home: number, action: "boost" | "allOff" | "resumeSchedule") => invoke<void>("quick_action", { home, action }),
  homeState: (home: number) => invoke<HomeState>("home_state", { home }),
  setPresence: (home: number, presence: "HOME" | "AWAY" | null) => invoke<void>("set_presence", { home, presence }),

  hueStatus: () => invoke<FoundBridge | null>("hue_status"),
  hueDiscover: () => invoke<FoundBridge[]>("hue_discover"),
  huePair: (ip: string) => invoke<void>("hue_pair", { ip }),
  hueUnpair: () => invoke<void>("hue_unpair"),
  hueState: () => invoke<HueState>("hue_state"),
  hueSetLight: (id: string, change: HueChange) => invoke<void>("hue_set_light", { id, change }),
  hueSetGroup: (id: string, change: HueChange) => invoke<void>("hue_set_group", { id, change }),
  hueScene: (id: string) => invoke<void>("hue_scene", { id }),

  goveeScan: () => invoke<{ devices: GoveeDevice[]; error: string | null }>("govee_scan"),
  goveeSet: (ip: string, change: GoveeChange) => invoke<void>("govee_set", { ip, change }),
};
