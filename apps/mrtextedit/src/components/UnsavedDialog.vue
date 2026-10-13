<script setup lang="ts">
import { computed } from "vue";
import { FileWarning } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { doc } from "@/lib/doc";
import { fileName } from "@/lib/document";

const open = computed({
  get: () => doc.unsaved !== null,
  set: (value) => !value && doc.unsaved?.("cancel"),
});
</script>

<template>
  <Dialog v-model:open="open" title="Änderungen speichern?" size="sm">
    <template #icon>
      <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text">
        <FileWarning class="size-4" />
      </div>
    </template>
    <p>„{{ doc.path ? fileName(doc.path) : "Unbenannt" }}“ wurde geändert. Ohne Speichern gehen die Änderungen verloren.</p>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="doc.unsaved?.('cancel')">Abbrechen</button>
      <button class="btn btn-outline" @click="doc.unsaved?.('discard')">Nicht speichern</button>
      <button class="btn btn-primary" @click="doc.unsaved?.('save')">Speichern</button>
    </template>
  </Dialog>
</template>
