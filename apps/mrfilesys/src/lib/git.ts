import { reactive, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { settings, state } from "./store";

export type GitStatus = "clean" | "ignored" | "untracked" | "staged" | "modified" | "conflict";

/** As sent by `git_status` (paths relative to the root, with `/`). */
interface RepoStatus {
  root: string;
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  files: Record<string, Exclude<GitStatus, "clean">>;
  whole: Record<string, Exclude<GitStatus, "clean">>;
  dirs: Record<string, Exclude<GitStatus, "clean">>;
}

export interface Change {
  /** Relative to the root, with `/`, as git writes it. */
  path: string;
  status: Exclude<GitStatus, "clean">;
}

export interface Commit {
  hash: string;
  short: string;
  author: string;
  /** ISO 8601. */
  date: string;
  subject: string;
}

/** The repository with lower-case keys – Windows paths ignore case. */
export interface Repo extends RepoStatus {
  /** Changed, staged and conflicting files (not untracked or ignored). */
  changed: number;
  /** Everything git reports except ignored entries, in git's spelling, most important first. */
  changes: Change[];
}

const RANK: Record<GitStatus, number> = { conflict: 0, modified: 1, staged: 2, untracked: 3, ignored: 4, clean: 5 };

/** Changes inside the folder `rel` (`""`: the whole repository). */
export function changesBelow(repo: Repo, rel: string): Change[] {
  const prefix = rel ? `${rel}/` : "";
  return repo.changes.filter((c) => c.path.toLowerCase().startsWith(prefix));
}

export async function lastCommit(path: string): Promise<Commit | null> {
  return invoke<Commit | null>("git_last_commit", { path });
}

export const git = reactive({
  repo: null as Repo | null,
  /** Installed git version; `undefined` while unknown, `null` if git is missing. */
  version: undefined as string | null | undefined,
  error: "",
});

const lower = <T>(map: Record<string, T>) =>
  Object.fromEntries(Object.entries(map).map(([k, v]) => [k.toLowerCase(), v])) as Record<string, T>;

export function prepare(status: RepoStatus): Repo {
  const files = lower(status.files);
  const changes: Change[] = [
    ...Object.entries(status.files).map(([path, s]) => ({ path, status: s })),
    ...Object.entries(status.whole).map(([path, s]) => ({ path: `${path}/`, status: s })),
  ]
    .filter((c) => c.status !== "ignored")
    .sort((a, b) => RANK[a.status] - RANK[b.status] || a.path.localeCompare(b.path));
  return {
    ...status,
    files,
    whole: lower(status.whole),
    dirs: lower(status.dirs),
    changed: Object.values(files).filter((s) => s !== "ignored" && s !== "untracked").length,
    changes,
  };
}

/** `C:\repo\src\a.ts` → `src/a.ts` (lower case), `null` outside of the repository. */
export function relative(path: string, root: string): string | null {
  const p = path.toLowerCase();
  const r = root.toLowerCase().replace(/\\$/, "");
  if (p === r) return "";
  if (!p.startsWith(`${r}\\`)) return null;
  return p.slice(r.length + 1).replace(/\\/g, "/");
}

/** Git state of a file or folder, `null` where nothing should be shown. */
export function statusOf(repo: Repo | null, entry: { path: string; isDir: boolean }, showIgnored: boolean): GitStatus | null {
  if (!repo) return null;
  const rel = relative(entry.path, repo.root);
  if (!rel || rel === ".git" || rel.startsWith(".git/")) return null;
  // Inside (or itself) a folder that is untracked or ignored as a whole?
  const parts = rel.split("/");
  for (let i = 1; i <= parts.length; i++) {
    const whole = repo.whole[parts.slice(0, i).join("/")];
    if (whole) return whole === "ignored" && !showIgnored ? null : whole;
  }
  const own = repo.files[rel] ?? (entry.isDir ? repo.dirs[rel] : undefined);
  if (own === "ignored") return showIgnored ? own : null;
  return own ?? "clean";
}

/** Status as shown with the current settings. */
export function gitStatus(entry: { path: string; isDir: boolean }): GitStatus | null {
  if (!settings.git.enabled) return null;
  const status = statusOf(git.repo, entry, settings.git.ignored);
  return status === "clean" && !settings.git.clean ? null : status;
}

const NAME_COLORS: Record<GitStatus, string> = {
  clean: "",
  modified: "text-amber-600 dark:text-amber-400",
  staged: "text-sky-600 dark:text-sky-400",
  untracked: "text-emerald-600 dark:text-emerald-400",
  conflict: "text-red-600 dark:text-red-400",
  ignored: "text-muted-foreground",
};

/** Text colour for a name, like VS Code's explorer (empty when switched off). */
export function nameColor(status: GitStatus | null): string {
  return status && settings.git.colorNames ? NAME_COLORS[status] : "";
}

let token = 0;

async function load() {
  const mine = ++token;
  const path = state.path;
  if (!settings.git.enabled || !path) {
    git.repo = null;
    return;
  }
  try {
    // Ignored files are needed to tell them apart from unchanged ones when those get a check mark.
    const ignored = settings.git.ignored || settings.git.clean;
    const status = await invoke<RepoStatus | null>("git_status", { path, ignored });
    if (mine !== token) return;
    git.repo = status ? prepare(status) : null;
    git.error = "";
  } catch (e) {
    if (mine !== token) return;
    git.repo = null;
    git.error = String(e);
  }
}

export async function initGit() {
  // A new or reloaded folder (navigation, F5, back in the window, after own changes) reads the status again.
  watch(
    () => [state.entries, settings.git.enabled, settings.git.ignored, settings.git.clean],
    load,
    { immediate: true },
  );
  git.version = await invoke<string | null>("git_version").catch(() => null);
}
