import { describe, expect, it } from "vitest";
import { buildRows, descendants, matches, type RowOptions } from "./rows";
import type { ProcInfo } from "./types";

function proc(pid: number, parent: number, name: string, extra: Partial<ProcInfo> = {}): ProcInfo {
  return {
    pid,
    parent,
    name,
    exe: "",
    cmd: "",
    user: "",
    cpu: 0,
    memory: 0,
    workingSet: 0,
    private: 0,
    diskRead: 0,
    diskWrite: 0,
    threads: 1,
    handles: 1,
    startTime: pid,
    cpuTime: 0,
    suspended: false,
    ...extra,
  };
}

const procs = [
  proc(1, 0, "wininit.exe"),
  proc(2, 1, "services.exe", { cpu: 5 }),
  proc(3, 2, "svchost.exe", { cpu: 1 }),
  proc(4, 2, "Code.exe", { cpu: 9, exe: "C:\\Tools\\Code.exe" }),
  proc(5, 4, "node.exe"),
  // Its parent has exited: shown as a root.
  proc(6, 9, "orphan.exe", { startTime: 0 }),
];
const opts: RowOptions = { query: "", sort: { key: "cpu", desc: true }, tree: false, collapsed: new Set() };

describe("buildRows", () => {
  it("sorts a flat list", () => {
    const rows = buildRows(procs, opts);
    expect(rows.map((r) => r.proc.pid)).toEqual([4, 2, 3, 5, 6, 1]);
  });

  it("builds the tree with sorted siblings", () => {
    const rows = buildRows(procs, { ...opts, tree: true });
    expect(rows.map((r) => [r.proc.pid, r.depth])).toEqual([
      [6, 0],
      [1, 0],
      [2, 1],
      [4, 2],
      [5, 3],
      [3, 2],
    ]);
  });

  it("hides children of collapsed processes", () => {
    const rows = buildRows(procs, { ...opts, tree: true, collapsed: new Set([2]) });
    expect(rows.map((r) => r.proc.pid)).toEqual([6, 1, 2]);
    expect(rows[2].hasChildren).toBe(true);
    expect(rows[2].expanded).toBe(false);
  });

  it("keeps the ancestors of matches in the tree", () => {
    const rows = buildRows(procs, { ...opts, tree: true, query: "node", collapsed: new Set([2]) });
    expect(rows.map((r) => r.proc.pid)).toEqual([1, 2, 4, 5]);
  });

  it("ignores a parent that started after the child", () => {
    const list = [proc(10, 0, "new.exe", { startTime: 100 }), proc(11, 10, "old.exe", { startTime: 50 })];
    const rows = buildRows(list, { ...opts, tree: true });
    expect(rows.every((r) => r.depth === 0)).toBe(true);
  });
});

describe("matches", () => {
  it("searches name, path and pid", () => {
    expect(matches(procs[3], "tools")).toBe(true);
    expect(matches(procs[3], "4")).toBe(true);
    expect(matches(procs[3], "chrome")).toBe(false);
  });
});

describe("descendants", () => {
  it("collects the whole subtree", () => {
    expect(descendants(2, procs).sort()).toEqual([2, 3, 4, 5]);
  });
});
