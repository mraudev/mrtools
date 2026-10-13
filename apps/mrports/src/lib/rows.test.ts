import { describe, expect, it } from "vitest";
import { buildRows, portIsFree, reach, serviceName } from "./rows";
import type { ProcessInfo, Socket } from "./types";

const sock = (p: Partial<Socket>): Socket => ({
  protocol: "TCP",
  ipv6: false,
  localAddress: "0.0.0.0",
  localPort: 80,
  remoteAddress: "",
  remotePort: 0,
  state: "Lauscht",
  pid: 1,
  ...p,
});

const procs: ProcessInfo[] = [
  { pid: 1, name: "node.exe", exe: "C:\\node\\node.exe", cmd: "node vite", cwd: "C:\\dev\\mrports" },
  { pid: 2, name: "postgres.exe", exe: "", cmd: "", cwd: "" },
];

const sockets = [
  sock({ localPort: 1470, localAddress: "127.0.0.1" }),
  sock({ localPort: 1470, localAddress: "::1", ipv6: true }),
  sock({ localPort: 5432, pid: 2 }),
  sock({ localPort: 5432, pid: 2, localAddress: "::", ipv6: true }),
  sock({ localPort: 50123, localAddress: "192.168.1.5", remoteAddress: "140.82.112.3", remotePort: 443, state: "Verbunden" }),
  sock({ protocol: "UDP", localPort: 5353, state: "", pid: 3 }),
];
const byPort = { key: "port" as const, desc: false };

describe("buildRows", () => {
  it("merges IPv4 and IPv6 of a listening port and hides connections", () => {
    const rows = buildRows(sockets, procs, "listening", "", byPort);
    expect(rows.map((r) => [r.protocol, r.port, r.reach])).toEqual([
      ["TCP", 1470, "Nur dieser PC"],
      ["UDP", 5353, "Alle Netzwerke"],
      ["TCP", 5432, "Alle Netzwerke"],
    ]);
    expect(rows[0].addresses).toEqual(["127.0.0.1", "::1"]);
    expect(rows[2].service).toBe("PostgreSQL");
    expect(rows[0].process.cwd).toBe("C:\\dev\\mrports");
  });

  it("shows every connection in 'all' mode", () => {
    const rows = buildRows(sockets, procs, "all", "", byPort);
    expect(rows).toHaveLength(6);
    expect(rows.find((r) => r.port === 50123)?.remote).toBe("140.82.112.3:443");
  });

  it("searches ports by number and everything else by text", () => {
    const ports = (q: string, mode: "listening" | "all" = "listening") =>
      buildRows(sockets, procs, mode, q, byPort).map((r) => r.port);
    expect(ports("14")).toEqual([1470]);
    expect(ports("443", "all")).toEqual([50123]);
    expect(ports("postgres")).toEqual([5432]);
    expect(ports("mrports")).toEqual([1470]);
    expect(ports("vite")).toEqual([1470]);
  });

  it("sorts by process, descending", () => {
    const rows = buildRows(sockets, procs, "listening", "", { key: "process", desc: true });
    expect(rows.map((r) => r.process.name)).toEqual(["Prozess 3", "postgres.exe", "node.exe"]);
  });
});

describe("helpers", () => {
  it("names reach and services", () => {
    expect(reach(["127.0.0.1", "::1"])).toBe("Nur dieser PC");
    expect(reach(["192.168.1.5"])).toBe("192.168.1.5");
    expect(reach(["127.0.0.1", "::"])).toBe("Alle Netzwerke");
    expect(serviceName(5173)).toBe("Vite");
    expect(serviceName(12345)).toBe("");
  });

  it("knows free ports", () => {
    expect(portIsFree(sockets, "1470")).toBe(false);
    expect(portIsFree(sockets, "1471")).toBe(true);
    expect(portIsFree(sockets, "70000")).toBe(false);
    expect(portIsFree(sockets, "node")).toBe(false);
  });
});
