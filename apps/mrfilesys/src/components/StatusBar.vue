<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { GitBranch, LoaderCircle, X } from "@lucide/vue";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import Tip from "@mrtools/ui/components/Tip";
import { cancelDeletion, deletions } from "@/lib/deletion";
import { baseName } from "@/lib/paths";
import { cancelTransfer, transfers } from "@/lib/transfer";
import { formatBytes, formatCount, formatPercent } from "@/lib/format";
import { git } from "@/lib/git";
import { selectedEntries, settings, state, visible } from "@/lib/store";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));

const selectedSize = computed(() => selectedEntries.value.filter((e) => !e.isDir).reduce((sum, e) => sum + e.size, 0));
const hiddenCount = computed(() => state.entries.length - visible.value.length);
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <template v-if="state.path">
      <Tip
        v-if="settings.git.enabled && settings.git.branch && git.repo"
        :text="`Git-Repository ${git.repo.root}${git.repo.upstream ? `\nRemote: ${git.repo.upstream}` : ''}`"
      >
        <span class="inline-flex items-center gap-1 font-medium text-foreground tabular-nums">
          <GitBranch class="size-3.5 text-accent-text" />
          {{ git.repo.branch ?? "losgelöster HEAD" }}
          <span v-if="git.repo.ahead" class="text-muted-foreground">↑{{ git.repo.ahead }}</span>
          <span v-if="git.repo.behind" class="text-muted-foreground">↓{{ git.repo.behind }}</span>
          <span v-if="git.repo.changed" class="font-normal text-amber-600 dark:text-amber-400">
            · {{ formatCount(git.repo.changed) }} geändert
          </span>
        </span>
      </Tip>
      <span class="tabular-nums">{{ formatCount(visible.length) }} Elemente</span>
      <span v-if="hiddenCount > 0" class="tabular-nums">· {{ formatCount(hiddenCount) }} ausgeblendet</span>
      <span v-if="selectedEntries.length" class="font-medium text-foreground tabular-nums">
        · {{ formatCount(selectedEntries.length) }} ausgewählt
        <template v-if="selectedSize">({{ formatBytes(selectedSize) }})</template>
      </span>
    </template>
    <span v-else>{{ state.drives.length }} Laufwerke</span>
    <div class="flex-1" />

    <div v-for="job in transfers.values()" :key="`t${job.id}`" class="flex min-w-0 items-center gap-1.5 text-foreground">
      <LoaderCircle class="size-3.5 shrink-0 animate-spin text-accent-text" />
      <Tip :text="job.current || undefined">
        <span class="truncate tabular-nums">
          {{ job.op === "move" ? "Verschiebe" : "Kopiere" }} {{ job.label }} nach „{{ baseName(job.target) }}“ …
          <template v-if="job.totalFiles">
            {{ formatCount(job.files) }} von {{ formatCount(job.totalFiles) }} Dateien ·
            {{ formatPercent(job.bytes, job.totalBytes) }}
          </template>
          <template v-else>{{ formatCount(job.files) }} Dateien</template>
          <template v-if="job.failed">· <span class="text-red-500">{{ formatCount(job.failed) }} Fehler</span></template>
        </span>
      </Tip>
      <Tip text="Abbrechen – bereits Übertragenes bleibt">
        <button class="icon-btn size-5 [&_svg]:size-3.5" aria-label="Abbrechen" @click="cancelTransfer(job.id)">
          <X />
        </button>
      </Tip>
    </div>

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
