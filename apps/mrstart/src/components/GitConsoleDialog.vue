<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { ClipboardCheck, CloudDownload, CloudUpload, LoaderCircle } from "@lucide/vue";
import Dialog from "./ui/Dialog.vue";
import { openGitTool } from "@/lib/actions";
import { gitConsole, type ConsoleLine } from "@/lib/gitConsole";

const scroller = ref<HTMLElement>();

const title = computed(() => `${gitConsole.action === "push" ? "Push" : "Pull"} · ${gitConsole.project?.name ?? ""}`);

// Follow the output while it streams in.
watch(
  () => [gitConsole.lines.length, gitConsole.lines[gitConsole.lines.length - 1]?.text],
  async () => {
    await nextTick();
    scroller.value?.scrollTo({ top: scroller.value.scrollHeight });
  },
);

function lineClass(line: ConsoleLine): string {
  if (line.kind === "command") return "text-accent-text font-semibold";
  if (line.kind === "success") return "text-emerald-500 font-semibold";
  if (line.kind === "error") return "text-red-500 font-semibold";
  if (/\b(error|fatal|conflict|failed|rejected)\b|could not/i.test(line.text)) return "text-red-400";
  if (/autostash|warning|hint:/i.test(line.text)) return "text-amber-500";
  if (/up to date|up-to-date|successfully|^done/i.test(line.text)) return "text-emerald-500";
  return "";
}
</script>

<template>
  <Dialog
    v-model:open="gitConsole.open"
    :title="title"
    :description="gitConsole.project?.path"
    size="lg"
    :persistent="gitConsole.running"
  >
    <template #icon>
      <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-[18px]">
        <CloudUpload v-if="gitConsole.action === 'push'" />
        <CloudDownload v-else />
      </div>
    </template>

    <div class="relative h-1 overflow-hidden rounded-full bg-foreground/8">
      <div v-if="gitConsole.running" class="absolute inset-y-0 w-1/3 animate-[slide_1.1s_ease-in-out_infinite] rounded-full bg-accent" />
      <div v-else class="absolute inset-0 bg-accent/60" />
    </div>

    <div
      ref="scroller"
      class="mt-3 h-[50vh] overflow-y-auto rounded-lg border border-border bg-background p-3 font-mono text-[12.5px] leading-5 select-text"
    >
      <div v-for="(line, index) in gitConsole.lines" :key="index" class="flex gap-3">
        <span class="w-7 shrink-0 text-right text-muted-foreground/50 tabular-nums select-none">{{ index + 1 }}</span>
        <span class="min-w-0 break-all whitespace-pre-wrap" :class="lineClass(line)">
          <template v-if="line.kind === 'command'">$ </template>{{ line.text }}
        </span>
      </div>
    </div>

    <template #footer>
      <button
        class="btn btn-ghost"
        :disabled="gitConsole.running || !gitConsole.project"
        @click="gitConsole.project && openGitTool(gitConsole.project, 'status')"
      >
        <ClipboardCheck />Änderungen prüfen
      </button>
      <div class="flex-1" />
      <span v-if="gitConsole.running" class="inline-flex items-center gap-2 text-muted-foreground">
        <LoaderCircle class="size-4 animate-spin" />läuft …
      </span>
      <button class="btn btn-primary" :disabled="gitConsole.running" @click="gitConsole.open = false">
        Schließen
      </button>
    </template>
  </Dialog>
</template>

<style>
@keyframes slide {
  from {
    left: -33%;
  }
  to {
    left: 100%;
  }
}
</style>
