<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { ShieldCheck } from "@lucide/vue";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import { rows, snapshot, state } from "@/lib/store";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <span class="tabular-nums">
      {{ rows.length }} {{ state.mode === "listening" ? "lauschende Ports" : "Verbindungen" }}
      <template v-if="state.query"> (gefiltert)</template>
    </span>
    <span class="tabular-nums">· {{ snapshot.sockets.length }} Sockets</span>
    <span v-if="state.paused" class="font-medium text-accent-text">· angehalten</span>
    <span v-if="state.elevated" class="inline-flex items-center gap-1"><ShieldCheck class="size-3.5" />Administrator</span>
    <div class="flex-1" />
    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
