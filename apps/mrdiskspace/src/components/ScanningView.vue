<script setup lang="ts">
import { LoaderCircle } from "@lucide/vue";
import { formatBytes, formatCount, formatDuration } from "@/lib/format";
import { cancelScan, state } from "@/lib/store";
</script>

<template>
  <div class="grid h-full place-items-center p-6">
    <div class="w-full max-w-lg rounded-xl border border-border bg-card p-6 shadow-sm">
      <div class="flex items-center gap-3">
        <LoaderCircle class="size-5 shrink-0 animate-spin text-accent-text" />
        <div class="min-w-0 flex-1">
          <p class="font-semibold">Scanne {{ state.scanPath }}</p>
          <p class="text-xs text-muted-foreground tabular-nums">
            {{ formatDuration(state.progress?.elapsedMs ?? 0) }}
          </p>
        </div>
        <button class="btn btn-outline" @click="cancelScan">Abbrechen</button>
      </div>

      <dl class="mt-5 grid grid-cols-3 gap-3">
        <div class="rounded-lg bg-foreground/4 px-3 py-2">
          <dt class="text-xs text-muted-foreground">Größe</dt>
          <dd class="text-base font-semibold tabular-nums">{{ formatBytes(state.progress?.bytes ?? 0) }}</dd>
        </div>
        <div class="rounded-lg bg-foreground/4 px-3 py-2">
          <dt class="text-xs text-muted-foreground">Dateien</dt>
          <dd class="text-base font-semibold tabular-nums">{{ formatCount(state.progress?.files ?? 0) }}</dd>
        </div>
        <div class="rounded-lg bg-foreground/4 px-3 py-2">
          <dt class="text-xs text-muted-foreground">Ordner</dt>
          <dd class="text-base font-semibold tabular-nums">{{ formatCount(state.progress?.dirs ?? 0) }}</dd>
        </div>
      </dl>

      <p class="mt-4 truncate font-mono text-xs text-muted-foreground" :title="state.progress?.current">
        {{ state.progress?.current || "…" }}
      </p>
    </div>
  </div>
</template>
