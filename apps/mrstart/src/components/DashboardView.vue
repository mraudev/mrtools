<script setup lang="ts">
import { computed, onMounted } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from "reka-ui";
import {
  ChevronDown,
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
import Tip from "./ui/Tip.vue";
import { dashboard, loadDashboard, pullKey, reviewWithClaude, updateBranch } from "@/lib/dashboard";
import { openInBrowser } from "@/lib/pulls";
import { store } from "@/lib/store";
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

const relative = new Intl.RelativeTimeFormat("de", { numeric: "auto" });
function ago(iso: string): string {
  const seconds = (Date.parse(iso) - Date.now()) / 1000;
  if (Number.isNaN(seconds)) return "";
  const steps: [Intl.RelativeTimeFormatUnit, number][] = [
    ["year", 31536000],
    ["month", 2592000],
    ["week", 604800],
    ["day", 86400],
    ["hour", 3600],
    ["minute", 60],
  ];
  for (const [unit, size] of steps) {
    if (Math.abs(seconds) >= size) return relative.format(Math.round(seconds / size), unit);
  }
  return "gerade eben";
}

const reviewTip = computed(
  () =>
    `Öffnet ${store.config.settings.reviewTarget === "desktop" ? "Claude Desktop" : "Claude Code im Terminal"} ` +
    "mit einem vorbereiteten Review-Auftrag (Titel, Beschreibung und Diff).\nDer Auftrag wird erst gesendet, wenn du ihn bestätigst.",
);

const time = (date: Date) => date.toLocaleTimeString("de", { hour: "2-digit", minute: "2-digit" });
const provider = (pr: DashboardPull) => (pr.provider === "gitea" ? "Gitea" : "GitHub");
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

      <section class="rounded-xl border border-border bg-card">
        <h2 class="flex items-center gap-2 border-b border-border px-4 py-3 font-semibold">
          <GitPullRequest class="size-4 text-accent-text" />
          Meine offenen Pull Requests
          <span class="rounded bg-foreground/8 px-1.5 text-xs font-normal tabular-nums">{{ dashboard.authored.length }}</span>
        </h2>
        <ul v-if="dashboard.authored.length" class="divide-y divide-border">
          <li v-for="pr in dashboard.authored" :key="pullKey(pr)" class="flex items-center gap-3 px-4 py-2.5">
            <div class="min-w-0 flex-1">
              <button class="block max-w-full truncate text-left font-medium hover:text-accent-text hover:underline" @click="openInBrowser(pr.url)">
                {{ pr.title }}
              </button>
              <p class="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
                <span class="rounded border border-border px-1">{{ provider(pr) }}</span>
                <span>{{ pr.owner }}/{{ pr.repo }} #{{ pr.number }}</span>
                <span v-if="pr.head" class="font-mono">{{ pr.head }} → {{ pr.base }}</span>
                <span>{{ ago(pr.updatedAt) }}</span>
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

            <DropdownMenuRoot v-if="pr.canUpdate">
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
            :class="dashboard.reviewRequests.length ? 'bg-accent font-semibold text-accent-foreground' : 'bg-foreground/8'"
          >
            {{ dashboard.reviewRequests.length }}
          </span>
        </h2>
        <ul v-if="dashboard.reviewRequests.length" class="divide-y divide-border">
          <li v-for="pr in dashboard.reviewRequests" :key="pullKey(pr)" class="flex items-center gap-3 px-4 py-2.5">
            <div class="min-w-0 flex-1">
              <button class="block max-w-full truncate text-left font-medium hover:text-accent-text hover:underline" @click="openInBrowser(pr.url)">
                {{ pr.title }}
              </button>
              <p class="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
                <span class="rounded border border-border px-1">{{ provider(pr) }}</span>
                <span>{{ pr.owner }}/{{ pr.repo }} #{{ pr.number }}</span>
                <span v-if="pr.author">von {{ pr.author }}</span>
                <span>{{ ago(pr.updatedAt) }}</span>
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
    </template>
  </div>
</template>
