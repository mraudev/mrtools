import type { Active, Connection, Kind } from "./types";

export const kindLabels: Record<Kind, string> = { rdp: "Remotedesktop", ssh: "SSH", web: "Web" };
export const defaultPorts: Record<Kind, number | null> = { rdp: 3389, ssh: 22, web: null };

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
    fullscreen: false,
    multimon: false,
    keyFile: "",
    lastUsed: null,
  };
}

/** Fills in fields missing in older or hand-edited files. */
export function normalize(list: unknown): Connection[] {
  if (!Array.isArray(list)) return [];
  return list
    .filter((c): c is Partial<Connection> => !!c && typeof c === "object")
    .map((c) => ({ ...newConnection(), ...c }) as Connection)
    .filter((c) => c.kind in kindLabels);
}

/** "user@host:port" as shown in the list; for web the address itself. */
export function address(c: Connection): string {
  if (c.kind === "web") return c.host;
  const host = c.port && c.port !== defaultPorts[c.kind] ? `${c.host}:${c.port}` : c.host;
  return c.user ? `${c.user}@${host}` : host;
}

export function displayName(c: Connection): string {
  return c.name.trim() || c.host.trim() || "Ohne Namen";
}

export function filterConnections(list: Connection[], query: string): Connection[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return list;
  return list.filter((c) => {
    const text = [c.name, c.host, c.user, c.group, c.notes, kindLabels[c.kind]].join(" ").toLowerCase();
    return words.every((w) => text.includes(w));
  });
}

export interface Group {
  name: string;
  items: Connection[];
}

const collator = new Intl.Collator("de", { sensitivity: "base", numeric: true });

/** Groups by name (connections without group last), each sorted by name. */
export function groupConnections(list: Connection[]): Group[] {
  const groups = new Map<string, Connection[]>();
  for (const c of list) {
    const name = c.group.trim();
    groups.set(name, [...(groups.get(name) ?? []), c]);
  }
  return [...groups.entries()]
    .sort(([a], [b]) => (!a ? 1 : !b ? -1 : collator.compare(a, b)))
    .map(([name, items]) => ({ name, items: items.sort((a, b) => collator.compare(displayName(a), displayName(b))) }));
}

/** Problems that prevent saving, or null. */
export function validate(c: Connection): string | null {
  const host = c.host.trim();
  if (!host) return c.kind === "web" ? "Bitte eine Adresse eingeben." : "Bitte einen Host eingeben.";
  if (c.kind === "web") return /\s/.test(host) ? "Die Adresse darf keine Leerzeichen enthalten." : null;
  if (host.startsWith("-") || !/^[\w.\-:[\]%]+$/.test(host)) return "Der Host enthält ungültige Zeichen.";
  if (c.port !== null && !(Number.isInteger(c.port) && c.port >= 1 && c.port <= 65535)) return "Der Port muss zwischen 1 und 65535 liegen.";
  return null;
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

/** Whether the open connection `a` is the saved connection `c` (by host name or IP and port). */
export function isActive(c: Connection, a: Active): boolean {
  const host = c.host.trim().toLowerCase();
  return (
    c.kind === a.kind &&
    (host === a.host.toLowerCase() || host === a.address) &&
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
