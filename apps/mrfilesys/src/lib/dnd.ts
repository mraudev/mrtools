import { reactive } from "vue";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { dropOp, type DropOp } from "./drop";
import { samePlace } from "./favorites";
import { startTransfer } from "./transfer";
import { pinFavorites, refresh, selectedEntries, selectOnly, settings, state } from "./store";

/**
 * Drag and drop. Inside the window it is done with pointer events (Tauri's file drop and HTML5
 * drag and drop exclude each other on Windows). When the pointer leaves the window, the drag is
 * handed to Windows, so files can be dropped in Explorer. Files dragged in from outside arrive
 * through Tauri's drag-drop events.
 *
 * Drop targets are elements with `data-drop="<folder path>"`. Inside `data-pin-zone` (the favorites in
 * the sidebar) the gaps between the items marked `data-pin-index` pin to the favorites instead.
 */
export const drag = reactive({
  /** A drag is in progress (started here or coming from outside). */
  active: false,
  external: false,
  paths: [] as string[],
  x: 0,
  y: 0,
  /** Folder under the pointer. */
  target: "",
  op: null as DropOp | null,
  /** Insert position in the favorites, -1 if not pinning. */
  pin: -1,
  /** A favorite is dragged within the sidebar – its whole row is an insert position, not a folder. */
  reorder: false,
});

/** Insert position in the favorites under the pointer, -1 outside of them or in the middle of a folder. */
function pinIndexAt(el: HTMLElement | null, y: number): number {
  const zone = el?.closest<HTMLElement>("[data-pin-zone]");
  if (!el || !zone) return -1;
  const item = el.closest<HTMLElement>("[data-pin-index]");
  if (item) {
    const index = Number(item.dataset.pinIndex);
    const rect = item.getBoundingClientRect();
    const rel = (y - rect.top) / rect.height;
    // The middle of a folder means "into this folder", like in Explorer's navigation pane.
    if (!drag.reorder && item.dataset.drop && rel > 0.25 && rel < 0.75) return -1;
    return rel < 0.5 ? index : index + 1;
  }
  const first = zone.querySelector("[data-pin-index]");
  return first && y < first.getBoundingClientRect().top ? 0 : settings.favorites.length;
}

function update(x: number, y: number, keys: { ctrl?: boolean; shift?: boolean } = {}) {
  drag.x = x;
  drag.y = y;
  const el = document.elementFromPoint(x, y) as HTMLElement | null;
  const pin = pinIndexAt(el, y);
  if (pin >= 0) {
    drag.pin = samePlace(settings.favorites, drag.paths, pin) ? -1 : pin;
    drag.target = "";
    drag.op = null;
    return;
  }
  drag.pin = -1;
  drag.target = el?.closest<HTMLElement>("[data-drop]")?.dataset.drop ?? "";
  drag.op = dropOp(drag.paths, drag.target, keys);
}

function reset() {
  drag.active = false;
  drag.external = false;
  drag.paths = [];
  drag.target = "";
  drag.op = null;
  drag.pin = -1;
  drag.reorder = false;
}

function finish(paths: string[], op: DropOp | null, target: string, pin: number) {
  if (pin >= 0) pinFavorites(paths, pin);
  else if (op && target) startTransfer(op, paths, target);
}


/** Scrolls the file list while dragging near its top or bottom edge. */
function autoScroll(x: number, y: number) {
  const list = document.getElementById("file-list");
  if (!list) return;
  const rect = list.getBoundingClientRect();
  if (x < rect.left || x > rect.right) return;
  if (y < rect.top + 32 && y >= rect.top - 8) list.scrollTop -= 16;
  else if (y > rect.bottom - 32 && y <= rect.bottom + 8) list.scrollTop += 16;
}

/** Call on pointerdown on an entry of the file list; the drag starts once the pointer has moved a few pixels. */
export function beginDrag(down: PointerEvent, path: string) {
  startDrag(down, () => {
    if (!state.selected.has(path)) selectOnly(path);
    return selectedEntries.value.map((entry) => entry.path);
  });
}

/** Call on pointerdown on a favorite in the sidebar. */
export function beginFavoriteDrag(down: PointerEvent, path: string) {
  startDrag(down, () => [path], true);
}

function startDrag(down: PointerEvent, pick: () => string[], reorder = false) {
  if (down.button !== 0 || (down.target as HTMLElement).closest("input")) return;
  const row = down.currentTarget as HTMLElement;
  let started = false;

  const outside = (e: PointerEvent) =>
    e.clientX < 0 || e.clientY < 0 || e.clientX >= window.innerWidth || e.clientY >= window.innerHeight;

  const onMove = (e: PointerEvent) => {
    if (!started) {
      if (Math.hypot(e.clientX - down.clientX, e.clientY - down.clientY) < 5) return;
      started = true;
      drag.paths = pick();
      drag.reorder = reorder;
      drag.active = true;
      row.setPointerCapture(e.pointerId);
    }
    if (outside(e)) {
      // Left the window: Windows takes over while the button is still down.
      const paths = [...drag.paths];
      stop();
      api
        .dragOut(paths)
        .catch((err) => toastError("Ziehen nicht möglich", err))
        .finally(() => refresh({ quiet: true }));
      return;
    }
    update(e.clientX, e.clientY, { ctrl: e.ctrlKey, shift: e.shiftKey });
    autoScroll(e.clientX, e.clientY);
  };

  const swallow = (c: Event) => c.stopPropagation();

  const onUp = () => {
    const { op, paths, target, pin } = drag;
    if (started) {
      // The click that follows the release must not change the selection.
      window.addEventListener("click", swallow, { capture: true, once: true });
      setTimeout(() => window.removeEventListener("click", swallow, { capture: true }), 0);
    }
    stop();
    if (started) finish([...paths], op, target, pin);
  };

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      stop();
    } else if (drag.active && (e.key === "Control" || e.key === "Shift")) {
      update(drag.x, drag.y, { ctrl: e.ctrlKey, shift: e.shiftKey });
    }
  };

  function stop() {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("keydown", onKey, { capture: true });
    window.removeEventListener("keyup", onKey, { capture: true });
    if (row.hasPointerCapture(down.pointerId)) row.releasePointerCapture(down.pointerId);
    reset();
  }

  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("keydown", onKey, { capture: true });
  window.addEventListener("keyup", onKey, { capture: true });
}

/** Files dragged in from Explorer, the desktop or other apps (or dragged out and back in). */
export async function initExternalDrop() {
  await getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === "leave") return reset();
    const x = payload.position.x / window.devicePixelRatio;
    const y = payload.position.y / window.devicePixelRatio;
    if (payload.type === "enter") {
      drag.active = true;
      drag.external = true;
      drag.paths = payload.paths;
      update(x, y);
    } else if (payload.type === "over") {
      update(x, y);
    } else if (payload.type === "drop") {
      drag.paths = payload.paths;
      update(x, y);
      const { op, target, pin } = drag;
      reset();
      finish([...payload.paths], op, target, pin);
    }
  });
}
