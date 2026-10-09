<script setup lang="ts">
import { computed, type Component } from "vue";
import { CircleAlert, CircleCheck, CircleDashed, CircleX, TriangleAlert } from "@lucide/vue";
import ChartCard from "./ui/ChartCard.vue";
import Tip from "./ui/Tip.vue";
import { dashboard } from "@/lib/dashboard";
import type { DashboardPull, PullStatus } from "@/lib/types";

const props = defineProps<{ authored: DashboardPull[]; reviews: DashboardPull[] }>();
const authored = computed(() => props.authored);
const reviews = computed(() => props.reviews);

// --- Key figures ------------------------------------------------------------

const behindCount = computed(() => authored.value.filter((p) => p.status === "behind").length);
const conflictCount = computed(() => authored.value.filter((p) => p.status === "conflict").length);

// --- Status of my pull requests (part-to-whole, reserved status colors) -----

interface StatusGroup {
  label: string;
  color: string;
  icon: Component;
  states: PullStatus[];
}

const statusGroups: StatusGroup[] = [
  { label: "Aktuell", color: "var(--status-good)", icon: CircleCheck, states: ["clean"] },
  { label: "Veraltet", color: "var(--status-warning)", icon: TriangleAlert, states: ["behind"] },
  { label: "Checks nicht grün", color: "var(--status-serious)", icon: CircleAlert, states: ["unstable"] },
  { label: "Konflikte", color: "var(--status-critical)", icon: CircleX, states: ["conflict"] },
  { label: "Sonstige", color: "var(--status-neutral)", icon: CircleDashed, states: ["blocked", "draft", "unknown"] },
];

const statusRows = computed(() =>
  statusGroups.map((group) => ({
    ...group,
    count: authored.value.filter((p) => group.states.includes(p.status)).length,
  })),
);

// --- Age of open pull requests (two series, grouped columns) -----------------

const ageBuckets = [
  { label: "< 1 Tag", maxDays: 1 },
  { label: "1–3 Tage", maxDays: 3 },
  { label: "3–7 Tage", maxDays: 7 },
  { label: "1–4 Wo.", maxDays: 28 },
  { label: "> 4 Wo.", maxDays: Infinity },
];

function bucketOf(pr: DashboardPull): number {
  const opened = Date.parse(pr.createdAt || pr.updatedAt);
  const days = Number.isNaN(opened) ? 0 : (Date.now() - opened) / 86_400_000;
  return ageBuckets.findIndex((b) => days < b.maxDays);
}

const ageRows = computed(() =>
  ageBuckets.map((bucket, index) => ({
    label: bucket.label,
    mine: authored.value.filter((p) => bucketOf(p) === index).length,
    reviews: reviews.value.filter((p) => bucketOf(p) === index).length,
  })),
);

/** Clean integer ticks from 0 to a rounded maximum. */
function niceTicks(max: number): number[] {
  const top = Math.max(1, max);
  const step = top <= 5 ? 1 : top <= 10 ? 2 : top <= 25 ? 5 : 10;
  const end = Math.ceil(top / step) * step;
  return Array.from({ length: end / step + 1 }, (_, i) => i * step);
}

const ageTicks = computed(() => niceTicks(Math.max(...ageRows.value.flatMap((r) => [r.mine, r.reviews]))));
const ageTop = computed(() => ageTicks.value[ageTicks.value.length - 1]);
const PLOT_HEIGHT = 120;
const columnHeight = (value: number) => (value / ageTop.value) * PLOT_HEIGHT;

// --- Pull requests per repository (two series, stacked bars) -----------------

const MAX_REPOS = 6;

const repoRows = computed(() => {
  const rows = new Map<string, { label: string; mine: number; reviews: number }>();
  const add = (pr: DashboardPull, series: "mine" | "reviews") => {
    const label = `${pr.owner}/${pr.repo}`;
    const row = rows.get(label) ?? { label, mine: 0, reviews: 0 };
    row[series]++;
    rows.set(label, row);
  };
  authored.value.forEach((p) => add(p, "mine"));
  reviews.value.forEach((p) => add(p, "reviews"));
  const sorted = [...rows.values()].sort(
    (a, b) => b.mine + b.reviews - (a.mine + a.reviews) || a.label.localeCompare(b.label),
  );
  if (sorted.length <= MAX_REPOS) return sorted;
  // Fold the tail into "Andere" instead of letting the list grow.
  const rest = sorted.slice(MAX_REPOS - 1);
  return [
    ...sorted.slice(0, MAX_REPOS - 1),
    {
      label: `Andere (${rest.length})`,
      mine: rest.reduce((n, r) => n + r.mine, 0),
      reviews: rest.reduce((n, r) => n + r.reviews, 0),
    },
  ];
});

const repoMax = computed(() => Math.max(1, ...repoRows.value.map((r) => r.mine + r.reviews)));
const share = (value: number, total: number) => `${(value / total) * 100}%`;

const series = [
  { key: "mine", label: "Meine PRs", color: "var(--series-1)" },
  { key: "reviews", label: "Reviews", color: "var(--series-2)" },
] as const;
</script>

<template>
  <div class="space-y-4 transition-opacity" :class="dashboard.loading && 'opacity-60'">
    <!-- Key figures -->
    <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
      <div class="rounded-xl border border-border bg-card px-4 py-3">
        <p class="text-xs text-muted-foreground">Meine offenen PRs</p>
        <p class="mt-1 text-2xl font-semibold">{{ authored.length }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card px-4 py-3">
        <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
          <TriangleAlert class="size-3.5" :style="{ color: 'var(--status-warning)' }" />Veraltet
        </p>
        <p class="mt-1 text-2xl font-semibold">{{ behindCount }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card px-4 py-3">
        <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
          <CircleX class="size-3.5" :style="{ color: 'var(--status-critical)' }" />Mit Konflikten
        </p>
        <p class="mt-1 text-2xl font-semibold">{{ conflictCount }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card px-4 py-3">
        <p class="text-xs text-muted-foreground">Angeforderte Reviews</p>
        <p class="mt-1 text-2xl font-semibold">{{ reviews.length }}</p>
      </div>
    </div>

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
      <!-- Status of my pull requests -->
      <ChartCard title="Status meiner Pull Requests" :subtitle="`${authored.length} offen`">
        <div v-if="authored.length" class="flex h-3 gap-[2px]" role="img" aria-label="Anteile der Status, siehe Legende">
          <template v-for="row in statusRows" :key="row.label">
            <Tip v-if="row.count" :text="`${row.count} · ${row.label}`">
              <div
                tabindex="0"
                class="h-full min-w-1 outline-none last:rounded-r-[4px] hover:brightness-110 focus-visible:ring-2 focus-visible:ring-foreground/40"
                :style="{ width: share(row.count, authored.length), background: row.color }"
              />
            </Tip>
          </template>
        </div>
        <p v-else class="text-muted-foreground">Keine eigenen offenen Pull Requests.</p>
        <ul class="mt-4 space-y-1.5">
          <li v-for="row in statusRows" :key="row.label" class="flex items-center gap-2 text-[13px]">
            <component :is="row.icon" class="size-4 shrink-0" :style="{ color: row.color }" />
            <span class="flex-1">{{ row.label }}</span>
            <span class="text-muted-foreground tabular-nums">{{ row.count }}</span>
          </li>
        </ul>
        <template #table>
          <table class="w-full">
            <thead class="text-left text-xs text-muted-foreground">
              <tr><th class="py-1 font-medium">Status</th><th class="py-1 text-right font-medium">Anzahl</th></tr>
            </thead>
            <tbody class="tabular-nums">
              <tr v-for="row in statusRows" :key="row.label" class="border-t border-border">
                <td class="py-1">{{ row.label }}</td><td class="py-1 text-right">{{ row.count }}</td>
              </tr>
            </tbody>
          </table>
        </template>
      </ChartCard>

      <!-- Age of open pull requests -->
      <ChartCard title="Alter der offenen Pull Requests" subtitle="seit Erstellung">
        <div class="mb-3 flex gap-4 text-xs">
          <span v-for="s in series" :key="s.key" class="flex items-center gap-1.5">
            <span class="size-2.5 rounded-[2px]" :style="{ background: s.color }" />{{ s.label }}
          </span>
        </div>
        <div class="flex gap-2">
          <!-- y-axis ticks -->
          <div class="relative w-5 shrink-0 text-right text-[11px] text-muted-foreground tabular-nums" :style="{ height: `${PLOT_HEIGHT}px` }">
            <span
              v-for="tick in ageTicks"
              :key="tick"
              class="absolute right-0 translate-y-1/2 leading-none"
              :style="{ bottom: `${(tick / ageTop) * PLOT_HEIGHT}px` }"
            >{{ tick }}</span>
          </div>
          <div class="min-w-0 flex-1">
            <div class="relative" :style="{ height: `${PLOT_HEIGHT}px` }">
              <!-- gridlines (solid hairlines), baseline last -->
              <div
                v-for="tick in ageTicks"
                :key="tick"
                class="absolute inset-x-0 h-px"
                :style="{ bottom: `${(tick / ageTop) * PLOT_HEIGHT}px`, background: tick === 0 ? 'var(--chart-baseline)' : 'var(--chart-grid)' }"
              />
              <div class="absolute inset-0 flex items-end justify-around">
                <div v-for="row in ageRows" :key="row.label" class="flex items-end gap-[2px]">
                  <Tip v-for="s in series" :key="s.key" :text="`${row[s.key]} · ${s.label} (${row.label})`">
                    <div
                      tabindex="0"
                      class="w-3 rounded-t-[4px] outline-none hover:brightness-110 focus-visible:ring-2 focus-visible:ring-foreground/40"
                      :style="{ height: `${Math.max(columnHeight(row[s.key]), row[s.key] ? 2 : 0)}px`, background: s.color }"
                    />
                  </Tip>
                </div>
              </div>
            </div>
            <div class="mt-1.5 flex justify-around text-[11px] text-muted-foreground">
              <span v-for="row in ageRows" :key="row.label" class="w-0 text-center whitespace-nowrap">
                <span class="inline-block -translate-x-1/2">{{ row.label }}</span>
              </span>
            </div>
          </div>
        </div>
        <template #table>
          <table class="w-full">
            <thead class="text-left text-xs text-muted-foreground">
              <tr>
                <th class="py-1 font-medium">Alter</th>
                <th class="py-1 text-right font-medium">Meine PRs</th>
                <th class="py-1 text-right font-medium">Reviews</th>
              </tr>
            </thead>
            <tbody class="tabular-nums">
              <tr v-for="row in ageRows" :key="row.label" class="border-t border-border">
                <td class="py-1">{{ row.label }}</td>
                <td class="py-1 text-right">{{ row.mine }}</td>
                <td class="py-1 text-right">{{ row.reviews }}</td>
              </tr>
            </tbody>
          </table>
        </template>
      </ChartCard>

      <!-- Pull requests per repository -->
      <ChartCard title="Pull Requests nach Repository">
        <div class="mb-3 flex gap-4 text-xs">
          <span v-for="s in series" :key="s.key" class="flex items-center gap-1.5">
            <span class="size-2.5 rounded-[2px]" :style="{ background: s.color }" />{{ s.label }}
          </span>
        </div>
        <ul v-if="repoRows.length" class="space-y-2.5">
          <li v-for="row in repoRows" :key="row.label" class="text-[13px]">
            <p class="mb-1 truncate" :title="row.label">{{ row.label }}</p>
            <div class="flex items-center gap-2">
              <div class="flex h-3 flex-1 gap-[2px]">
                <template v-for="s in series" :key="s.key">
                  <Tip v-if="row[s.key]" :text="`${row[s.key]} · ${s.label}`">
                    <div
                      tabindex="0"
                      class="h-full outline-none last:rounded-r-[4px] hover:brightness-110 focus-visible:ring-2 focus-visible:ring-foreground/40"
                      :style="{ width: share(row[s.key], repoMax), background: s.color }"
                    />
                  </Tip>
                </template>
              </div>
              <span class="w-5 text-right text-xs text-muted-foreground tabular-nums">{{ row.mine + row.reviews }}</span>
            </div>
          </li>
        </ul>
        <p v-else class="text-muted-foreground">Keine Daten.</p>
        <template #table>
          <table class="w-full">
            <thead class="text-left text-xs text-muted-foreground">
              <tr>
                <th class="py-1 font-medium">Repository</th>
                <th class="py-1 text-right font-medium">Meine PRs</th>
                <th class="py-1 text-right font-medium">Reviews</th>
              </tr>
            </thead>
            <tbody class="tabular-nums">
              <tr v-for="row in repoRows" :key="row.label" class="border-t border-border">
                <td class="max-w-0 truncate py-1">{{ row.label }}</td>
                <td class="py-1 text-right">{{ row.mine }}</td>
                <td class="py-1 text-right">{{ row.reviews }}</td>
              </tr>
            </tbody>
          </table>
        </template>
      </ChartCard>
    </div>
  </div>
</template>
