import { reactive } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { toastError } from "@mrtools/ui/lib/toast";

/**
 * Automatic updates like in mrphone: check at startup and every 4 hours,
 * download a found update silently, then offer "Neu starten" – or install it
 * when mrtools is closed.
 */
export const updater = reactive({
  /** Version of the downloaded update, ready to install. */
  ready: "",
  notes: "",
  installing: false,
});

// Kept outside the reactive state: the Update object holds a native resource.
let pending: Update | null = null;
let checking = false;

async function checkForUpdate() {
  if (checking || pending) return;
  checking = true;
  try {
    const update = await check();
    if (update) {
      await update.download();
      pending = update;
      updater.ready = update.version;
      updater.notes = update.body?.trim() ?? "";
    }
  } catch (e) {
    // Offline or GitHub not reachable – try again at the next check.
    console.warn("Update-Prüfung fehlgeschlagen:", e);
  } finally {
    checking = false;
  }
}

/** Installs the downloaded update; the installer closes mrtools and starts it again. */
export async function installNow() {
  if (!pending || updater.installing) return;
  updater.installing = true;
  try {
    await pending.install();
  } catch (e) {
    updater.installing = false;
    toastError("Update fehlgeschlagen", e);
  }
}

const FOUR_HOURS = 4 * 60 * 60 * 1000;

export function startUpdateChecks() {
  // Only the installed app updates itself, not `tauri dev`.
  if (import.meta.env.DEV) return;
  checkForUpdate();
  setInterval(checkForUpdate, FOUR_HOURS);

  // Not restarted yet: install when mrtools is closed, without starting it again.
  const appWindow = getCurrentWindow();
  appWindow.onCloseRequested(async (event) => {
    if (!pending) return;
    event.preventDefault();
    try {
      await pending.install({ restartAfterInstall: false });
    } catch {
      await appWindow.destroy();
    }
  });
}
