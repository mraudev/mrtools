import { reactive } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "./api";
import { store } from "./store";
import { toastError } from "@mrtools/ui/lib/toast";
import type { PullRequestResult } from "./types";

/** Open pull requests of the projects in the current tab, shown on their tiles. */
export const pullState = reactive<PullRequestResult>({ pulls: [], errors: [] });

let request = 0;

export async function loadPulls(paths: string[]) {
  const current = ++request;
  const result = paths.length
    ? await api
        .pullRequests({ paths, giteaHost: store.config.settings.giteaHost })
        .catch((e) => ({ pulls: [], errors: [{ paths, message: String(e) }] }))
    : { pulls: [], errors: [] };
  // Ignore responses of requests that were superseded by a tab switch.
  if (current !== request) return;
  pullState.pulls = result.pulls;
  pullState.errors = result.errors;
}

const samePath = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

export function pullFor(path: string) {
  return pullState.pulls.find((pr) => pr.paths.some((p) => samePath(p, path)));
}

export function pullErrorFor(path: string) {
  return pullState.errors.find((e) => e.paths.some((p) => samePath(p, path)))?.message;
}

export function openInBrowser(url: string) {
  openUrl(url).catch((e) => toastError("Link konnte nicht geöffnet werden", e));
}
