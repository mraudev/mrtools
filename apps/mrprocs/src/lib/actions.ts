import { reactive } from "vue";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { api } from "./api";
import { descendants } from "./rows";
import { procs, refresh } from "./store";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import type { ProcInfo } from "./types";

export const PRIORITIES = [
  { value: 0x100, label: "Echtzeit" },
  { value: 0x80, label: "Hoch" },
  { value: 0x8000, label: "Höher als normal" },
  { value: 0x20, label: "Normal" },
  { value: 0x4000, label: "Niedriger als normal" },
  { value: 0x40, label: "Niedrig" },
];

/** Ending these takes Windows down (blue screen or logoff). */
const CRITICAL = new Set(["system", "registry", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe", "lsass.exe", "memory compression"]);

export const isCritical = (p: ProcInfo) => p.pid <= 4 || CRITICAL.has(p.name.toLowerCase());

/** Error text for actions that typically fail without admin rights. */
function denied(e: unknown) {
  const text = String(e);
  return /zugriff|access/i.test(text) ? `${text} – mrprocs als Administrator starten hilft meist.` : text;
}

// ---------------------------------------------------------------------------
// Ending processes (with confirmation)

export const confirm = reactive({
  open: false,
  proc: null as ProcInfo | null,
  tree: false,
  /** Descendants that would end with it. */
  children: 0,
});

export function askKill(proc: ProcInfo, tree = false) {
  confirm.proc = proc;
  confirm.tree = tree;
  confirm.children = tree ? descendants(proc.pid, procs.value).length - 1 : 0;
  confirm.open = true;
}

export async function confirmKill() {
  const { proc, tree } = confirm;
  confirm.open = false;
  if (!proc) return;
  try {
    if (tree) {
      const ended = await api.killTree(proc.pid);
      toast("success", `${ended} ${ended === 1 ? "Prozess" : "Prozesse"} beendet`, proc.name);
    } else {
      await api.kill(proc.pid);
      toast("success", "Prozess beendet", `${proc.name} (PID ${proc.pid})`);
    }
  } catch (e) {
    toast("error", `${proc.name} konnte nicht beendet werden`, denied(e));
  }
  refresh();
}

// ---------------------------------------------------------------------------
// Other actions

export async function setSuspended(proc: ProcInfo, suspend: boolean) {
  try {
    await api.suspend(proc.pid, suspend);
  } catch (e) {
    toast("error", `${proc.name} konnte nicht ${suspend ? "angehalten" : "fortgesetzt"} werden`, denied(e));
  }
  refresh();
}

export async function setPriority(proc: ProcInfo, priorityClass: number) {
  try {
    await api.setPriority(proc.pid, priorityClass);
    toast("success", "Priorität geändert", `${proc.name}: ${PRIORITIES.find((p) => p.value === priorityClass)?.label}`);
    return true;
  } catch (e) {
    toast("error", "Priorität konnte nicht geändert werden", denied(e));
    return false;
  }
}

export async function showInExplorer(path: string) {
  try {
    await revealItemInDir(path);
  } catch (e) {
    toastError("Explorer konnte nicht geöffnet werden", e);
  }
}

export async function showProperties(path: string) {
  try {
    await api.showProperties(path);
  } catch (e) {
    toastError("Eigenschaften konnten nicht geöffnet werden", e);
  }
}

export async function searchOnline(name: string) {
  try {
    await openUrl(`https://www.bing.com/search?q=${encodeURIComponent(name)}`);
  } catch (e) {
    toastError("Browser konnte nicht geöffnet werden", e);
  }
}

export async function copy(text: string, what: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast("success", `${what} kopiert`, text.length > 200 ? `${text.slice(0, 200)} …` : text);
  } catch (e) {
    toastError("Kopieren fehlgeschlagen", e);
  }
}

export async function restartAsAdmin() {
  try {
    await api.restartAsAdmin();
  } catch (e) {
    toastError("Neustart als Administrator fehlgeschlagen", e);
  }
}
