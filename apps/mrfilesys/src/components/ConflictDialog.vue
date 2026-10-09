<script setup lang="ts">
import { computed } from "vue";
import { CopyPlus, FileExclamationPoint, Replace, SkipForward } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { formatCount } from "@/lib/format";
import { baseName } from "@/lib/paths";
import { conflictQuestion } from "@/lib/transfer";

const SHOWN = 6;

const open = computed({
  get: () => conflictQuestion.value !== null,
  set: (value) => !value && conflictQuestion.value?.answer(null),
});
const question = computed(() => conflictQuestion.value);
const title = computed(() => {
  const count = question.value?.names.length ?? 0;
  return count === 1 ? "Element ist schon vorhanden" : `${formatCount(count)} Elemente sind schon vorhanden`;
});

const choices = [
  { value: "replace" as const, icon: Replace, label: "Ersetzen", hint: "Dateien überschreiben, Ordner zusammenführen" },
  { value: "skip" as const, icon: SkipForward, label: "Überspringen", hint: "Vorhandene Elemente nicht anfassen" },
  { value: "keepBoth" as const, icon: CopyPlus, label: "Beide behalten", hint: "Neue Elemente bekommen einen Namen wie „x (2)“" },
];
</script>

<template>
  <Dialog
    v-model:open="open"
    :title="title"
    :description="question ? `in „${baseName(question.target) || question.target}“` : undefined"
    size="sm"
  >
    <template #icon>
      <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text">
        <FileExclamationPoint class="size-4" />
      </div>
    </template>
    <template v-if="question">
      <ul class="mb-3 rounded-md border border-border bg-background px-2.5 py-1.5 text-xs">
        <li v-for="name in question.names.slice(0, SHOWN)" :key="name" class="truncate">{{ name }}</li>
        <li v-if="question.names.length > SHOWN" class="text-muted-foreground">
          … und {{ formatCount(question.names.length - SHOWN) }} weitere
        </li>
      </ul>
      <div class="flex flex-col gap-1.5">
        <button
          v-for="choice in choices"
          :key="choice.value"
          class="flex items-start gap-3 rounded-lg border border-border px-3 py-2 text-left transition-colors hover:border-accent/60 hover:bg-accent/5"
          @click="question.answer(choice.value)"
        >
          <component :is="choice.icon" class="mt-0.5 size-4 shrink-0 text-accent-text" />
          <span>
            <span class="block font-medium">{{ choice.label }}</span>
            <span class="block text-xs text-muted-foreground">{{ choice.hint }}</span>
          </span>
        </button>
      </div>
    </template>
  </Dialog>
</template>
