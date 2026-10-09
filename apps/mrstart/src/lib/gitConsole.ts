import { reactive } from "vue";
import { api } from "./api";
import { refresh } from "./store";
import type { GitAction, Project } from "./types";

export interface ConsoleLine {
  kind: "command" | "output" | "error" | "success";
  text: string;
}

export const gitConsole = reactive({
  open: false,
  running: false,
  action: "pull" as GitAction,
  project: null as Project | null,
  lines: [] as ConsoleLine[],
});

// Output arrives in arbitrary chunks; git progress lines are rewritten with `\r`.
let lineOpen = false;
let overwrite = false;

function appendOutput(text: string) {
  for (const part of text.split(/(\r\n|\n|\r)/)) {
    if (part === "\n" || part === "\r\n") {
      if (!lineOpen) gitConsole.lines.push({ kind: "output", text: "" });
      lineOpen = false;
      overwrite = false;
    } else if (part === "\r") {
      overwrite = true;
    } else if (part) {
      const last = gitConsole.lines[gitConsole.lines.length - 1];
      if (lineOpen && last) {
        last.text = overwrite ? part : last.text + part;
      } else {
        gitConsole.lines.push({ kind: "output", text: part });
        lineOpen = true;
      }
      overwrite = false;
    }
  }
}

function addLine(kind: ConsoleLine["kind"], text: string) {
  gitConsole.lines.push({ kind, text });
  lineOpen = false;
  overwrite = false;
}

export async function runGit(project: Project, action: GitAction) {
  if (gitConsole.running) return;
  Object.assign(gitConsole, { open: true, running: true, action, project, lines: [] });
  lineOpen = false;
  overwrite = false;
  try {
    await api.gitRun(project.path, action, (event) =>
      event.kind === "command" ? addLine("command", event.text) : appendOutput(event.text),
    );
    addLine("success", "Erfolgreich abgeschlossen.");
  } catch (e) {
    addLine("error", String(e));
  } finally {
    gitConsole.running = false;
    refresh();
  }
}
