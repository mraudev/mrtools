<script setup lang="ts">
import { computed } from "vue";
import ProcIcon from "./ProcIcon.vue";
import Sparkline from "./Sparkline.vue";
import { formatBytes, formatCount, formatCpu } from "@/lib/format";
import { procs, state, systemHistoryCopy } from "@/lib/store";
import type { ProcInfo } from "@/lib/types";

const history = computed(systemHistoryCopy);

const totals = computed(() =>
  procs.value.reduce((t, p) => ({ threads: t.threads + p.threads, handles: t.handles + p.handles }), {
    threads: 0,
    handles: 0,
  }),
);

function top(key: "cpu" | "memory") {
  return [...procs.value].sort((a, b) => b[key] - a[key]).slice(0, 6);
}
const topCpu = computed(() => top("cpu"));
const topMemory = computed(() => top("memory"));

const lists: { title: string; items: () => ProcInfo[]; value: (p: ProcInfo) => string }[] = [
  { title: "Meiste CPU", items: () => topCpu.value, value: (p) => `${formatCpu(p.cpu)} %` },
  { title: "Meister Arbeitsspeicher", items: () => topMemory.value, value: (p) => formatBytes(p.memory) },
];
</script>

<template>
  <div class="flex h-full min-w-0 flex-col gap-3 overflow-y-auto p-3">
    <div>
      <p class="font-semibold">System</p>
      <p class="text-xs text-muted-foreground">Prozess anklicken für Details und Aktionen</p>
    </div>

    <Sparkline
      label="CPU gesamt"
      :values="history.cpu"
      :max="100"
      :format="(v) => `${formatCpu(v)} %`"
      color="var(--series-1)"
      :step="state.interval / 1000"
    />
    <Sparkline
      label="Arbeitsspeicher belegt"
      :values="history.memory"
      :max="state.memoryTotal"
      :format="formatBytes"
      color="var(--series-2)"
      :step="state.interval / 1000"
    />

    <dl class="grid grid-cols-4 gap-2">
      <div v-for="[label, value] in [
        ['Prozesse', formatCount(procs.length)],
        ['Threads', formatCount(totals.threads)],
        ['Handles', formatCount(totals.handles)],
        ['Prozessoren', formatCount(state.cpuCount)],
      ]" :key="label" class="rounded-lg bg-foreground/4 px-2.5 py-1.5">
        <dt class="text-xs text-muted-foreground">{{ label }}</dt>
        <dd class="font-semibold tabular-nums">{{ value }}</dd>
      </div>
    </dl>

    <section v-for="list in lists" :key="list.title">
      <p class="mb-1 text-xs font-medium text-muted-foreground">{{ list.title }}</p>
      <button
        v-for="p in list.items()"
        :key="p.pid"
        class="flex w-full items-center gap-2 rounded-md px-1.5 py-1 text-left text-[13px] hover:bg-foreground/5"
        @click="state.selected = p.pid"
      >
        <ProcIcon :exe="p.exe" />
        <span class="min-w-0 flex-1 truncate">{{ p.name }}</span>
        <span class="text-xs text-muted-foreground tabular-nums">{{ list.value(p) }}</span>
      </button>
    </section>
  </div>
</template>
