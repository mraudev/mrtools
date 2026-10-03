import { reactive } from "vue";
import { api } from "./api";
import { store } from "./store";
import { toast, toastError } from "./toast";
import type { DashboardPull } from "./types";

export const dashboard = reactive({
  /** `false` if neither a Gitea host+token nor a GitHub token is set up. */
  configured: true,
  loading: false,
  authored: [] as DashboardPull[],
  reviewRequests: [] as DashboardPull[],
  errors: [] as string[],
  loadedAt: null as Date | null,
  /** Key of the pull request whose branch is being updated. */
  updating: "",
});

export const pullKey = (pr: DashboardPull) => `${pr.provider}:${pr.owner}/${pr.repo}#${pr.number}`;

let request = 0;

export async function loadDashboard() {
  const current = ++request;
  const tokens = await api.secretStatus().catch(() => ({ github: false, gitea: false }));
  const giteaHost = store.config.settings.giteaHost;
  dashboard.configured = tokens.github || (tokens.gitea && giteaHost.trim() !== "");
  if (!dashboard.configured) {
    Object.assign(dashboard, { authored: [], reviewRequests: [], errors: [], loadedAt: null });
    return;
  }

  dashboard.loading = true;
  try {
    const result = await api.dashboard(giteaHost);
    if (current !== request) return;
    Object.assign(dashboard, { ...result, loadedAt: new Date() });
  } catch (e) {
    if (current === request) dashboard.errors = [String(e)];
  } finally {
    if (current === request) dashboard.loading = false;
  }
}

export async function updateBranch(pr: DashboardPull, rebase: boolean) {
  dashboard.updating = pullKey(pr);
  try {
    await api.updatePullBranch(pr, rebase, store.config.settings.giteaHost);
    toast("success", `Branch ${pr.head} aktualisiert`, `${pr.owner}/${pr.repo} #${pr.number} per ${rebase ? "Rebase" : "Merge"}`);
    // The server needs a moment to recompute the merge status.
    setTimeout(loadDashboard, 2000);
  } catch (e) {
    toastError("Aktualisieren fehlgeschlagen", e);
  } finally {
    dashboard.updating = "";
  }
}
