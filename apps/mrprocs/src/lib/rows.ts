import type { ProcInfo } from "./types";

export type SortKey = "name" | "pid" | "cpu" | "memory" | "disk" | "threads" | "user" | "startTime";

export interface Sort {
  key: SortKey;
  desc: boolean;
}

export interface ProcRow {
  proc: ProcInfo;
  depth: number;
  hasChildren: boolean;
  expanded: boolean;
}

export interface RowOptions {
  query: string;
  sort: Sort;
  tree: boolean;
  /** Collapsed pids in the tree (ignored while searching). */
  collapsed: Set<number>;
}

const collator = new Intl.Collator("de", { sensitivity: "base", numeric: true });

export function matches(p: ProcInfo, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  if (/^\d+$/.test(q)) return String(p.pid).startsWith(q) || p.name.toLowerCase().includes(q);
  return [p.name, p.exe, p.cmd, p.user].some((s) => s.toLowerCase().includes(q));
}

function value(p: ProcInfo, key: SortKey): number | string {
  switch (key) {
    case "disk":
      return p.diskRead + p.diskWrite;
    case "user":
      return p.user;
    case "name":
      return p.name;
    default:
      return p[key];
  }
}

export function compare(a: ProcInfo, b: ProcInfo, sort: Sort): number {
  const va = value(a, sort.key);
  const vb = value(b, sort.key);
  const result = typeof va === "string" ? collator.compare(va, vb as string) : va - (vb as number);
  if (result !== 0) return sort.desc ? -result : result;
  return collator.compare(a.name, b.name) || a.pid - b.pid;
}

/**
 * The parent of `p`, or 0. A parent that started after its child is a different process that
 * reused the pid of the real (exited) parent.
 */
export function parentOf(p: ProcInfo, byPid: Map<number, ProcInfo>): number {
  const parent = byPid.get(p.parent);
  return parent && parent.pid !== p.pid && parent.startTime <= p.startTime ? parent.pid : 0;
}

/** Visible rows in display order: a sorted flat list, or the process tree with sorted siblings. */
export function buildRows(procs: ProcInfo[], opts: RowOptions): ProcRow[] {
  const searching = opts.query.trim() !== "";
  if (!opts.tree) {
    return procs
      .filter((p) => matches(p, opts.query))
      .sort((a, b) => compare(a, b, opts.sort))
      .map((proc) => ({ proc, depth: 0, hasChildren: false, expanded: false }));
  }

  const byPid = new Map(procs.map((p) => [p.pid, p]));
  const children = new Map<number, ProcInfo[]>();
  const roots: ProcInfo[] = [];
  for (const p of procs) {
    const parent = parentOf(p, byPid);
    if (parent) {
      const list = children.get(parent);
      if (list) list.push(p);
      else children.set(parent, [p]);
    } else {
      roots.push(p);
    }
  }

  // While searching: the matches plus their ancestors, all expanded.
  let visible: Set<number> | undefined;
  if (searching) {
    visible = new Set();
    for (const p of procs) {
      if (!matches(p, opts.query)) continue;
      for (let pid = p.pid; pid && !visible.has(pid); pid = parentOf(byPid.get(pid)!, byPid)) visible.add(pid);
    }
  }

  const out: ProcRow[] = [];
  const walk = (list: ProcInfo[], depth: number) => {
    for (const proc of [...list].sort((a, b) => compare(a, b, opts.sort))) {
      if (visible && !visible.has(proc.pid)) continue;
      const kids = children.get(proc.pid) ?? [];
      const hasChildren = visible ? kids.some((k) => visible.has(k.pid)) : kids.length > 0;
      const expanded = hasChildren && (searching || !opts.collapsed.has(proc.pid));
      out.push({ proc, depth, hasChildren, expanded });
      if (expanded) walk(kids, depth + 1);
    }
  };
  walk(roots, 0);
  return out;
}

/** Pids of `pid` and all its descendants. */
export function descendants(pid: number, procs: ProcInfo[]): number[] {
  const byPid = new Map(procs.map((p) => [p.pid, p]));
  const out = [pid];
  for (let i = 0; i < out.length; i++) {
    for (const p of procs) if (parentOf(p, byPid) === out[i]) out.push(p.pid);
  }
  return out;
}
