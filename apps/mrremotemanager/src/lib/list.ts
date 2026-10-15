import type { Active, Candidate, Connection, Kind } from "./types";

export const kinds: Kind[] = ["rdp", "ssh", "web", "vnc", "telnet", "serial", "hyperv"];

export const kindLabels: Record<Kind, string> = {
  rdp: "Remotedesktop",
  ssh: "SSH",
  web: "Web",
  vnc: "VNC",
  telnet: "Telnet",
  serial: "Seriell",
  hyperv: "Hyper-V",
};

export const defaultPorts: Record<Kind, number | null> = {
  rdp: 3389,
  ssh: 22,
  web: null,
  vnc: 5900,
  telnet: 23,
  serial: null,
  hyperv: null,
};

export const DEFAULT_BAUD = 9600;

export function newConnection(kind: Kind = "rdp", group = ""): Connection {
  return {
    id: crypto.randomUUID(),
    name: "",
    kind,
    host: "",
    port: null,
    user: "",
    group,
    notes: "",
    favorite: false,
    lastUsed: null,
    fullscreen: false,
    multimon: false,
    width: null,
    height: null,
    clipboard: true,
    drives: false,
    gateway: "",
    admin: false,
    keyFile: "",
    jumpHost: "",
    forwards: "",
    baud: null,
    vmName: "",
  };
}

/** Fills in fields missing in older or hand-edited files. */
export function normalize(list: unknown): Connection[] {
  if (!Array.isArray(list)) return [];
  return list
    .filter((c): c is Partial<Connection> => !!c && typeof c === "object" && typeof c.id === "string" && !!c.id)
    .map((c) => ({ ...newConnection(), ...c }) as Connection)
    .filter((c) => kinds.includes(c.kind));
}

/** How the connection is reached, as shown in the list. */
export function address(c: Connection): string {
  switch (c.kind) {
    case "web":
      return c.host;
    case "serial":
      return `${c.host} · ${c.baud ?? DEFAULT_BAUD} Baud`;
    case "hyperv":
      return `${c.vmName} auf ${c.host.trim() || "diesem PC"}`;
    default: {
      const host = c.port && c.port !== defaultPorts[c.kind] ? `${c.host}:${c.port}` : c.host;
      return c.user ? `${c.user}@${host}` : host;
    }
  }
}

export function displayName(c: Connection): string {
  return c.name.trim() || (c.kind === "hyperv" ? c.vmName.trim() : c.host.trim()) || "Ohne Namen";
}

export function filterConnections(list: Connection[], query: string): Connection[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return list;
  return list.filter((c) => {
    const text = [c.name, c.host, c.user, c.group, c.notes, c.vmName, kindLabels[c.kind]].join(" ").toLowerCase();
    return words.every((w) => text.includes(w));
  });
}

export type Sort = "name" | "recent";

export interface Group {
  name: string;
  favorites?: boolean;
  items: Connection[];
}

const collator = new Intl.Collator("de", { sensitivity: "base", numeric: true });

/**
 * Favorites first, then by group (connections without group last). Within a group by name,
 * or most recently used first.
 */
export function groupConnections(list: Connection[], sort: Sort = "name"): Group[] {
  const order = (items: Connection[]) =>
    items.sort((a, b) =>
      sort === "recent" && (a.lastUsed ?? 0) !== (b.lastUsed ?? 0)
        ? (b.lastUsed ?? 0) - (a.lastUsed ?? 0)
        : collator.compare(displayName(a), displayName(b)),
    );
  const groups = new Map<string, Connection[]>();
  for (const c of list.filter((c) => !c.favorite)) {
    const name = c.group.trim();
    groups.set(name, [...(groups.get(name) ?? []), c]);
  }
  const favorites = list.filter((c) => c.favorite);
  return [
    ...(favorites.length ? [{ name: "Favoriten", favorites: true, items: order(favorites) }] : []),
    ...[...groups.entries()]
      .sort(([a], [b]) => (!a ? 1 : !b ? -1 : collator.compare(a, b)))
      .map(([name, items]) => ({ name, items: order(items) })),
  ];
}

/** Problems that prevent saving, or null. */
export function validate(c: Connection): string | null {
  const host = c.host.trim();
  const hostPattern = /^[\w.\-:[\]%]+$/;
  switch (c.kind) {
    case "web":
      if (!host) return "Bitte eine Adresse eingeben.";
      return /\s/.test(host) ? "Die Adresse darf keine Leerzeichen enthalten." : null;
    case "serial":
      if (!/^COM\d{1,3}$/i.test(host)) return "Bitte eine Schnittstelle wie COM3 eingeben.";
      return null;
    case "hyperv":
      if (host && (host.startsWith("-") || !hostPattern.test(host))) return "Der Server enthält ungültige Zeichen.";
      if (!c.vmName.trim()) return "Bitte den Namen der VM eingeben.";
      return /["\\]/.test(c.vmName) ? "Der VM-Name darf kein \" oder \\ enthalten." : null;
  }
  if (!host) return "Bitte einen Host eingeben.";
  if (host.startsWith("-") || !hostPattern.test(host)) return "Der Host enthält ungültige Zeichen.";
  if (c.port !== null && !(Number.isInteger(c.port) && c.port >= 1 && c.port <= 65535)) return "Der Port muss zwischen 1 und 65535 liegen.";
  if (c.kind === "rdp") {
    if ((c.width === null) !== (c.height === null)) return "Bitte Breite und Höhe angeben – oder beides leer lassen.";
    if (c.width !== null && c.height !== null && [c.width, c.height].some((v) => !(Number.isInteger(v) && v >= 200 && v <= 8192)))
      return "Die Fenstergröße muss zwischen 200 und 8192 Pixeln liegen.";
    if (c.gateway.trim() && !hostPattern.test(c.gateway.trim())) return "Das Gateway enthält ungültige Zeichen.";
  }
  if (c.kind === "ssh") {
    if (c.jumpHost.trim() && (c.jumpHost.trim().startsWith("-") || !/^[\w.\-:@[\],%]+$/.test(c.jumpHost.trim())))
      return "Der Jump-Host enthält ungültige Zeichen.";
    for (const line of c.forwards.split("\n").map((l) => l.trim()).filter(Boolean)) {
      if (!/^-?[LRD]\s*[\w.:[\]/*][\w.:[\]/*-]*$/i.test(line)) return `Ungültige Weiterleitung: „${line}“ (Beispiel: L 8080:localhost:80)`;
    }
  }
  return null;
}

/** Host and port for the reachability check; null where it makes no sense (serial, Hyper-V). */
export function probeTarget(c: Connection): { host: string; port: number } | null {
  const host = c.host.trim();
  if (!host) return null;
  if (c.kind === "web") {
    try {
      const url = new URL(/^https?:\/\//i.test(host) ? host : `https://${host}`);
      return { host: url.hostname, port: Number(url.port) || (url.protocol === "http:" ? 80 : 443) };
    } catch {
      return null;
    }
  }
  const port = c.port ?? defaultPorts[c.kind];
  return port ? { host, port } : null;
}

/** Whether the open connection `a` is the saved connection `c` (by host name, IP or resolved IP, and port). */
export function isActive(c: Connection, a: Active, ips: string[] = []): boolean {
  const host = c.host.trim().toLowerCase();
  return (
    c.kind === a.kind &&
    (host === a.host.toLowerCase() || host === a.address || ips.includes(a.address)) &&
    (c.port ?? defaultPorts[c.kind]) === a.port
  );
}

/** A new connection prefilled from an open one, for saving it. */
export function fromActive(a: Active): Connection {
  return {
    ...newConnection(a.kind),
    name: a.host,
    host: a.host,
    port: a.port === defaultPorts[a.kind] ? null : a.port,
    user: a.user,
    keyFile: a.keyFile,
  };
}

/** Whether a found connection is already in the list (same kind, host and port). */
export function isSaved(candidate: Candidate, list: Connection[]): boolean {
  const port = candidate.port ?? defaultPorts[candidate.kind];
  return list.some(
    (c) => c.kind === candidate.kind && c.host.trim().toLowerCase() === candidate.host.toLowerCase() && (c.port ?? defaultPorts[c.kind]) === port,
  );
}

export function fromCandidate(candidate: Candidate, group: string): Connection {
  return {
    ...newConnection(candidate.kind, group),
    name: candidate.name,
    host: candidate.host,
    port: candidate.port,
    user: candidate.user,
    baud: candidate.baud,
    notes: candidate.detail && candidate.source === "~/.ssh/config" ? `Aus ~/.ssh/config: ${candidate.detail}` : "",
  };
}

/** Adds the connections of a backup that are not in the list yet (by id); returns how many. */
export function mergeBackup(list: Connection[], backup: unknown): number {
  const ids = new Set(list.map((c) => c.id));
  const added = normalize(backup).filter((c) => !ids.has(c.id));
  list.push(...added);
  return added.length;
}

export function formatLastUsed(time: number | null, now = Date.now()): string {
  if (!time) return "noch nie";
  const minutes = Math.round((now - time) / 60000);
  if (minutes < 1) return "gerade eben";
  if (minutes < 60) return `vor ${minutes} min`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `vor ${hours} h`;
  return new Date(time).toLocaleString("de-DE", { dateStyle: "medium", timeStyle: "short" });
}
