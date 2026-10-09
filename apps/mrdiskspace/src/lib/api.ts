import { invoke } from "@tauri-apps/api/core";
import type { Drive, FileHit, NodeView, TypeStat } from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  listDrives: () => invoke<Drive[]>("list_drives"),
  startScan: (path: string) => invoke<number>("start_scan", { path }),
  cancelScan: () => invoke<void>("cancel_scan"),
  getNode: (index: number[]) => invoke<NodeView>("get_node", { index }),
  fileTypes: (index: number[], limit: number) => invoke<TypeStat[]>("file_types", { index, limit }),
  largestFiles: (index: number[], limit: number) => invoke<FileHit[]>("largest_files", { index, limit }),
};
