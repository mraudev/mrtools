export interface Drive {
  path: string;
  label: string;
  fileSystem: string;
  total: number;
  free: number;
  kind: "fixed" | "removable" | "network" | "cdrom";
}

export interface Entry {
  name: string;
  path: string;
  isDir: boolean;
  /** Bytes, 0 for folders. */
  size: number;
  /** ms since 1970, 0 if unknown. */
  modified: number;
  created: number;
  hidden: boolean;
  system: boolean;
  readonly: boolean;
  /** Symbolic link or junction. */
  link: boolean;
}

export interface FolderSize {
  size: number;
  files: number;
  dirs: number;
  errors: number;
}

export interface Clipboard {
  paths: string[];
  cut: boolean;
}

export interface DeleteProgress {
  id: number;
  deleted: number;
  failed: number;
  current: string;
}

export interface DeleteDone {
  id: number;
  deleted: number;
  failed: number;
  /** The first few failures. */
  failures: { path: string; message: string }[];
  cancelled: boolean;
  elapsedMs: number;
}

export type FileOp = "copy" | "move" | "recycle";

export type SortKey = "name" | "modified" | "type" | "size";
