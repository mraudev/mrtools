<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { ShieldCheck } from "@lucide/vue";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import Tip from "@mrtools/ui/components/Tip";
import { formatBytes, formatCount, formatCpu, formatPercent } from "@/lib/format";
import { INTERVALS, procs, rows, state } from "@/lib/store";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <span class="tabular-nums">
      {{ formatCount(procs.length) }} Prozesse<template v-if="state.query"> · {{ formatCount(rows.length) }} angezeigt</template>
    </span>
    <span class="tabular-nums">CPU {{ formatCpu(state.cpu) }} %</span>
    <span class="tabular-nums">
      Arbeitsspeicher {{ formatBytes(state.memoryUsed) }} von {{ formatBytes(state.memoryTotal) }}
      ({{ formatPercent(state.memoryUsed, state.memoryTotal) }})
    </span>
    <span v-if="state.paused" class="rounded bg-accent/15 px-1.5 font-medium text-accent-text">Pausiert</span>
    <div class="flex-1" />

    <label class="flex items-center gap-1.5">
      Aktualisierung
      <select
        v-model.number="state.interval"
        class="h-5 rounded border border-border bg-background px-1 text-xs text-foreground outline-none focus:border-accent"
      >
        <option v-for="ms in INTERVALS" :key="ms" :value="ms">
          {{ (ms / 1000).toLocaleString("de-DE") }} s
        </option>
      </select>
    </label>
    <Tip v-if="state.elevated" text="mrprocs läuft mit Administratorrechten">
      <span class="inline-flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
        <ShieldCheck class="size-3.5" />Administrator
      </span>
    </Tip>
    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
