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

export interface Measure {
  size: number;
  /** Rounded up to whole clusters (approximately what Explorer shows). */
  onDisk: number;
  files: number;
  dirs: number;
  /** Entries that could not be read. */
  errors: number;
}

export interface ItemInfo {
  /** ms since 1970, 0 if unknown. */
  accessed: number;
  /** Files only. */
  onDisk: number | null;
  /** Program that opens files of this type. */
  openWith: string | null;
  /** Target of a symbolic link or junction. */
  linkTarget: string | null;
}

export interface Hashes {
  sha256: string;
  sha1: string;
  md5: string;
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

export type TransferOp = "copy" | "move";

/** What happens to items whose name exists in the target already. */
export type ConflictChoice = "replace" | "skip" | "keepBoth";

export interface TransferProgress {
  id: number;
  files: number;
  bytes: number;
  /** 0 while still being counted. */
  totalFiles: number;
  totalBytes: number;
  failed: number;
  current: string;
}

export interface TransferDone {
  id: number;
  files: number;
  bytes: number;
  skipped: number;
  failed: number;
  failures: { path: string; message: string }[];
  cancelled: boolean;
  elapsedMs: number;
}

export type SortKey = "name" | "modified" | "type" | "size";

export interface SearchHit {
  name: string;
  path: string;
  isDir: boolean;
  /** Bytes, 0 for folders and deleted files. */
  size: number;
  /** ms since 1970, 0 if the file is gone. */
  modified: number;
}

export interface SearchResults {
  /** All matches – `hits` holds only the first ones. */
  total: number;
  hits: SearchHit[];
  elapsedMs: number;
}

export interface IndexStatus {
  /** Entries in the index, 0 if there is none yet. */
  entries: number;
  builtAt: number;
  building: boolean;
  /** Entries read so far while building. */
  scanned: number;
}
