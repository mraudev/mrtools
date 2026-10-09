import { invoke } from "@tauri-apps/api/core";
import type { Connection, Details, Module, Snapshot } from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  kill: (pid: number) => invoke<void>("kill", { pid }),
  killTree: (pid: number) => invoke<number>("kill_tree", { pid }),
  suspend: (pid: number, suspend: boolean) => invoke<void>("suspend", { pid, suspend }),
  setPriority: (pid: number, priorityClass: number) => invoke<void>("set_priority", { pid, class: priorityClass }),
  details: (pid: number) => invoke<Details>("details", { pid }),
  modules: (pid: number) => invoke<Module[]>("modules", { pid }),
  connections: (pid: number) => invoke<Connection[]>("connections", { pid }),
  icons: (paths: string[]) => invoke<Record<string, string | null>>("icons", { paths }),
  showProperties: (path: string) => invoke<void>("show_properties", { path }),
  isElevated: () => invoke<boolean>("is_elevated"),
  restartAsAdmin: () => invoke<void>("restart_as_admin"),
};
