import { invoke } from "@tauri-apps/api/core";
import type { App, Release } from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  apps: () => invoke<App[]>("apps"),
  /** Latest release per app folder. */
  releases: () => invoke<Record<string, Release>>("releases"),
  install: (folder: string) => invoke<void>("install", { folder }),
  launch: (name: string) => invoke<void>("launch", { name }),
  uninstall: (name: string) => invoke<void>("uninstall", { name }),
  reveal: (name: string) => invoke<void>("reveal", { name }),
};
