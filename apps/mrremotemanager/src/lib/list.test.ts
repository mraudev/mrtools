import { describe, expect, it } from "vitest";
import {
  address,
  filterConnections,
  formatLastUsed,
  fromActive,
  fromCandidate,
  groupConnections,
  isActive,
  isSaved,
  mergeBackup,
  newConnection,
  normalize,
  probeTarget,
  validate,
} from "./list";
import type { Active, Candidate, Connection } from "./types";

const conn = (fields: Partial<Connection>): Connection => ({ ...newConnection(), ...fields });

describe("address", () => {
  it("hides the default port", () => {
    expect(address(conn({ kind: "rdp", host: "srv", port: 3389, user: "admin" }))).toBe("admin@srv");
    expect(address(conn({ kind: "ssh", host: "pi", port: 2222 }))).toBe("pi:2222");
    expect(address(conn({ kind: "web", host: "https://nas", user: "x" }))).toBe("https://nas");
  });
});

describe("filterConnections", () => {
  const list = [
    conn({ name: "Server Büro", host: "10.0.0.5", group: "Firma" }),
    conn({ name: "Raspberry", kind: "ssh", host: "pi.local", notes: "Wohnzimmer" }),
  ];

  it("matches all words in any field", () => {
    expect(filterConnections(list, "büro 10.0").map((c) => c.host)).toEqual(["10.0.0.5"]);
    expect(filterConnections(list, "wohnzimmer ssh").map((c) => c.host)).toEqual(["pi.local"]);
    expect(filterConnections(list, "  ")).toHaveLength(2);
    expect(filterConnections(list, "nichts")).toHaveLength(0);
  });
});

describe("groupConnections", () => {
  it("sorts groups and names, ungrouped last", () => {
    const groups = groupConnections([
      conn({ name: "b", group: "Zuhause" }),
      conn({ name: "x" }),
      conn({ name: "Server 10", group: "Arbeit" }),
      conn({ name: "Server 9", group: "Arbeit " }),
      conn({ name: "a", group: "Zuhause" }),
    ]);
    expect(groups.map((g) => g.name)).toEqual(["Arbeit", "Zuhause", ""]);
    expect(groups[0].items.map((c) => c.name)).toEqual(["Server 9", "Server 10"]);
    expect(groups[1].items.map((c) => c.name)).toEqual(["a", "b"]);
  });
});

describe("validate", () => {
  it("requires a sane host and port", () => {
    expect(validate(conn({ host: "" }))).toMatch(/Host/);
    expect(validate(conn({ host: "-oProxyCommand=x" }))).toMatch(/ungültige/);
    expect(validate(conn({ host: "srv", port: 70000 }))).toMatch(/Port/);
    expect(validate(conn({ host: "fe80::1%3" }))).toBeNull();
    expect(validate(conn({ kind: "web", host: "https://nas.local:5001/x?y=1" }))).toBeNull();
  });
});

describe("normalize", () => {
  it("fills missing fields and drops garbage", () => {
    const list = normalize([{ id: "1", name: "a", kind: "ssh", host: "h" }, null, { id: "2", kind: "ftp" }, { kind: "ssh" }]);
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({ id: "1", kind: "ssh", port: null, keyFile: "" });
    expect(normalize(null)).toEqual([]);
  });
});

describe("formatLastUsed", () => {
  it("is relative for recent times", () => {
    const now = 1_000_000_000_000;
    expect(formatLastUsed(null, now)).toBe("noch nie");
    expect(formatLastUsed(now - 5 * 60000, now)).toBe("vor 5 min");
    expect(formatLastUsed(now - 3 * 3600000, now)).toBe("vor 3 h");
  });
});

describe("active connections", () => {
  const open: Active = { kind: "rdp", host: "Server01", port: 3389, user: "me", keyFile: "", address: "10.0.0.5", pid: 1, process: "mstsc.exe" };

  it("matches saved connections by name or IP and port", () => {
    expect(isActive(conn({ kind: "rdp", host: "server01" }), open)).toBe(true);
    expect(isActive(conn({ kind: "rdp", host: "10.0.0.5", port: 3389 }), open)).toBe(true);
    expect(isActive(conn({ kind: "rdp", host: "server01", port: 3390 }), open)).toBe(false);
    expect(isActive(conn({ kind: "ssh", host: "server01" }), open)).toBe(false);
  });

  it("prefills a new connection", () => {
    expect(fromActive(open)).toMatchObject({ kind: "rdp", name: "Server01", host: "Server01", port: null, user: "me" });
    expect(fromActive({ ...open, kind: "ssh", port: 2222 }).port).toBe(2222);
  });
});

describe("favorites and sorting", () => {
  it("lists favorites first, recent sorting by last use", () => {
    const groups = groupConnections(
      [
        conn({ name: "a", group: "X", lastUsed: 1 }),
        conn({ name: "b", group: "X", lastUsed: 5 }),
        conn({ name: "c", group: "X", favorite: true }),
      ],
      "recent",
    );
    expect(groups.map((g) => [g.name, g.favorites ?? false])).toEqual([["Favoriten", true], ["X", false]]);
    expect(groups[1].items.map((c) => c.name)).toEqual(["b", "a"]);
  });
});

describe("other kinds", () => {
  it("shows and validates serial lines and Hyper-V machines", () => {
    expect(address(conn({ kind: "serial", host: "COM3" }))).toBe("COM3 · 9600 Baud");
    expect(address(conn({ kind: "hyperv", vmName: "Dev" }))).toBe("Dev auf diesem PC");
    expect(validate(conn({ kind: "serial", host: "COM3" }))).toBeNull();
    expect(validate(conn({ kind: "serial", host: "/dev/tty" }))).toMatch(/COM3/);
    expect(validate(conn({ kind: "hyperv", host: "", vmName: "Dev" }))).toBeNull();
    expect(validate(conn({ kind: "hyperv", vmName: 'a" b' }))).toMatch(/VM-Name/);
  });

  it("validates Remotedesktop and SSH extras", () => {
    expect(validate(conn({ host: "srv", width: 1600, height: null }))).toMatch(/Breite/);
    expect(validate(conn({ host: "srv", width: 1600, height: 900 }))).toBeNull();
    expect(validate(conn({ kind: "ssh", host: "h", forwards: "L 8080:localhost:80\nD 1080" }))).toBeNull();
    expect(validate(conn({ kind: "ssh", host: "h", forwards: "X 1:2" }))).toMatch(/Weiterleitung/);
    expect(validate(conn({ kind: "ssh", host: "h", jumpHost: "-oProxyCommand=x" }))).toMatch(/Jump/);
  });
});

describe("probeTarget", () => {
  it("knows the port to check", () => {
    expect(probeTarget(conn({ kind: "rdp", host: "srv" }))).toEqual({ host: "srv", port: 3389 });
    expect(probeTarget(conn({ kind: "vnc", host: "nas", port: 5901 }))).toEqual({ host: "nas", port: 5901 });
    expect(probeTarget(conn({ kind: "web", host: "nas.local:5001" }))).toEqual({ host: "nas.local", port: 5001 });
    expect(probeTarget(conn({ kind: "web", host: "http://router" }))).toEqual({ host: "router", port: 80 });
    expect(probeTarget(conn({ kind: "serial", host: "COM3" }))).toBeNull();
  });
});

describe("matching by resolved IP", () => {
  it("finds a session that only knows the IP", () => {
    const putty: Active = { kind: "ssh", host: "192.168.1.9", port: 22, user: "", keyFile: "", address: "192.168.1.9", pid: 2, process: "putty.exe" };
    const saved = conn({ kind: "ssh", host: "pi.local" });
    expect(isActive(saved, putty)).toBe(false);
    expect(isActive(saved, putty, ["192.168.1.9"])).toBe(true);
  });
});

describe("import", () => {
  const candidate: Candidate = { kind: "rdp", name: "srv", host: "SRV", port: null, user: "me", baud: null, source: "PuTTY", detail: "" };

  it("recognizes connections that are already saved", () => {
    expect(isSaved(candidate, [conn({ kind: "rdp", host: "srv", port: 3389 })])).toBe(true);
    expect(isSaved(candidate, [conn({ kind: "rdp", host: "srv", port: 3390 })])).toBe(false);
    expect(fromCandidate(candidate, "Büro")).toMatchObject({ kind: "rdp", host: "SRV", user: "me", group: "Büro" });
  });

  it("merges a backup by id", () => {
    const list = [conn({ id: "a", host: "x" })];
    expect(mergeBackup(list, [{ id: "a", kind: "rdp", host: "x" }, { id: "b", kind: "ssh", host: "y" }, { kind: "ssh" }])).toBe(1);
    expect(list.map((c) => c.id)).toEqual(["a", "b"]);
    expect(mergeBackup(list, "kaputt")).toBe(0);
  });
});
