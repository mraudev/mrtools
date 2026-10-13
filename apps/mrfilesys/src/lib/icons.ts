import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { extension } from "./fileTypes";

export type IconSize = "small" | "large";

/** Files that bring their own icon – looked up one by one instead of by extension. */
const OWN_ICON = new Set(["exe", "lnk", "ico", "url", "msi", "scr", "cpl", "appref-ms"]);

/**
 * What decides the icon: the extension for plain files (no disk access, shared by all of them),
 * the path for folders (special folders, own icons) and files with their own icon.
 */
export function iconKey(entry: { name: string; isDir: boolean; path?: string }): string {
  if (entry.isDir) return entry.path ? `path:${entry.path}` : "dir";
  const ext = extension(entry.name);
  if (entry.path && OWN_ICON.has(ext)) return `path:${entry.path}`;
  return `ext:${ext ? `.${ext}` : ""}`;
}

/** `size|key` → PNG data URL; `null` while loading or when Windows has none. */
export const icons = reactive(new Map<string, string | null>());

const requested = new Set<string>();
const queue: Record<IconSize, Set<string>> = { small: new Set(), large: new Set() };
let timer: ReturnType<typeof setTimeout> | undefined;

async function flush() {
  timer = undefined;
  for (const size of ["small", "large"] as const) {
    const keys = [...queue[size]];
    queue[size].clear();
    if (!keys.length) continue;
    try {
      const result = await invoke<Record<string, string | null>>("file_icons", { keys, size });
      for (const [key, icon] of Object.entries(result)) icons.set(`${size}|${key}`, icon);
    } catch {
      // Without real icons the type symbols stay – nothing to report.
    }
  }
}

/** Asks for an icon; requests of the same moment go to the backend together. */
export function requestIcon(key: string, size: IconSize) {
  const id = `${size}|${key}`;
  if (requested.has(id)) return;
  requested.add(id);
  queue[size].add(key);
  timer ??= setTimeout(flush, 16);
}
