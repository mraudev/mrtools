import { reactive, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { state } from "./store";
import type { IndexStatus, SearchHit } from "./types";

/** Hits shown at most – the total is still counted. */
export const LIMIT = 500;

/** Search over all local drives (see `src-tauri/src/search.rs`); the query is `state.filter`. */
export const search = reactive({
  /** The search box searches everywhere instead of filtering the current folder. */
  everywhere: false,
  hits: [] as SearchHit[],
  total: 0,
  elapsedMs: 0,
  loading: false,
  status: { entries: 0, builtAt: 0, building: false, scanned: 0 } as IndexStatus,
});

export const searching = () => search.everywhere && state.filter.trim() !== "";

let token = 0;
let timer: ReturnType<typeof setTimeout> | undefined;

async function run() {
  const query = state.filter.trim();
  const mine = ++token;
  if (!search.everywhere || !query) {
    search.hits = [];
    search.total = 0;
    search.loading = false;
    return;
  }
  search.loading = true;
  try {
    const results = await api.search(query, LIMIT);
    if (mine !== token) return;
    search.hits = results.hits;
    search.total = results.total;
    search.elapsedMs = results.elapsedMs;
  } catch (e) {
    if (mine === token) toastError("Suche fehlgeschlagen", e);
  } finally {
    if (mine === token) search.loading = false;
  }
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
  // Opening a folder (also from the results) goes back to filtering that folder.
  watch(
    () => state.path,
    () => (search.everywhere = false),
  );
  await listen<IndexStatus>("index-status", ({ payload }) => {
    const finished = search.status.building && !payload.building;
    search.status = payload;
    // Results from the old (or a missing) index are refreshed with the new one.
    if (finished || (payload.entries && !search.total)) run();
  });
  search.status = await api.searchStatus();
}
