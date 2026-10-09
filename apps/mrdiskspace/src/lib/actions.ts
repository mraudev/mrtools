import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast, toastError } from "@mrtools/ui/lib/toast";

export async function showInExplorer(path: string) {
  try {
    await revealItemInDir(path);
  } catch (e) {
    toastError("Explorer konnte nicht geöffnet werden", e);
  }
}

export async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    toast("success", "Pfad kopiert", path);
  } catch (e) {
    toastError("Kopieren fehlgeschlagen", e);
  }
}
