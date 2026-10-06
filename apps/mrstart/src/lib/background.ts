// Periodic background work: optional git fetch (off by default) and the
// dashboard refresh that drives review notifications.
import { watch } from "vue";
import { api } from "./api";
import { loadDashboard } from "./dashboard";
import { refresh, store } from "./store";

const FIVE_MINUTES = 5 * 60_000;

let fetchTimer: ReturnType<typeof setInterval> | undefined;

async function fetchAll() {
  const paths = [...store.config.projects, ...store.watched].map((p) => p.path);
  if (paths.length === 0) return;
  // Failures (offline, credentials) are expected in the background and not shown.
  await api.gitFetchAll(paths).catch(() => []);
  refresh();
}

export function startBackgroundWork() {
  watch(
    () => store.config.settings.autoFetchMinutes,
    (minutes) => {
      clearInterval(fetchTimer);
      if (minutes > 0) fetchTimer = setInterval(fetchAll, minutes * 60_000);
    },
    { immediate: true },
  );
  // Keeps the review counter current and notices new review requests.
  setInterval(loadDashboard, FIVE_MINUTES);
}
