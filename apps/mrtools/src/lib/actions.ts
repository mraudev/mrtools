import { reactive } from "vue";
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

export async function revealApp(app: App) {
  try {
    await api.reveal(app.name);
  } catch (e) {
    toastError("Ordner ließ sich nicht öffnen", e);
  }
}

/** The app waiting for the user to confirm its uninstallation. */
export const confirmUninstall = reactive({ open: false, app: null as App | null });

export function askUninstall(app: App) {
  confirmUninstall.app = app;
  confirmUninstall.open = true;
}

export async function uninstallApp() {
  const app = confirmUninstall.app;
  confirmUninstall.open = false;
  if (!app || state.uninstalling.has(app.folder)) return;
  state.uninstalling.add(app.folder);
  try {
    await api.uninstall(app.name);
    await loadInstalled();
    toast("success", `${app.name} deinstalliert`);
  } catch (e) {
    toastError(`${app.name} ließ sich nicht deinstallieren`, e);
  } finally {
    state.uninstalling.delete(app.folder);
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
