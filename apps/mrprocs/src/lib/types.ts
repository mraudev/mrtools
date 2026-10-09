export interface ProcInfo {
  pid: number;
  /** 0 if unknown or the parent has exited. */
  parent: number;
  name: string;
  exe: string;
  cmd: string;
  user: string;
  /** Share of the whole machine (0–100). */
  cpu: number;
  /** Private working set in bytes, like Task Manager. */
  memory: number;
  workingSet: number;
  /** Commit charge in bytes. */
  private: number;
  /** Bytes since the last snapshot. */
  diskRead: number;
  diskWrite: number;
  threads: number;
  handles: number;
  /** Seconds since 1970. */
  startTime: number;
  /** ms */
  cpuTime: number;
  suspended: boolean;
}

export interface Snapshot {
  processes: ProcInfo[];
  cpu: number;
  cpuCount: number;
  memoryUsed: number;
  memoryTotal: number;
  /** Seconds since the previous snapshot. */
  interval: number;
}

export interface Details {
  /** Windows priority class, 0 if unreadable. */
  priority: number;
  elevated: boolean | null;
}

export interface Module {
  name: string;
  path: string;
  size: number;
  base: string;
}

export interface Connection {
  protocol: string;
  local: string;
  remote: string;
  state: string;
}
