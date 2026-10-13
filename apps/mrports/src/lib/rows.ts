import type { ProcessInfo, Socket } from "./types";

/** Well-known ports – a hint what is listening there. */
const SERVICES: Record<number, string> = {
  20: "FTP-Daten", 21: "FTP", 22: "SSH", 23: "Telnet", 25: "SMTP", 53: "DNS", 67: "DHCP", 68: "DHCP",
  80: "HTTP", 110: "POP3", 123: "NTP", 135: "Windows RPC", 137: "NetBIOS", 138: "NetBIOS", 139: "NetBIOS",
  143: "IMAP", 161: "SNMP", 389: "LDAP", 443: "HTTPS", 445: "Windows-Freigaben", 465: "SMTPS", 500: "IPsec",
  587: "SMTP", 636: "LDAPS", 993: "IMAPS", 995: "POP3S", 1433: "SQL Server", 1434: "SQL Server Browser",
  1521: "Oracle", 1883: "MQTT", 1900: "SSDP", 3000: "Dev-Server", 3306: "MySQL", 3389: "Remotedesktop",
  4200: "Angular", 5000: "Dev-Server", 5037: "Android ADB", 5040: "Windows CDP", 5050: "Dev-Server",
  5060: "SIP", 5061: "SIP (TLS)", 5173: "Vite", 5353: "mDNS", 5355: "LLMNR", 5432: "PostgreSQL",
  5672: "RabbitMQ", 5900: "VNC", 6379: "Redis", 6443: "Kubernetes", 7680: "Windows-Übermittlung",
  8000: "Dev-Server", 8080: "HTTP (alternativ)", 8443: "HTTPS (alternativ)", 8888: "Jupyter",
  9000: "Dev-Server", 9200: "Elasticsearch", 9229: "Node-Debugger", 11434: "Ollama", 27017: "MongoDB",
};

export function serviceName(port: number): string {
  return SERVICES[port] ?? "";
}

export type Mode = "listening" | "all";
export type SortKey = "port" | "process" | "pid" | "protocol";
export interface Sort {
  key: SortKey;
  desc: boolean;
}

export interface Row {
  key: string;
  protocol: "TCP" | "UDP";
  port: number;
  /** Local addresses of the merged sockets (IPv4 and IPv6 of the same port are one row). */
  addresses: string[];
  /** "Alle Netzwerke", "Nur dieser PC" or the addresses. */
  reach: string;
  remote: string;
  state: string;
  pid: number;
  process: ProcessInfo;
  service: string;
}

const WILDCARD = new Set(["0.0.0.0", "::"]);
const isLoopback = (a: string) => a.startsWith("127.") || a === "::1";

/** Who can connect: everybody, only this PC, or specific addresses. */
export function reach(addresses: string[]): string {
  if (addresses.some((a) => WILDCARD.has(a))) return "Alle Netzwerke";
  if (addresses.every(isLoopback)) return "Nur dieser PC";
  return addresses.join(", ");
}

/** A server socket: TCP that waits for connections, or any UDP socket. */
export const isListening = (s: Socket) => s.protocol === "UDP" || s.state === "Lauscht";

const remoteOf = (s: Socket) =>
  s.remoteAddress ? (s.remoteAddress.includes(":") ? `[${s.remoteAddress}]:${s.remotePort}` : `${s.remoteAddress}:${s.remotePort}`) : "";

/**
 * Table rows: in "listening" mode one row per protocol, port and process (IPv4 and IPv6 merged), in
 * "all" mode one row per socket. Filtered by `query` and sorted.
 */
export function buildRows(sockets: Socket[], processes: ProcessInfo[], mode: Mode, query: string, sort: Sort): Row[] {
  const byPid = new Map(processes.map((p) => [p.pid, p]));
  const groups = new Map<string, Row>();
  for (const s of sockets) {
    if (mode === "listening" && !isListening(s)) continue;
    const remote = remoteOf(s);
    const key = mode === "listening" ? `${s.protocol}:${s.localPort}:${s.pid}` : `${s.protocol}:${s.localAddress}:${s.localPort}:${remote}:${s.pid}`;
    const row = groups.get(key);
    if (row) {
      if (!row.addresses.includes(s.localAddress)) row.addresses.push(s.localAddress);
      continue;
    }
    groups.set(key, {
      key,
      protocol: s.protocol,
      port: s.localPort,
      addresses: [s.localAddress],
      reach: "",
      remote,
      state: s.state,
      pid: s.pid,
      process: byPid.get(s.pid) ?? { pid: s.pid, name: `Prozess ${s.pid}`, exe: "", cmd: "", cwd: "" },
      service: serviceName(s.localPort),
    });
  }
  const rows = [...groups.values()];
  for (const row of rows) row.reach = reach(row.addresses);
  return sortRows(rows.filter((r) => matches(r, query)), sort);
}

/** A number searches ports (and PIDs), anything else names, folders, command lines and addresses. */
export function matches(row: Row, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  if (/^\d+$/.test(q)) return String(row.port).startsWith(q) || row.remote.endsWith(`:${q}`) || String(row.pid) === q;
  const p = row.process;
  return [p.name, p.exe, p.cwd, p.cmd, row.service, row.remote, ...row.addresses].some((t) => t.toLowerCase().includes(q));
}

const collator = new Intl.Collator("de", { numeric: true, sensitivity: "base" });

export function sortRows(rows: Row[], sort: Sort): Row[] {
  const dir = sort.desc ? -1 : 1;
  const compare = (a: Row, b: Row): number => {
    switch (sort.key) {
      case "process":
        return collator.compare(a.process.name, b.process.name);
      case "pid":
        return a.pid - b.pid;
      case "protocol":
        return collator.compare(a.protocol, b.protocol);
      default:
        return a.port - b.port;
    }
  };
  return [...rows].sort((a, b) => compare(a, b) * dir || a.port - b.port || collator.compare(a.key, b.key));
}

/** For a searched port number: true if nothing uses it (in any state). */
export function portIsFree(sockets: Socket[], query: string): boolean {
  const q = query.trim();
  if (!/^\d{1,5}$/.test(q) || Number(q) > 65535) return false;
  return !sockets.some((s) => s.localPort === Number(q));
}
