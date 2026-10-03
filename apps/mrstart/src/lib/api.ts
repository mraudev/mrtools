import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  BranchInfo,
  Config,
  Dashboard,
  DashboardPull,
  GitAction,
  GitEvent,
  PullRequestResult,
  SecretName,
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

  branchInfo: (path: string, giteaHost: string) =>
    invoke<BranchInfo | null>("branch_info", { path, giteaHost }),
  gitRun: (path: string, action: GitAction, onEvent: (event: GitEvent) => void) => {
    const channel = new Channel<GitEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("git_run", { path, action, onEvent: channel });
  },

  pullRequests: (query: { paths: string[]; giteaHost: string }) =>
    invoke<PullRequestResult>("pull_requests", { query }),

  dashboard: (giteaHost: string) => invoke<Dashboard>("dashboard", { giteaHost }),
  updatePullBranch: (pull: DashboardPull, rebase: boolean, giteaHost: string) =>
    invoke<void>("update_pull_branch", {
      request: {
        provider: pull.provider,
        nodeId: pull.nodeId,
        owner: pull.owner,
        repo: pull.repo,
        number: pull.number,
        headSha: pull.headSha,
        rebase,
        giteaHost,
      },
    }),

  // Tokens can only be written or deleted – the backend never returns them.
  secretStatus: () => invoke<Record<SecretName, boolean>>("secret_status"),
  setSecret: (name: SecretName, value: string) => invoke<void>("set_secret", { name, value }),
  deleteSecret: (name: SecretName) => invoke<void>("delete_secret", { name }),
};
