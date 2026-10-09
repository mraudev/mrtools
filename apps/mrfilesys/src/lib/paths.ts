/**
 * Windows paths. The empty string stands for "Dieser PC" – the level above the drives.
 */

/** `c:` → `C:\`, trailing backslashes removed (except at a drive root). */
export function normalizePath(path: string): string {
  let p = path.trim().replace(/\//g, "\\");
  if (/^[a-z]:$/i.test(p)) p += "\\";
  if (/^[a-z]:\\/i.test(p)) p = p[0].toUpperCase() + p.slice(1);
  while (p.length > 3 && p.endsWith("\\") && !/^\\\\[^\\]+\\[^\\]+\\$/.test(p)) p = p.slice(0, -1);
  return p;
}

export function isDriveRoot(path: string): boolean {
  return /^[A-Z]:\\$/i.test(path);
}

/** Parent folder, `""` for a drive root. */
export function parentPath(path: string): string {
  if (!path || isDriveRoot(path)) return "";
  const i = path.lastIndexOf("\\");
  if (i < 0) return "";
  // `C:\foo` → `C:\`
  if (i === 2 && path[1] === ":") return path.slice(0, 3);
  // `\\server\share` has no parent we can list
  if (path.startsWith("\\\\") && path.indexOf("\\", 2) === i) return "";
  return path.slice(0, i);
}

export function joinPath(dir: string, name: string): string {
  return dir.endsWith("\\") ? dir + name : `${dir}\\${name}`;
}

export function baseName(path: string): string {
  if (isDriveRoot(path)) return path.slice(0, 2);
  return path.slice(path.lastIndexOf("\\") + 1) || path;
}

export interface Crumb {
  label: string;
  path: string;
}

/** Breadcrumbs from the drive down to `path`. */
export function crumbs(path: string): Crumb[] {
  const out: Crumb[] = [];
  for (let p = path; p; p = parentPath(p)) {
    out.unshift({ label: baseName(p), path: p });
    if (p.startsWith("\\\\") && !parentPath(p)) break;
  }
  return out;
}

/** True if `path` is `dir` or lies below it. */
export function isInside(path: string, dir: string): boolean {
  const a = path.toLowerCase();
  const b = dir.toLowerCase();
  return a === b || a.startsWith(b.endsWith("\\") ? b : `${b}\\`);
}
