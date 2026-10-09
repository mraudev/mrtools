import { invoke } from "@tauri-apps/api/core";
import type { Clipboard, Drive, Entry, FileOp, FolderSize } from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  listDrives: () => invoke<Drive[]>("list_drives"),
  listDir: (path: string) => invoke<Entry[]>("list_dir", { path }),
  stat: (path: string) => invoke<Entry>("stat", { path }),
  rename: (path: string, name: string) => invoke<string>("rename", { path, name }),
  create: (dir: string, name: string, folder: boolean) => invoke<string>("create", { dir, name, folder }),
  readText: (path: string) => invoke<string | null>("read_text", { path }),
  folderSize: (path: string) => invoke<FolderSize>("folder_size", { path }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  openWith: (path: string) => invoke<void>("open_with", { path }),
  properties: (path: string) => invoke<void>("properties", { path }),
  openTerminal: (dir: string) => invoke<void>("open_terminal", { dir }),
  fileOp: (op: FileOp, paths: string[], target?: string) => invoke<void>("file_op", { op, paths, target }),
  clipboardSet: (paths: string[], cut: boolean) => invoke<void>("clipboard_set", { paths, cut }),
  clipboardGet: () => invoke<Clipboard>("clipboard_get"),
  clipboardClear: () => invoke<void>("clipboard_clear"),
  dragOut: (paths: string[]) => invoke<void>("drag_out", { paths }),
  deletePermanently: (paths: string[]) => invoke<number>("delete_permanently", { paths }),
  cancelDelete: (id: number) => invoke<void>("cancel_delete", { id }),
};
