<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import { state, visible } from "@/lib/store";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <span class="tabular-nums">
      {{ visible.length }} {{ visible.length === 1 ? "Verbindung" : "Verbindungen" }}
      <template v-if="state.query"> (gefiltert)</template>
    </span>
    <div class="flex-1" />
    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
