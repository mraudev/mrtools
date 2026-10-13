import { invoke } from "@tauri-apps/api/core";
import type { Snapshot } from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  kill: (pid: number) => invoke<void>("kill", { pid }),
  icons: (paths: string[]) => invoke<Record<string, string | null>>("icons", { paths }),
  isElevated: () => invoke<boolean>("is_elevated"),
  restartAsAdmin: () => invoke<void>("restart_as_admin"),
};
