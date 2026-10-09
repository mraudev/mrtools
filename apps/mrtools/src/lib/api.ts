import { invoke } from "@tauri-apps/api/core";
import type { App, Release } from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  defaultRoot: () => invoke<string>("default_root"),
  scan: (root: string) => invoke<App[]>("scan", { root }),
  latestRelease: (repo: string, tagPrefix: string | null) =>
    invoke<Release | null>("latest_release", { repo, tagPrefix }),
  launch: (name: string) => invoke<void>("launch", { name }),
  openFolder: (path: string) => invoke<void>("open_folder", { path }),
};
