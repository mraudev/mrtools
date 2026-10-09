export interface Favorite {
  path: string;
  isDir: boolean;
}

const key = (path: string) => path.toLowerCase();

/** Saved favorites; older versions stored plain folder paths. */
export function readFavorites(raw: unknown): Favorite[] {
  if (!Array.isArray(raw)) return [];
  return raw.flatMap((item): Favorite[] => {
    if (typeof item === "string") return [{ path: item, isDir: true }];
    if (item && typeof item.path === "string") return [{ path: item.path, isDir: item.isDir !== false }];
    return [];
  });
}

/**
 * Puts `items` at `index` of `list` (an index into the list as it is now). Items already in the list
 * are moved there, so the same function pins and reorders.
 */
export function insertFavorites(list: Favorite[], items: Favorite[], index: number): Favorite[] {
  const moving = new Set(items.map((f) => key(f.path)));
  const unique = items.filter((f, i) => items.findIndex((g) => key(g.path) === key(f.path)) === i);
  const before = list.slice(0, index).filter((f) => !moving.has(key(f.path))).length;
  const rest = list.filter((f) => !moving.has(key(f.path)));
  rest.splice(before, 0, ...unique);
  return rest;
}

/** True if dropping `paths` at `index` would change nothing (reordering onto its own place). */
export function samePlace(list: Favorite[], paths: string[], index: number): boolean {
  const moved = insertFavorites(
    list,
    paths.map((path) => list.find((f) => key(f.path) === key(path)) ?? { path, isDir: true }),
    index,
  );
  return moved.length === list.length && moved.every((f, i) => key(f.path) === key(list[i].path));
}
