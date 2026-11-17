import { reactive, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { state } from "./store";
import type { IndexStatus, SearchHit } from "./types";

/** Hits shown at most – the total is still counted. */
export const LIMIT = 500;

/** Search below the current folder or over all local drives (see `src-tauri/src/search.rs`); the query is `state.filter`. */
export const search = reactive({
  /** The search box searches all drives instead of the current folder and its subfolders. */
  everywhere: false,
  hits: [] as SearchHit[],
  total: 0,
  elapsedMs: 0,
  loading: false,
  /** The folder is being read for a new search there. */
  reading: false,
  /** Hits are from an older reading of the folder; a fresh one runs in the background. */
  refreshing: false,
  status: { entries: 0, builtAt: 0, building: false, scanned: 0, watching: false } as IndexStatus,
});

/** Without a folder (the drive overview) the search box only filters the list. */
export const searching = () => (search.everywhere || state.path !== "") && state.filter.trim() !== "";

let token = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
/** A folder search reads the folder once when it starts and reuses that while the query is refined. */
let fresh = true;

async function run() {
  const query = state.filter.trim();
  const mine = ++token;
  if (!searching()) {
    search.hits = [];
    search.total = 0;
    search.loading = false;
    return;
  }
  const within = search.everywhere ? null : state.path;
  const reread = within !== null && fresh;
  fresh = false;
  search.loading = true;
  search.reading = reread;
  try {
    const results = await api.search(query, LIMIT, within, reread);
    if (mine !== token) return;
    search.hits = results.hits;
    search.total = results.total;
    search.elapsedMs = results.elapsedMs;
    search.refreshing = results.refreshing;
  } catch (e) {
    if (mine === token) toastError("Suche fehlgeschlagen", e);
  } finally {
    if (mine === token) search.loading = search.reading = false;
  }
}

/** Searches again, e.g. after deleting hits (they are left out then). */
export function rerunSearch() {
  if (searching()) run();
}

/** Reads the folder again (F5) – finds what was added since the search started. */
export function refreshSearch() {
  fresh = true;
  run();
}

/** Starts searching everywhere and puts the cursor into the search box. */
export function searchEverywhere() {
  search.everywhere = true;
  const input = document.getElementById("search") as HTMLInputElement | null;
  input?.select();
}

export function rebuildIndex() {
  api.rebuildIndex().catch((e) => toastError("Index konnte nicht erneuert werden", e));
}

export async function initSearch() {
  watch(
    () => [state.filter, search.everywhere] as const,
    () => {
      clearTimeout(timer);
      timer = setTimeout(run, 120);
    },
  );
  // A new search (or one in another folder) reads the folder again.
  watch(
    () => [state.filter.trim() === "", state.path, search.everywhere],
    () => (fresh = true),
  );
  // Opening a folder (also from the results) goes back to searching that folder.
  watch(
    () => state.path,
    () => (search.everywhere = false),
  );
  // The folder was read again – the hits shown came from an older reading.
  await listen("folder-read", () => search.refreshing && !search.everywhere && rerunSearch());
  await listen<IndexStatus>("index-status", ({ payload }) => {
    const finished = search.status.building && !payload.building;
    search.status = payload;
    // Results from the old (or a missing) index are refreshed with the new one.
    if (finished || (payload.entries && !search.total)) run();
  });
  search.status = await api.searchStatus();
}
