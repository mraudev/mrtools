<script setup lang="ts">
import { computed, onMounted, reactive } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from "reka-ui";
import {
  ChevronDown,
  CircleDot,
  CircleCheck,
  CircleX,
  Clock,
  GitCommitVertical,
  GitMerge,
  GitPullRequest,
  KeyRound,
  LoaderCircle,
  MessageSquareMore,
  RefreshCw,
  Sparkles,
  TriangleAlert,
} from "@lucide/vue";
import DashboardCharts from "./DashboardCharts.vue";
import Tip from "@mrtools/ui/components/Tip";
import { dashboard, loadDashboard, pullKey, reviewWithClaude, updateBranch } from "@/lib/dashboard";
import { openInBrowser } from "@/lib/pulls";
import { store } from "@/lib/store";
import { ago, dateTime, olderThan, WEEK_MS } from "@/lib/time";
import { SETTINGS, type DashboardPull, type PullStatus } from "@/lib/types";

onMounted(loadDashboard);

const statusBadge: Record<PullStatus, { label: string; tip: string; class: string }> = {
  behind: { label: "Veraltet", tip: "Der Ziel-Branch hat neuere Commits.", class: "bg-amber-500/15 text-amber-600 dark:text-amber-400" },
  conflict: { label: "Konflikte", tip: "Nicht automatisch mergebar.", class: "bg-red-500/15 text-red-600 dark:text-red-400" },
  clean: { label: "Aktuell", tip: "Auf dem Stand des Ziel-Branches und mergebar.", class: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400" },
  blocked: { label: "Blockiert", tip: "Durch Branch-Schutz blockiert (Reviews oder Checks fehlen).", class: "bg-foreground/8 text-muted-foreground" },
  unstable: { label: "Checks nicht grün", tip: "Mergebar, aber Checks sind fehlgeschlagen oder laufen noch.", class: "bg-amber-500/15 text-amber-600 dark:text-amber-400" },
  draft: { label: "Entwurf", tip: "Entwurf – noch nicht zum Mergen bereit.", class: "bg-foreground/8 text-muted-foreground" },
  unknown: { label: "Wird geprüft", tip: "Der Server berechnet den Status noch.", class: "bg-foreground/8 text-muted-foreground" },
};

const reviewBadge: Record<string, { label: string; class: string }> = {
  APPROVED: { label: "Freigegeben", class: "text-emerald-600 dark:text-emerald-400" },
  CHANGES_REQUESTED: { label: "Änderungen angefordert", class: "text-red-600 dark:text-red-400" },
  REVIEW_REQUIRED: { label: "Review ausstehend", class: "text-muted-foreground" },
};

/** No activity on the pull request for more than a week. */
const isStale = (pr: DashboardPull) => olderThan(pr.updatedAt, WEEK_MS);

// Filters scope the charts and both lists alike.
const filter = reactive({ repo: "", provider: "", onlyBehind: false, onlyStale: false });
const repoOf = (item: { owner: string; repo: string }) => `${item.owner}/${item.repo}`;
const allPulls = computed(() => [...dashboard.authored, ...dashboard.reviewRequests]);
const allItems = computed(() => [...allPulls.value, ...dashboard.issues]);
const repoOptions = computed(() => [...new Set(allItems.value.map(repoOf))].sort((a, b) => a.localeCompare(b)));
const providers = computed(() => [...new Set(allItems.value.map((p) => p.provider))]);
const filterActive = computed(() => !!(filter.repo || filter.provider || filter.onlyBehind || filter.onlyStale));

function matches(pr: DashboardPull) {
  return (
    (!filter.repo || repoOf(pr) === filter.repo) &&
    (!filter.provider || pr.provider === filter.provider) &&
    (!filter.onlyBehind || pr.canUpdate || pr.status === "behind") &&
    (!filter.onlyStale || isStale(pr))
  );
}
const authored = computed(() => dashboard.authored.filter(matches));
const reviews = computed(() => dashboard.reviewRequests.filter(matches));
// "Nur veraltete" concerns branches, so it does not apply to issues.
const issues = computed(() =>
  dashboard.issues.filter(
    (issue) =>
      (!filter.repo || repoOf(issue) === filter.repo) &&
      (!filter.provider || issue.provider === filter.provider) &&
      (!filter.onlyStale || olderThan(issue.updatedAt, WEEK_MS)),
  ),
);

function resetFilter() {
  Object.assign(filter, { repo: "", provider: "", onlyBehind: false, onlyStale: false });
}

const ciBadge = {
  success: { label: "CI grün", icon: CircleCheck, color: "var(--status-good)" },
  failure: { label: "CI rot", icon: CircleX, color: "var(--status-critical)" },
  pending: { label: "CI läuft", icon: Clock, color: "var(--status-warning)" },
} as const;

const reviewTip = computed(
  () =>
    store.config.settings.reviewTarget === "desktop"
      ? "Öffnet Claude Desktop mit einem vorbereiteten Review-Auftrag (Titel, Beschreibung und Diff).\nDer Auftrag wird erst gesendet, wenn du ihn bestätigst."
      : "Startet Claude Code im Terminal im Auto-Modus und schickt den Review-Auftrag (Titel, Beschreibung und Diff) sofort ab.",
);

const time = (date: Date) => date.toLocaleTimeString("de", { hour: "2-digit", minute: "2-digit" });
const provider = (item: { provider: string }) => (item.provider === "gitea" ? "Gitea" : "GitHub");

function baseTip(pr: DashboardPull): string {
  const base = pr.base || "dem Ziel-Branch";
  return (
    `Der Branch enthält ${base} mit Stand vom ${dateTime(pr.baseDate ?? "")}.\n` +
    `So alt ist die Basis, auf der er aufbaut – nach dem Aktualisieren (Merge/Rebase von ${base}) ist sie wieder aktuell.`
  );
}
</script>

<template>
  <div class="mx-auto max-w-5xl space-y-4 p-5">
    <header class="flex items-center gap-3">
      <h1 class="text-lg font-semibold">Pull Requests</h1>
      <span v-if="dashboard.loadedAt" class="text-xs text-muted-foreground">Stand {{ time(dashboard.loadedAt) }}</span>
      <div class="flex-1" />
      <button class="btn btn-outline" :disabled="dashboard.loading || !dashboard.configured" @click="loadDashboard">
        <LoaderCircle v-if="dashboard.loading" class="animate-spin" />
        <RefreshCw v-else />
        Aktualisieren
      </button>
    </header>

    <div v-if="!dashboard.configured" class="rounded-xl border border-border bg-card p-6 text-center">
      <KeyRound class="mx-auto mb-3 size-8 text-accent-text" />
      <p class="font-medium">Für das Dashboard wird ein Zugang benötigt</p>
      <p class="mx-auto mt-1 max-w-lg text-muted-foreground">
        In den Einstellungen unter „Pull Requests“ den Gitea-Host und ein Gitea-Token (und/oder ein
        GitHub-Token) hinterlegen. Die Tokens werden sicher in der Windows-Anmeldeinformationsverwaltung
        gespeichert.
      </p>
      <button class="btn btn-primary mt-4" @click="store.view = SETTINGS">Zu den Einstellungen</button>
    </div>

    <template v-else>
      <div
        v-for="error in dashboard.errors"
        :key="error"
        class="flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-[13px]"
      >
        <TriangleAlert class="mt-0.5 size-4 shrink-0 text-amber-500" />
        <span class="select-text">{{ error }}</span>
      </div>

      <!-- One filter row above everything it scopes -->
      <div v-if="allItems.length" class="flex flex-wrap items-center gap-2 text-[13px]">
        <select v-model="filter.repo" class="input h-8 w-auto max-w-64 pr-7" aria-label="Repository">
          <option value="">Alle Repositories</option>
          <option v-for="r in repoOptions" :key="r" :value="r">{{ r }}</option>
        </select>
        <select v-if="providers.length > 1" v-model="filter.provider" class="input h-8 w-auto" aria-label="Anbieter">
          <option value="">Gitea und GitHub</option>
          <option value="gitea">Gitea</option>
          <option value="github">GitHub</option>
        </select>
        <button
          class="btn h-8"
          :class="filter.onlyBehind ? 'btn-primary' : 'btn-outline'"
          :aria-pressed="filter.onlyBehind"
          @click="filter.onlyBehind = !filter.onlyBehind"
        >
          Nur veraltete
        </button>
        <button
          class="btn h-8"
          :class="filter.onlyStale ? 'btn-primary' : 'btn-outline'"
          :aria-pressed="filter.onlyStale"
          @click="filter.onlyStale = !filter.onlyStale"
        >
          Nur inaktive (&gt; 1 Woche)
        </button>
        <template v-if="filterActive">
          <span class="text-xs text-muted-foreground">
            {{ authored.length + reviews.length + issues.length }} von {{ allItems.length }}
          </span>
          <button class="btn btn-ghost h-8" @click="resetFilter">Zurücksetzen</button>
        </template>
      </div>

      <DashboardCharts v-if="authored.length || reviews.length" :authored="authored" :reviews="reviews" />

      <section class="rounded-xl border border-border bg-card">
        <h2 class="flex items-center gap-2 border-b border-border px-4 py-3 font-semibold">
          <GitPullRequest class="size-4 text-accent-text" />
          Meine offenen Pull Requests
          <span class="rounded bg-foreground/8 px-1.5 text-xs font-normal tabular-nums">{{ authored.length }}</span>
        </h2>
        <ul v-if="authored.length" class="divide-y divide-border">
          <li v-for="pr in authored" :key="pullKey(pr)" class="flex items-center gap-3 px-4 py-2.5">
            <div class="min-w-0 flex-1">
              <button class="block max-w-full truncate text-left font-medium hover:text-accent-text hover:underline" @click="openInBrowser(pr.url)">
                {{ pr.title }}
              </button>
              <p class="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
                <span class="rounded border border-border px-1">{{ provider(pr) }}</span>
                <span>{{ pr.owner }}/{{ pr.repo }} #{{ pr.number }}</span>
                <span v-if="pr.head" class="font-mono">{{ pr.head }} → {{ pr.base }}</span>
                <Tip v-if="pr.ci" :text="pr.ciUrl ? `${ciBadge[pr.ci].label} – Ergebnisse im Browser öffnen` : ciBadge[pr.ci].label">
                  <button
                    class="inline-flex items-center gap-1 hover:text-foreground"
                    :disabled="!pr.ciUrl"
                    @click="pr.ciUrl && openInBrowser(pr.ciUrl)"
                  >
                    <component :is="ciBadge[pr.ci].icon" class="size-3" :style="{ color: ciBadge[pr.ci].color }" />
                    {{ ciBadge[pr.ci].label }}
                  </button>
                </Tip>
                <Tip v-if="isStale(pr)" :text="`Seit über einer Woche keine Aktivität (zuletzt am ${dateTime(pr.updatedAt)}).`">
                  <span class="inline-flex items-center gap-1 rounded bg-amber-500/15 px-1.5 font-medium text-amber-700 dark:text-amber-400">
                    <Clock class="size-3" />geändert {{ ago(pr.updatedAt) }}
                  </span>
                </Tip>
                <span v-else>geändert {{ ago(pr.updatedAt) }}</span>
                <Tip v-if="pr.baseDate" :text="baseTip(pr)">
                  <span class="inline-flex items-center gap-1">
                    <GitMerge class="size-3" />Stand von {{ pr.base || "Basis" }}: {{ ago(pr.baseDate) }}
                  </span>
                </Tip>
                <span v-if="pr.reviewDecision && reviewBadge[pr.reviewDecision]" :class="reviewBadge[pr.reviewDecision].class">
                  {{ reviewBadge[pr.reviewDecision].label }}
                </span>
              </p>
            </div>

            <Tip :text="statusBadge[pr.status].tip">
              <span class="shrink-0 rounded-md px-2 py-0.5 text-xs font-medium" :class="statusBadge[pr.status].class">
                {{ statusBadge[pr.status].label }}
              </span>
            </Tip>

            <Tip
              v-if="pr.canUpdate && pr.hasConflicts"
              :text="`${pr.head} liegt hinter ${pr.base}, lässt sich aber nicht automatisch aktualisieren: Beim Zusammenführen würde es Konflikte geben.\nDie Konflikte lokal lösen (z. B. Pull in der Kachel) und dann pushen.`"
            >
              <span
                class="inline-flex h-7 shrink-0 items-center gap-1.5 rounded-md border border-red-500/40 px-2.5 text-xs font-medium text-red-600 dark:text-red-400"
                tabindex="0"
              >
                <CircleX class="size-3.5" />Aktualisieren nur mit Konfliktlösung
              </span>
            </Tip>
            <DropdownMenuRoot v-else-if="pr.canUpdate">
              <DropdownMenuTrigger as-child>
                <button class="btn btn-primary h-7 px-2.5 text-xs" :disabled="dashboard.updating !== ''">
                  <LoaderCircle v-if="dashboard.updating === pullKey(pr)" class="animate-spin" />
                  Aktualisieren
                  <ChevronDown class="!size-3.5" />
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuPortal>
                <DropdownMenuContent
                  align="end"
                  :side-offset="4"
                  class="anim-fade z-50 w-64 rounded-lg border border-border bg-popover p-1 shadow-xl"
                >
                  <DropdownMenuItem
                    class="flex cursor-default gap-2.5 rounded-md px-2.5 py-2 outline-none data-[highlighted]:bg-foreground/8"
                    @select="updateBranch(pr, false)"
                  >
                    <GitMerge class="mt-0.5 size-4 shrink-0 text-accent-text" />
                    <span>
                      <span class="block text-sm font-medium">Per Merge aktualisieren</span>
                      <span class="block text-xs text-muted-foreground">Merge-Commit von {{ pr.base }} in {{ pr.head }}</span>
                    </span>
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    class="flex cursor-default gap-2.5 rounded-md px-2.5 py-2 outline-none data-[highlighted]:bg-foreground/8"
                    @select="updateBranch(pr, true)"
                  >
                    <GitCommitVertical class="mt-0.5 size-4 shrink-0 text-accent-text" />
                    <span>
                      <span class="block text-sm font-medium">Per Rebase aktualisieren</span>
                      <span class="block text-xs text-muted-foreground">Schreibt die Branch-Historie neu – lokale Kopien danach neu holen</span>
                    </span>
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenuPortal>
            </DropdownMenuRoot>
          </li>
        </ul>
        <p v-else class="px-4 py-6 text-center text-muted-foreground">
          {{ dashboard.loading ? "Wird geladen …" : "Keine offenen Pull Requests." }}
        </p>
      </section>

      <section class="rounded-xl border border-border bg-card">
        <h2 class="flex items-center gap-2 border-b border-border px-4 py-3 font-semibold">
          <MessageSquareMore class="size-4 text-accent-text" />
          Angeforderte Reviews
          <span
            class="rounded px-1.5 text-xs font-normal tabular-nums"
            :class="reviews.length ? 'bg-accent font-semibold text-accent-foreground' : 'bg-foreground/8'"
          >
            {{ reviews.length }}
          </span>
        </h2>
        <ul v-if="reviews.length" class="divide-y divide-border">
          <li v-for="pr in reviews" :key="pullKey(pr)" class="flex items-center gap-3 px-4 py-2.5">
            <div class="min-w-0 flex-1">
              <button class="block max-w-full truncate text-left font-medium hover:text-accent-text hover:underline" @click="openInBrowser(pr.url)">
                {{ pr.title }}
              </button>
              <p class="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
                <span class="rounded border border-border px-1">{{ provider(pr) }}</span>
                <span>{{ pr.owner }}/{{ pr.repo }} #{{ pr.number }}</span>
                <span v-if="pr.author">von {{ pr.author }}</span>
                <Tip v-if="isStale(pr)" :text="`Seit über einer Woche keine Aktivität (zuletzt am ${dateTime(pr.updatedAt)}).`">
                  <span class="inline-flex items-center gap-1 rounded bg-amber-500/15 px-1.5 font-medium text-amber-700 dark:text-amber-400">
                    <Clock class="size-3" />geändert {{ ago(pr.updatedAt) }}
                  </span>
                </Tip>
                <span v-else>geändert {{ ago(pr.updatedAt) }}</span>
                <Tip v-if="pr.baseDate" :text="baseTip(pr)">
                  <span class="inline-flex items-center gap-1">
                    <GitMerge class="size-3" />Stand von {{ pr.base || "Basis" }}: {{ ago(pr.baseDate) }}
                  </span>
                </Tip>
              </p>
            </div>
            <Tip :text="reviewTip">
              <button
                class="btn btn-outline h-7 px-2.5 text-xs"
                :disabled="dashboard.reviewing !== ''"
                @click="reviewWithClaude(pr)"
              >
                <LoaderCircle v-if="dashboard.reviewing === pullKey(pr)" class="animate-spin" />
                <Sparkles v-else class="text-accent-text" />
                Mit Claude reviewen
              </button>
            </Tip>
          </li>
        </ul>
        <p v-else class="px-4 py-6 text-center text-muted-foreground">
          {{ dashboard.loading ? "Wird geladen …" : "Keine Reviews angefordert." }}
        </p>
      </section>
      <section class="rounded-xl border border-border bg-card">
        <h2 class="flex items-center gap-2 border-b border-border px-4 py-3 font-semibold">
          <CircleDot class="size-4 text-accent-text" />
          Mir zugewiesene Issues
          <span class="rounded bg-foreground/8 px-1.5 text-xs font-normal tabular-nums">{{ issues.length }}</span>
        </h2>
        <ul v-if="issues.length" class="divide-y divide-border">
          <li v-for="issue in issues" :key="`${issue.provider}:${repoOf(issue)}#${issue.number}`" class="px-4 py-2.5">
            <button class="block max-w-full truncate text-left font-medium hover:text-accent-text hover:underline" @click="openInBrowser(issue.url)">
              {{ issue.title }}
            </button>
            <p class="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
              <span class="rounded border border-border px-1">{{ provider(issue) }}</span>
              <span>{{ issue.owner }}/{{ issue.repo }} #{{ issue.number }}</span>
              <span v-if="issue.author">von {{ issue.author }}</span>
              <span
                v-if="olderThan(issue.updatedAt, WEEK_MS)"
                class="inline-flex items-center gap-1 rounded bg-amber-500/15 px-1.5 font-medium text-amber-700 dark:text-amber-400"
              >
                <Clock class="size-3" />geändert {{ ago(issue.updatedAt) }}
              </span>
              <span v-else>geändert {{ ago(issue.updatedAt) }}</span>
              <span v-for="label in issue.labels" :key="label" class="rounded-full bg-foreground/8 px-2">{{ label }}</span>
            </p>
          </li>
        </ul>
        <p v-else class="px-4 py-6 text-center text-muted-foreground">
          {{ dashboard.loading ? "Wird geladen …" : "Keine Issues zugewiesen." }}
        </p>
      </section>
    </template>
  </div>
</template>
