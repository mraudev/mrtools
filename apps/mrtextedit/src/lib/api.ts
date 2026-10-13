import { invoke } from "@tauri-apps/api/core";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  readText: (path: string) => invoke<string>("read_text", { path }),
  writeText: (path: string, content: string) => invoke<void>("write_text", { path, content }),
  startupFile: () => invoke<string | null>("startup_file"),
  loadConfig: () => invoke<string | null>("load_config"),
  saveConfig: (json: string) => invoke<void>("save_config", { json }),
};
