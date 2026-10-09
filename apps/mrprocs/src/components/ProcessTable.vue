<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { ArrowDown, ArrowUp, ChevronRight, Pause } from "@lucide/vue";
import ProcIcon from "./ProcIcon.vue";
import ProcessMenu from "./ProcessMenu.vue";
import { askKill } from "@/lib/actions";
import { formatBytes, formatCount, formatCpu, formatRate } from "@/lib/format";
import type { SortKey } from "@/lib/rows";
import { byPid, isEnded, isNew, rows, setSort, state, toggleCollapsed } from "@/lib/store";
import type { ProcInfo } from "@/lib/types";

const ROW = 26;
/** Extra rows rendered above and below the viewport. */
const OVERSCAN = 8;
const COLUMNS = "grid-cols-[minmax(200px,1fr)_64px_68px_104px_96px_64px_minmax(110px,170px)]";

const columns: { key: SortKey; label: string; align: "left" | "right"; title?: string }[] = [
  { key: "name", label: "Name", align: "left" },
  { key: "pid", label: "PID", align: "right" },
  { key: "cpu", label: "CPU %", align: "right", title: "Anteil an der gesamten Rechenleistung" },
  { key: "memory", label: "Arbeitsspeicher", align: "right", title: "Privater Arbeitssatz wie im Task-Manager" },
  { key: "disk", label: "Datenträger", align: "right", title: "Lesen und Schreiben pro Sekunde" },
  { key: "threads", label: "Threads", align: "right" },
  { key: "user", label: "Benutzer", align: "left" },
];

const container = ref<HTMLElement>();
const scrollTop = ref(0);
const height = ref(0);
let observer: ResizeObserver | undefined;

onMounted(() => {
  observer = new ResizeObserver(([entry]) => (height.value = entry.contentRect.height));
  if (container.value) observer.observe(container.value);
});
onUnmounted(() => observer?.disconnect());

const start = computed(() => Math.max(0, Math.floor(scrollTop.value / ROW) - OVERSCAN));
const visible = computed(() =>
  rows.value.slice(start.value, Math.floor(scrollTop.value / ROW) + Math.ceil(height.value / ROW) + OVERSCAN),
);

const menuPid = ref<number | null>(null);
const menuProc = computed(() => (menuPid.value === null ? undefined : byPid.value.get(menuPid.value)));

/** Tint that grows with the load, like the Task Manager heat map. */
function heat(fraction: number) {
  const pct = Math.round(Math.min(1, Math.max(0, fraction)) * 40);
  return pct >= 2 ? { background: `color-mix(in oklab, var(--accent) ${pct}%, transparent)` } : undefined;
}

function rowClass(p: ProcInfo) {
  if (p.pid === state.selected) return "bg-accent/15";
  if (isEnded(p.pid)) return "bg-red-500/12 opacity-60";
  if (isNew(p.pid)) return "bg-emerald-500/14";
  return "hover:bg-foreground/5";
}

function scrollToIndex(i: number) {
  const el = container.value;
  if (!el || i < 0) return;
  const top = i * ROW;
  if (top < el.scrollTop) el.scrollTop = top;
  else if (top + ROW > el.scrollTop + el.clientHeight) el.scrollTop = top + ROW - el.clientHeight;
}

watch(
  () => state.selected,
  (pid) => scrollToIndex(rows.value.findIndex((r) => r.proc.pid === pid)),
  { flush: "post" },
);

function onKeydown(event: KeyboardEvent) {
  const list = rows.value;
  const i = list.findIndex((r) => r.proc.pid === state.selected);
  const row = list[i];
  const page = Math.max(1, Math.floor(height.value / ROW) - 1);
  const go = (index: number) => {
    event.preventDefault();
    const target = list[Math.min(Math.max(index, 0), list.length - 1)];
    if (target) state.selected = target.proc.pid;
  };
  switch (event.key) {
    case "ArrowDown":
      return go(i + 1);
    case "ArrowUp":
      return go(i < 0 ? 0 : i - 1);
    case "PageDown":
      return go(i + page);
    case "PageUp":
      return go(i - page);
    case "Home":
      return go(0);
    case "End":
      return go(list.length - 1);
    case "ArrowRight":
    case "ArrowLeft":
      if (!row?.hasChildren || (event.key === "ArrowRight") === row.expanded) return;
      event.preventDefault();
      return toggleCollapsed(row.proc.pid);
    case "Delete":
      if (!row || isEnded(row.proc.pid)) return;
      event.preventDefault();
      return askKill(row.proc, event.shiftKey);
  }
}
</script>

<template>
  <div class="flex h-full min-w-0 flex-col">
    <div class="grid shrink-0 border-b border-border px-2 text-xs font-medium text-muted-foreground" :class="COLUMNS">
      <button
        v-for="col in columns"
        :key="col.key"
        class="flex items-center gap-1 py-1.5 transition-colors hover:text-foreground [&_svg]:size-3"
        :class="[
          col.align === 'right' ? 'flex-row-reverse pr-3 text-right' : 'pl-1 text-left',
          col.key === 'name' && 'pl-7',
          state.sort.key === col.key && 'text-foreground',
        ]"
        :title="col.title"
        @click="setSort(col.key)"
      >
        <span class="truncate">{{ col.label }}</span>
        <template v-if="state.sort.key === col.key">
          <ArrowDown v-if="state.sort.desc" />
          <ArrowUp v-else />
        </template>
      </button>
    </div>

    <ProcessMenu :proc="menuProc">
      <div
        ref="container"
        class="min-h-0 flex-1 overflow-auto outline-none"
        tabindex="0"
        @scroll="scrollTop = ($event.target as HTMLElement).scrollTop"
        @keydown="onKeydown"
      >
        <div class="relative" :style="{ height: `${rows.length * ROW}px` }">
          <div class="absolute inset-x-0" :style="{ top: `${start * ROW}px` }">
            <div
              v-for="row in visible"
              :key="row.proc.pid"
              class="grid h-[26px] cursor-default items-center px-2 text-[13px]"
              :class="[COLUMNS, rowClass(row.proc)]"
              :title="row.proc.exe || undefined"
              @click="state.selected = row.proc.pid"
              @dblclick="row.hasChildren && toggleCollapsed(row.proc.pid)"
              @contextmenu="(menuPid = row.proc.pid), (state.selected = row.proc.pid)"
            >
              <div class="flex min-w-0 items-center gap-1.5" :style="{ paddingLeft: `${row.depth * 16}px` }">
                <button
                  v-if="row.hasChildren"
                  class="grid size-4.5 shrink-0 place-items-center rounded text-muted-foreground hover:bg-foreground/10 hover:text-foreground"
                  :aria-label="row.expanded ? 'Zuklappen' : 'Aufklappen'"
                  tabindex="-1"
                  @click.stop="toggleCollapsed(row.proc.pid)"
                  @dblclick.stop
                >
                  <ChevronRight class="size-3.5 transition-transform" :class="row.expanded && 'rotate-90'" />
                </button>
                <span v-else class="w-4.5 shrink-0" />
                <ProcIcon :exe="row.proc.exe" />
                <span class="truncate" :class="row.proc.suspended && 'text-muted-foreground'">{{ row.proc.name }}</span>
                <span
                  v-if="row.proc.suspended"
                  class="inline-flex shrink-0 items-center gap-0.5 rounded bg-foreground/8 px-1 text-[11px] text-muted-foreground"
                >
                  <Pause class="size-2.5" />Angehalten
                </span>
              </div>
              <div class="pr-3 text-right text-muted-foreground tabular-nums">{{ row.proc.pid }}</div>
              <div class="h-full pr-3 text-right leading-[26px] tabular-nums" :style="heat(row.proc.cpu / 50)">
                {{ formatCpu(row.proc.cpu) }}
              </div>
              <div
                class="h-full pr-3 text-right leading-[26px] tabular-nums"
                :style="heat(row.proc.memory / (state.memoryTotal * 0.15 || 1))"
              >
                {{ formatBytes(row.proc.memory) }}
              </div>
              <div
                class="h-full pr-3 text-right leading-[26px] text-muted-foreground tabular-nums"
                :style="heat((row.proc.diskRead + row.proc.diskWrite) / (20 * 1024 * 1024))"
              >
                {{ formatRate(row.proc.diskRead + row.proc.diskWrite) }}
              </div>
              <div class="pr-3 text-right text-muted-foreground tabular-nums">{{ formatCount(row.proc.threads) }}</div>
              <div class="truncate pl-1 text-xs text-muted-foreground" :title="row.proc.user">
                {{ row.proc.user.split("\\").pop() }}
              </div>
            </div>
          </div>
        </div>
        <p v-if="state.loaded && !rows.length" class="py-10 text-center text-muted-foreground">
          Kein Prozess passt zu „{{ state.query }}“
        </p>
      </div>
    </ProcessMenu>
  </div>
</template>
