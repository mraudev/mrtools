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

export interface Settings {
  watchedDirectories: string[];
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
}

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
    watchedDirectories: [],
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

export const WATCHED = "watched";
export const SETTINGS = "settings";

/** Names of tabs: a category name, {@link WATCHED} or {@link SETTINGS}. */
export type View = string;

export interface WatchedEntry {
  name: string;
  path: string;
  apps: string[];
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

export interface PullRequestResult {
  pulls: PullRequest[];
  errors: { paths: string[]; message: string }[];
}
