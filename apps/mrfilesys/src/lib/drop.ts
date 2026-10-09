import { isInside, parentPath } from "./paths";

export type DropOp = "copy" | "move";

/** `C:` or `\\server\share` – moves within it are cheap renames. */
function volume(path: string): string {
  const unc = /^\\\\[^\\]+\\[^\\]+/.exec(path);
  return (unc ? unc[0] : path.slice(0, 2)).toLowerCase();
}

/**
 * What dropping `paths` on the folder `target` does – like Explorer: move within a drive, copy across
 * drives; Ctrl forces copy, Shift forces move. `null` if the drop is impossible or would change nothing.
 */
export function dropOp(paths: string[], target: string, keys: { ctrl?: boolean; shift?: boolean } = {}): DropOp | null {
  if (!target || !paths.length) return null;
  // A folder cannot go into itself or one of its subfolders.
  if (paths.some((p) => isInside(target, p))) return null;
  const op = keys.ctrl ? "copy" : keys.shift ? "move" : paths.every((p) => volume(p) === volume(target)) ? "move" : "copy";
  if (op === "move" && paths.every((p) => parentPath(p).toLowerCase() === target.toLowerCase())) return null;
  return op;
}
