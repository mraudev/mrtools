<script setup lang="ts">
import { computed } from "vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { dialog } from "@/lib/nav";

const offen = computed({
  get: () => dialog.offen,
  set: (v) => {
    if (!v) antworten(false);
  },
});

function antworten(ok: boolean) {
  const a = dialog.antwort;
  dialog.offen = false;
  dialog.antwort = null;
  a?.(ok);
}
</script>

<template>
  <Dialog v-model:open="offen" :title="dialog.titel" size="sm">
    <p class="text-sm whitespace-pre-line text-muted-foreground">{{ dialog.text }}</p>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="antworten(false)">Abbrechen</button>
      <button class="btn" :class="dialog.gefaehrlich ? 'bg-red-600 text-white hover:bg-red-600/85' : 'btn-primary'" @click="antworten(true)">
        {{ dialog.ok }}
      </button>
    </template>
  </Dialog>
</template>
