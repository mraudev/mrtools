export interface Drive {
  path: string;
  label: string;
  fileSystem: string;
  total: number;
  free: number;
  removable: boolean;
}

export interface Entry {
  name: string;
  size: number;
  /** ms since 1970, 0 if unknown. For folders the newest change of their contents. */
  modified: number;
  files: number;
  dirs: number;
  /** Entries that could not be read (usually access denied). */
  errors: number;
  isDir: boolean;
  hasChildren: boolean;
}

export interface NodeView extends Entry {
  path: string;
  /** Sorted by size, largest first. */
  children: Entry[];
  /** Small children not included in `children`. */
  omitted: number;
}

export interface TypeStat {
  ext: string;
  size: number;
  count: number;
}

export interface FileHit {
  name: string;
  path: string;
  size: number;
  modified: number;
  index: number[];
}

export interface ScanProgress {
  id: number;
  files: number;
  dirs: number;
  bytes: number;
  errors: number;
  current: string;
  elapsedMs: number;
}

export interface ScanDone {
  id: number;
  cancelled: boolean;
  elapsedMs: number;
}
