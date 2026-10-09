<script setup lang="ts">
import { ref } from "vue";
import { CircleArrowUp, LoaderCircle, RotateCw } from "@lucide/vue";
import { installNow, updater } from "@/lib/updater";

const showNotes = ref(false);
</script>

<template>
  <div v-if="updater.ready" class="shrink-0 border-b border-border bg-accent/10 px-4 py-2 text-[13px]">
    <div class="flex items-center gap-3">
      <CircleArrowUp class="size-4 shrink-0 text-accent-text" />
      <span class="min-w-0 flex-1">
        Update auf <b class="tabular-nums">{{ updater.ready }}</b> ist bereit
        <span class="text-muted-foreground"> – wird sonst beim Beenden installiert</span>
      </span>
      <button
        v-if="updater.notes"
        class="btn btn-ghost h-7 px-2.5 text-xs"
        :aria-expanded="showNotes"
        @click="showNotes = !showNotes"
      >
        {{ showNotes ? "Weniger anzeigen" : "Was ist neu?" }}
      </button>
      <button class="btn btn-primary h-7 px-2.5 text-xs" :disabled="updater.installing" @click="installNow">
        <LoaderCircle v-if="updater.installing" class="animate-spin" />
        <RotateCw v-else />
        Neu starten
      </button>
    </div>
    <pre
      v-if="showNotes"
      class="mt-2 max-h-40 overflow-y-auto font-sans text-xs whitespace-pre-wrap text-muted-foreground select-text"
    >{{ updater.notes }}</pre>
  </div>
</template>
