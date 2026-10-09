import { reactive, shallowRef } from "vue";
import { listen } from "@tauri-apps/api/event";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { formatBytes, formatCount } from "./format";
import { baseName } from "./paths";
import { refresh } from "./store";
import type { ConflictChoice, TransferDone, TransferOp, TransferProgress } from "./types";

export interface TransferJob {
  id: number;
  op: TransferOp;
  /** "„name“" or "3 Elemente". */
  label: string;
  target: string;
  files: number;
  bytes: number;
  /** 0 while still being counted. */
  totalFiles: number;
  totalBytes: number;
  failed: number;
  current: string;
  onSuccess?: () => void;
}

/** Running copies and moves (done in Rust, see `src-tauri/src/transfer.rs`). */
export const transfers = reactive(new Map<number, TransferJob>());

export interface ConflictQuestion {
  op: TransferOp;
  target: string;
  names: string[];
  answer: (choice: ConflictChoice | null) => void;
}

/** Shown by `ConflictDialog` while names exist in the target already. */
export const conflictQuestion = shallowRef<ConflictQuestion | null>(null);

const earlyDone = new Map<number, TransferDone>();

function seconds(ms: number) {
  return `${(ms / 1000).toLocaleString("de-DE", { maximumFractionDigits: 1 })} s`;
}

function finish(done: TransferDone) {
  const job = transfers.get(done.id);
  transfers.delete(done.id);
  refresh({ quiet: true });
  const verb = job?.op === "move" ? "verschoben" : "kopiert";
  if (done.cancelled) {
    toast("info", job?.op === "move" ? "Verschieben abgebrochen" : "Kopieren abgebrochen", `${formatCount(done.files)} Dateien wurden bereits ${verb}.`);
  } else if (done.failed) {
    const first = done.failures[0];
    const more = done.failed > 1 ? ` (und ${formatCount(done.failed - 1)} weitere)` : "";
    toast(
      "error",
      `${formatCount(done.failed)} ${done.failed === 1 ? "Element konnte" : "Elemente konnten"} nicht ${verb} werden`,
      first ? `${first.path}: ${first.message}${more}` : undefined,
    );
  } else {
    job?.onSuccess?.();
    const skipped = done.skipped ? ` · ${formatCount(done.skipped)} übersprungen` : "";
    const detail = done.bytes ? `${formatCount(done.files)} Dateien, ${formatBytes(done.bytes)} in ${seconds(done.elapsedMs)}` : "";
    toast("success", `${job?.label ?? "Elemente"} ${verb}`, detail + skipped || undefined);
  }
}

function ask(op: TransferOp, target: string, names: string[]): Promise<ConflictChoice | null> {
  return new Promise((resolve) => {
    conflictQuestion.value = {
      op,
      target,
      names,
      answer: (choice) => {
        conflictQuestion.value = null;
        resolve(choice);
      },
    };
  });
}

/**
 * Copies or moves `paths` into `target`. Asks once what to do if names exist there already.
 * `onSuccess` runs when everything went through (e.g. clearing the clipboard after cut & paste).
 */
export async function startTransfer(op: TransferOp, paths: string[], target: string, onSuccess?: () => void) {
  let choice: ConflictChoice = "keepBoth";
  try {
    const names = await api.checkConflicts(paths, target);
    if (names.length) {
      const answer = await ask(op, target, names);
      if (!answer) return;
      choice = answer;
    }
    const id = await api.startTransfer(op, paths, target, choice);
    const label = paths.length === 1 ? `„${baseName(paths[0])}“` : `${formatCount(paths.length)} Elemente`;
    transfers.set(id, {
      id, op, label, target, onSuccess,
      files: 0, bytes: 0, totalFiles: 0, totalBytes: 0, failed: 0, current: "",
    });
    const done = earlyDone.get(id);
    earlyDone.delete(id);
    if (done) finish(done);
  } catch (e) {
    toastError(op === "move" ? "Verschieben fehlgeschlagen" : "Kopieren fehlgeschlagen", e);
  }
}

export function cancelTransfer(id: number) {
  api.cancelTransfer(id).catch((e) => toastError("Abbrechen fehlgeschlagen", e));
}

export async function initTransfer() {
  await listen<TransferProgress>("transfer-progress", ({ payload }) => {
    const job = transfers.get(payload.id);
    if (job) Object.assign(job, { ...payload, id: job.id });
  });
  await listen<TransferDone>("transfer-done", ({ payload }) => {
    if (transfers.has(payload.id)) finish(payload);
    else earlyDone.set(payload.id, payload);
  });
}
