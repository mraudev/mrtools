import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "./api";
import { setRoot, state } from "./store";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import type { App } from "./types";

export async function launchApp(app: App) {
  try {
    await api.launch(app.name);
    toast("success", `${app.name} gestartet`);
  } catch (e) {
    toastError(`${app.name} ließ sich nicht starten`, e);
  }
}

export async function openFolder(path: string) {
  try {
    await api.openFolder(path);
  } catch (e) {
    toastError("Ordner ließ sich nicht öffnen", e);
  }
}

export async function openGithub(url: string) {
  if (!url.startsWith("https://github.com/")) return;
  try {
    await openUrl(url);
  } catch (e) {
    toastError("Link ließ sich nicht öffnen", e);
  }
}

export async function chooseRoot() {
  const folder = await openDialog({ directory: true, defaultPath: state.root, title: "Ordner mit den Apps wählen" });
  if (typeof folder === "string") await setRoot(folder);
}
