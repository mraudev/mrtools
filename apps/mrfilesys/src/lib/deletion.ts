import { reactive } from "vue";
import { listen } from "@tauri-apps/api/event";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { formatCount } from "./format";
import { baseName } from "./paths";
import { refresh } from "./store";
import type { DeleteDone, DeleteProgress } from "./types";

export interface DeleteJob {
  id: number;
  /** "„name“" or "3 Elemente". */
  label: string;
  deleted: number;
  failed: number;
  current: string;
}

/** Running permanent deletions (done in Rust, see `src-tauri/src/delete.rs`). */
export const deletions = reactive(new Map<number, DeleteJob>());

/** Done events that arrived before `deletePermanently` returned the id (small deletions). */
const earlyDone = new Map<number, DeleteDone>();

function seconds(ms: number) {
  return `${(ms / 1000).toLocaleString("de-DE", { maximumFractionDigits: 1 })} s`;
}

function finish(done: DeleteDone) {
  deletions.delete(done.id);
  refresh({ quiet: true });
  const count = `${formatCount(done.deleted)} ${done.deleted === 1 ? "Element" : "Elemente"}`;
  if (done.cancelled) {
    toast("info", "Löschen abgebrochen", `${count} wurden bereits gelöscht.`);
  } else if (done.failed) {
    const first = done.failures[0];
    const more = done.failed > 1 ? ` (und ${formatCount(done.failed - 1)} weitere)` : "";
    toast(
      "error",
      `${formatCount(done.failed)} ${done.failed === 1 ? "Element konnte" : "Elemente konnten"} nicht gelöscht werden`,
      first ? `${first.path}: ${first.message}${more}` : undefined,
    );
  } else {
    toast("success", `${count} endgültig gelöscht`, `in ${seconds(done.elapsedMs)}`);
  }
}

export async function deletePermanently(paths: string[]) {
  let id: number;
  try {
    id = await api.deletePermanently(paths);
  } catch (e) {
    return toastError("Löschen fehlgeschlagen", e);
  }
  const done = earlyDone.get(id);
  earlyDone.delete(id);
  if (done) return finish(done);
  const label = paths.length === 1 ? `„${baseName(paths[0])}“` : `${formatCount(paths.length)} Elemente`;
  deletions.set(id, { id, label, deleted: 0, failed: 0, current: "" });
}

export function cancelDeletion(id: number) {
  api.cancelDelete(id).catch((e) => toastError("Abbrechen fehlgeschlagen", e));
}

export async function initDeletion() {
  await listen<DeleteProgress>("delete-progress", ({ payload }) => {
    const job = deletions.get(payload.id);
    if (job) Object.assign(job, { deleted: payload.deleted, failed: payload.failed, current: payload.current });
  });
  await listen<DeleteDone>("delete-done", ({ payload }) => {
    if (deletions.has(payload.id)) finish(payload);
    else earlyDone.set(payload.id, payload);
  });
}
