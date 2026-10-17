import type { App, Release } from "./types";
import { appStatus } from "./version";

/** Category per app folder (trial; apps without an entry land in "Sonstige"). */
const CATEGORIES: Record<string, string> = {
  mrprocs: "System",
  mrports: "System",
  mrdiskspace: "System",
  mrfilesys: "System",
  mrremotemanager: "Netzwerk & Kommunikation",
  mrphone: "Netzwerk & Kommunikation",
  mrstart: "Entwicklung & Werkzeuge",
  mrtextedit: "Entwicklung & Werkzeuge",
  mrhome: "Zuhause",
  mrfakt: "Büro",
};

const OTHER = "Sonstige";

/** Order of the categories; unknown ones follow alphabetically, "Sonstige" last. */
const ORDER = ["Büro", "System", "Netzwerk & Kommunikation", "Entwicklung & Werkzeuge", "Zuhause"];

export function categoryOf(folder: string): string {
  return CATEGORIES[folder] ?? OTHER;
}

/** 0: update available, 1: installed and current, 2: not installed. */
export function rank(app: App, releases: Record<string, Release>): number {
  const status = appStatus(app.installed ? app.installed.version : undefined, releases[app.folder]?.version ?? null);
  return status.kind === "outdated" ? 0 : status.kind === "current" ? 1 : 2;
}

const collator = new Intl.Collator("de", { sensitivity: "base", numeric: true });

/** Apps with an update first, then the other installed ones, then the rest – each by name. */
export function sortApps(apps: App[], releases: Record<string, Release>): App[] {
  return [...apps].sort((a, b) => rank(a, releases) - rank(b, releases) || collator.compare(a.name, b.name));
}

export interface Category {
  name: string;
  apps: App[];
}

export function groupApps(apps: App[], releases: Record<string, Release>): Category[] {
  const groups = new Map<string, App[]>();
  for (const app of sortApps(apps, releases)) {
    const name = categoryOf(app.folder);
    groups.set(name, [...(groups.get(name) ?? []), app]);
  }
  const position = (name: string) => (name === OTHER ? Infinity : ORDER.includes(name) ? ORDER.indexOf(name) : ORDER.length);
  return [...groups.entries()]
    .sort(([a], [b]) => position(a) - position(b) || collator.compare(a, b))
    .map(([name, apps]) => ({ name, apps }));
}
