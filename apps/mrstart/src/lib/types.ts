export interface ProjectCommand {
  caption: string;
  command: string;
}

export interface Project {
  id: string;
  name: string;
  category: string;
  path: string;
  /** Optional free-text version/label, shown as a badge and used for sorting. */
  version: string;
  /** Absolute paths of files that get a launch button (.sln, .exe, …). */
  apps: string[];
  commands: ProjectCommand[];
}

export type GitTool = "fork" | "tortoise";
export type Theme = "dark" | "light" | "system";

/** A folder whose subfolders appear as projects in the tab `category`. */
export interface WatchedFolder {
  path: string;
  category: string;
}

export interface Settings {
  watchedFolders: WatchedFolder[];
  /** File names or `*.ext` patterns that are offered as apps automatically. */
  defaultApps: string[];
  /** Command templates with a `{path}` placeholder; empty = built-in default. */
  editorCommand: string;
  terminalCommand: string;
  bashCommand: string;
  gitTool: GitTool;
  /** Empty = auto-detect. */
  forkPath: string;
  tortoisePath: string;
  /** Tokens are not part of the settings; they live in the credential store. */
  giteaHost: string;
  theme: Theme;
  accent: string;
  /** Where "Review with Claude" opens: the Claude desktop app or Claude Code in a terminal. */
  reviewTarget: ReviewTarget;
}

export type ReviewTarget = "desktop" | "terminal";

export interface Config {
  projects: Project[];
  settings: Settings;
}

export type LaunchKind = "editor" | "terminal" | "bash";

export const DEFAULT_COMMANDS: Record<LaunchKind, string> = {
  editor: 'code "{path}"',
  terminal: 'wt.exe -d "{path}"',
  bash: '"C:\\Program Files\\Git\\git-bash.exe" --cd="{path}"',
};

export function defaultSettings(): Settings {
  return {
    watchedFolders: [],
    defaultApps: ["*.sln", "*.slnx"],
    editorCommand: "",
    terminalCommand: "",
    bashCommand: "",
    gitTool: "fork",
    forkPath: "",
    tortoisePath: "",
    giteaHost: "",
    theme: "dark",
    accent: "amber",
    reviewTarget: "desktop",
  };
}

export function emptyProject(category = ""): Project {
  return {
    id: crypto.randomUUID(),
    name: "",
    category,
    path: "",
    version: "",
    apps: [],
    commands: [],
  };
}

export const SETTINGS = "settings";
export const PULLS = "pulls";

/** Names of tabs: `cat:<category>`, {@link PULLS} or {@link SETTINGS}. */
export type View = string;

export interface WatchedEntry {
  name: string;
  path: string;
  apps: string[];
  /** The watched folder the entry was found in. */
  root: string;
}

/** Projects found in watched folders are generated, not edited. */
export const WATCHED_ID_PREFIX = "watched:";
export const isWatched = (project: Project) => project.id.startsWith(WATCHED_ID_PREFIX);

/** Last path segment, e.g. the folder name. */
export function folderName(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? path;
}

export type GitAction = "pull" | "push";

export type GitEvent =
  | { kind: "command"; text: string }
  | { kind: "output"; text: string };

export interface BranchInfo {
  /** Empty for a detached HEAD. */
  branch: string;
  /** Web page to create a pull request for the branch (GitHub/Gitea only). */
  createPullUrl: string | null;
}

export interface PullRequest {
  repo: string;
  branch: string;
  number: number;
  title: string;
  url: string;
  /** Project folders that have the branch checked out. */
  paths: string[];
}

export type SecretName = "github" | "gitea";

export type PullStatus = "behind" | "conflict" | "clean" | "blocked" | "unstable" | "draft" | "unknown";

export interface DashboardPull {
  provider: "gitea" | "github";
  nodeId: string;
  owner: string;
  repo: string;
  number: number;
  title: string;
  url: string;
  isDraft: boolean;
  createdAt: string;
  updatedAt: string;
  head: string;
  base: string;
  headSha: string;
  baseSha: string;
  /** Commit date of the newest base-branch state the PR branch contains. */
  baseDate: string | null;
  status: PullStatus;
  canUpdate: boolean;
  reviewDecision: string | null;
  author: string;
}

export interface Dashboard {
  authored: DashboardPull[];
  reviewRequests: DashboardPull[];
  errors: string[];
}

export interface PullRequestResult {
  pulls: PullRequest[];
  errors: { paths: string[]; message: string }[];
}
