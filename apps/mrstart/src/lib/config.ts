// Pure validation/migration of the stored configuration (no side effects,
// unit-tested in config.test.ts).
import { defaultSettings, folderName, type Config, type Project, type Settings, type WatchedFolder } from "./types";

function normalizeProject(raw: any): Project {
  const commands = Array.isArray(raw?.commands) ? raw.commands : [];
  return {
    id: String(raw?.id ?? crypto.randomUUID()),
    name: String(raw?.name ?? ""),
    category: String(raw?.category ?? ""),
    path: String(raw?.path ?? ""),
    version: raw?.version == null ? "" : String(raw.version),
    apps: Array.isArray(raw?.apps) ? raw.apps.map(String) : [],
    commands: commands.map((c: any) => ({
      caption: String(c?.caption ?? ""),
      command: String(c?.command ?? ""),
    })),
  };
}

const stringList = (value: unknown): string[] =>
  Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : [];

/** Validates watched folders and migrates the former `watchedDirectories` list
 *  (one shared tab) to one tab per folder, named after the folder. */
function normalizeWatchedFolders(raw: any): WatchedFolder[] {
  const list: any[] = Array.isArray(raw?.watchedFolders)
    ? raw.watchedFolders
    : Array.isArray(raw?.watchedDirectories)
      ? raw.watchedDirectories.map((path: unknown) => ({ path }))
      : [];
  return list
    .filter((f) => typeof f?.path === "string" && f.path.trim() !== "")
    .map((f) => ({ path: f.path, category: typeof f.category === "string" ? f.category : folderName(f.path) }));
}

/** Takes only known keys with the expected type – unknown entries (e.g. tokens
 *  from an imported radstart file) are dropped and never written back. */
export function normalizeSettings(raw: any): Settings {
  const settings = defaultSettings();
  for (const key of Object.keys(settings) as (keyof Settings)[]) {
    const value = raw?.[key];
    if (value !== undefined && typeof value === typeof settings[key] && Array.isArray(value) === Array.isArray(settings[key])) {
      (settings as any)[key] = value;
    }
  }
  settings.watchedFolders = normalizeWatchedFolders(raw);
  settings.defaultApps = stringList(settings.defaultApps);
  settings.pinnedPaths = stringList(settings.pinnedPaths);
  settings.tabOrder = stringList(settings.tabOrder);
  if (![0, 5, 15, 30, 60].includes(settings.autoFetchMinutes)) settings.autoFetchMinutes = 0;
  return settings;
}

export function normalizeConfig(raw: any): Config {
  return {
    projects: Array.isArray(raw?.projects) ? raw.projects.map(normalizeProject) : [],
    settings: normalizeSettings(raw?.settings),
  };
}

/** Tab of a watched folder; an empty name falls back to the folder name. */
export const watchedCategory = (folder: WatchedFolder) => folder.category.trim() || folderName(folder.path);

/** Tabs in the user's order (drag & drop); new tabs are appended alphabetically. */
export function orderTabs(categories: string[], order: string[]): string[] {
  const collator = new Intl.Collator("de", { numeric: true, sensitivity: "base" });
  const rank = (c: string) => {
    const i = order.indexOf(c);
    return i < 0 ? Number.MAX_SAFE_INTEGER : i;
  };
  return [...categories].sort((a, b) => rank(a) - rank(b) || collator.compare(a, b));
}
