import { reactive } from "vue";

export interface Toast {
  id: number;
  kind: "success" | "error" | "info";
  title: string;
  detail?: string;
}

export const toasts = reactive<Toast[]>([]);
let nextId = 0;

export function dismissToast(id: number) {
  const index = toasts.findIndex((t) => t.id === id);
  if (index >= 0) toasts.splice(index, 1);
}

export function toast(kind: Toast["kind"], title: string, detail?: string) {
  const id = ++nextId;
  toasts.push({ id, kind, title, detail });
  setTimeout(() => dismissToast(id), kind === "error" ? 8000 : 3500);
}

export function toastError(title: string, error: unknown) {
  toast("error", title, error instanceof Error ? error.message : String(error));
}
