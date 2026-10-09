/** `v1.2.3` → `1.2.3`. */
export function cleanVersion(version: string): string {
  return version.trim().replace(/^v/i, "");
}

/** Compares dotted versions numerically (`1.10.0` > `1.9.2`); a pre-release sorts before its release. */
export function compareVersions(a: string, b: string): number {
  const [coreA, preA = ""] = cleanVersion(a).split("-", 2);
  const [coreB, preB = ""] = cleanVersion(b).split("-", 2);
  const partsA = coreA.split(".").map((n) => parseInt(n, 10) || 0);
  const partsB = coreB.split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
    const diff = (partsA[i] ?? 0) - (partsB[i] ?? 0);
    if (diff) return Math.sign(diff);
  }
  if (preA === preB) return 0;
  if (!preA) return 1;
  if (!preB) return -1;
  return preA < preB ? -1 : 1;
}

export type Status =
  | { kind: "missing" }
  | { kind: "current"; latest: string | null }
  | { kind: "outdated"; latest: string };

/** Installed version (`undefined` = not installed) vs. the latest release. */
export function appStatus(installed: string | null | undefined, latest: string | null): Status {
  if (installed === undefined) return { kind: "missing" };
  if (installed && latest && compareVersions(installed, latest) < 0) return { kind: "outdated", latest };
  return { kind: "current", latest };
}
