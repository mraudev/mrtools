<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { LoaderCircle, X } from "@lucide/vue";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import Tip from "@mrtools/ui/components/Tip";
import { cancelDeletion, deletions } from "@/lib/deletion";
import { formatBytes, formatCount } from "@/lib/format";
import { selectedEntries, state, visible } from "@/lib/store";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));

const selectedSize = computed(() => selectedEntries.value.filter((e) => !e.isDir).reduce((sum, e) => sum + e.size, 0));
const hiddenCount = computed(() => state.entries.length - visible.value.length);
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <template v-if="state.path">
      <span class="tabular-nums">{{ formatCount(visible.length) }} Elemente</span>
      <span v-if="hiddenCount > 0" class="tabular-nums">· {{ formatCount(hiddenCount) }} ausgeblendet</span>
      <span v-if="selectedEntries.length" class="font-medium text-foreground tabular-nums">
        · {{ formatCount(selectedEntries.length) }} ausgewählt
        <template v-if="selectedSize">({{ formatBytes(selectedSize) }})</template>
      </span>
    </template>
    <span v-else>{{ state.drives.length }} Laufwerke</span>
    <div class="flex-1" />

    <div v-for="job in deletions.values()" :key="job.id" class="flex min-w-0 items-center gap-1.5 text-foreground">
      <LoaderCircle class="size-3.5 shrink-0 animate-spin text-accent-text" />
      <Tip :text="job.current || undefined">
        <span class="truncate tabular-nums">
          Lösche {{ job.label }} … {{ formatCount(job.deleted) }} gelöscht
          <template v-if="job.failed">· <span class="text-red-500">{{ formatCount(job.failed) }} Fehler</span></template>
        </span>
      </Tip>
      <Tip text="Abbrechen – bereits Gelöschtes bleibt gelöscht">
        <button class="icon-btn size-5 [&_svg]:size-3.5" aria-label="Löschen abbrechen" @click="cancelDeletion(job.id)">
          <X />
        </button>
      </Tip>
    </div>

    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
