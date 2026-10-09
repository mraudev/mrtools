<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { TriangleAlert } from "@lucide/vue";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import Tip from "@mrtools/ui/components/Tip";
import { formatBytes, formatCount, formatDuration } from "@/lib/format";
import { root, state } from "@/lib/store";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <template v-if="state.phase === 'result' && root">
      <span class="tabular-nums">{{ formatBytes(root.size) }}</span>
      <span class="tabular-nums">· {{ formatCount(root.files) }} Dateien</span>
      <span class="tabular-nums">· {{ formatCount(root.dirs) }} Ordner</span>
      <span class="tabular-nums">· gescannt in {{ formatDuration(state.elapsedMs) }}</span>
      <Tip
        v-if="root.errors"
        text="Diese Einträge konnten nicht gelesen werden (meist fehlende Rechte). Als Administrator gestartet werden sie mitgezählt."
      >
        <span class="inline-flex items-center gap-1 text-amber-600 dark:text-amber-400">
          <TriangleAlert class="size-3.5" />{{ formatCount(root.errors) }} nicht lesbar
        </span>
      </Tip>
    </template>
    <span v-else>{{ state.drives.length }} Laufwerke</span>
    <div class="flex-1" />

    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
