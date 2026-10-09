import { reactive } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { toast, toastError } from "@mrtools/ui/lib/toast";

export const updater = reactive({
  status: "idle" as "idle" | "checking" | "available" | "current" | "error" | "downloading" | "installing",
  version: "",
  notes: "",
  error: "",
  downloaded: 0,
  total: 0,
  dialogOpen: false,
});

// Kept outside the reactive state: the Update object is a class with a native resource id.
let pending: Update | null = null;

export async function checkForUpdate(manual = false) {
  if (updater.status === "checking" || updater.status === "downloading" || updater.status === "installing") {
    return;
  }
  updater.status = "checking";
  try {
    pending = await check();
    if (pending) {
      updater.status = "available";
      updater.version = pending.version;
      updater.notes = pending.body ?? "";
      if (manual) updater.dialogOpen = true;
    } else {
      updater.status = "current";
      if (manual) toast("success", "mrstart ist auf dem neuesten Stand");
    }
  } catch (e) {
    updater.status = "error";
    updater.error = String(e);
    if (manual) toastError("Update-Prüfung fehlgeschlagen", e);
  }
}

export async function installUpdate() {
  if (!pending) return;
  updater.status = "downloading";
  updater.downloaded = 0;
  updater.total = 0;
  try {
    await pending.downloadAndInstall((event) => {
      if (event.event === "Started") updater.total = event.data.contentLength ?? 0;
      else if (event.event === "Progress") updater.downloaded += event.data.chunkLength;
      else updater.status = "installing";
    });
    // On Windows the installer quits the app itself; elsewhere restart explicitly.
    await relaunch();
  } catch (e) {
    updater.status = "available";
    toastError("Update fehlgeschlagen", e);
  }
}

const SIX_HOURS = 6 * 60 * 60 * 1000;

/** Checks at startup and periodically, since the launcher usually stays open all day. */
export function startUpdateChecks() {
  checkForUpdate();
  setInterval(() => {
    if (updater.status !== "available") checkForUpdate();
  }, SIX_HOURS);
}
