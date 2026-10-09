import { openUrl } from "@tauri-apps/plugin-opener";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { loadInstalled, state } from "./store";
import type { App } from "./types";

export async function launchApp(app: App) {
  try {
    await api.launch(app.name);
    toast("success", `${app.name} gestartet`);
  } catch (e) {
    toastError(`${app.name} ließ sich nicht starten`, e);
  }
}

/** Installs or updates `app` from its latest release. */
export async function installApp(app: App, update: boolean) {
  if (state.installing.has(app.folder)) return;
  state.installing.add(app.folder);
  try {
    await api.install(app.folder);
    await loadInstalled();
    toast("success", `${app.name} ${update ? "aktualisiert" : "installiert"}`);
  } catch (e) {
    toastError(`${app.name} ließ sich nicht ${update ? "aktualisieren" : "installieren"}`, e);
  } finally {
    state.installing.delete(app.folder);
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
