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
  /** Key of the pull request being prepared for a Claude review. */
  reviewing: "",
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

/** Loads the pull request and opens Claude with a prefilled review prompt. */
export async function reviewWithClaude(pr: DashboardPull) {
  const { giteaHost, reviewTarget } = store.config.settings;
  const projectPaths = [...store.config.projects, ...store.watched].map((p) => p.path);
  dashboard.reviewing = pullKey(pr);
  try {
    await api.reviewWithClaude(pr, giteaHost, projectPaths, reviewTarget);
    if (reviewTarget === "desktop") {
      toast("success", "Review in Claude Desktop vorbereitet", "Auftrag prüfen und selbst absenden.");
    } else {
      toast("success", "Review in Claude Code gestartet", "Läuft im Auto-Modus im neuen Terminalfenster.");
    }
  } catch (e) {
    toastError("Review konnte nicht vorbereitet werden", e);
  } finally {
    dashboard.reviewing = "";
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
