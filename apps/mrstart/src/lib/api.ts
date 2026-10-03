import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  Config,
  GitAction,
  GitEvent,
  PullRequestResult,
  WatchedEntry,
} from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  loadConfig: () => invoke<unknown>("load_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  configPath: () => invoke<string>("config_path"),

  existingFiles: (directory: string, patterns: string[]) =>
    invoke<string[]>("existing_files", { directory, patterns }),
  watchedProjects: (directories: string[], defaultApps: string[]) =>
    invoke<WatchedEntry[]>("watched_projects", { directories, defaultApps }),

  runCommand: (command: string, cwd?: string) =>
    invoke<void>("run_command", { command, cwd }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  openGitTool: (tool: string, executable: string, action: "status" | "log", path: string) =>
    invoke<void>("open_git_tool", { tool, executable, action, path }),

  gitBranch: (path: string) => invoke<string | null>("git_branch", { path }),
  gitRun: (path: string, action: GitAction, onEvent: (event: GitEvent) => void) => {
    const channel = new Channel<GitEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("git_run", { path, action, onEvent: channel });
  },

  pullRequests: (query: {
    paths: string[];
    githubToken: string;
    giteaHost: string;
    giteaToken: string;
  }) => invoke<PullRequestResult>("pull_requests", { query }),
};
