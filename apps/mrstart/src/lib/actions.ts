import { api } from "./api";
import { store } from "./store";
import { toastError } from "@mrtools/ui/lib/toast";
import { DEFAULT_COMMANDS, type LaunchKind, type Project } from "./types";

export function commandTemplate(kind: LaunchKind): string {
  const s = store.config.settings;
  const configured = { editor: s.editorCommand, terminal: s.terminalCommand, bash: s.bashCommand }[kind];
  return configured.trim() || DEFAULT_COMMANDS[kind];
}

/** Opens the project in the editor, a terminal or Git Bash. */
export function launch(kind: LaunchKind, project: Project) {
  const command = commandTemplate(kind).replaceAll("{path}", project.path);
  api.runCommand(command, project.path).catch((e) => toastError("Start fehlgeschlagen", e));
}

export function runProjectCommand(command: string, project: Project) {
  api.runCommand(command, project.path).catch((e) => toastError("Befehl fehlgeschlagen", e));
}

export function openPath(path: string) {
  api.openPath(path).catch((e) => toastError("Öffnen fehlgeschlagen", e));
}

export function openGitTool(project: Project, action: "status" | "log") {
  const s = store.config.settings;
  const executable = s.gitTool === "tortoise" ? s.tortoisePath : s.forkPath;
  api
    .openGitTool(s.gitTool, executable, action, project.path)
    .catch((e) => toastError("Git-Programm konnte nicht gestartet werden", e));
}

/** File name without directory and extension, used as button caption. */
export function baseName(path: string): string {
  const file = path.split(/[\\/]/).pop() ?? path;
  const dot = file.lastIndexOf(".");
  return dot > 0 ? file.slice(0, dot) : file;
}
