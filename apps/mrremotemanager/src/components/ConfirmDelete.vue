<script setup lang="ts">
import { computed } from "vue";
import { TriangleAlert } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { remove } from "@/lib/actions";
import { address, displayName } from "@/lib/list";
import { state } from "@/lib/store";

const open = computed({
  get: () => state.confirmDelete !== null,
  set: (value) => !value && (state.confirmDelete = null),
});
</script>

<template>
  <Dialog v-model:open="open" :title="state.confirmDelete ? `${displayName(state.confirmDelete)} löschen?` : ''" size="sm">
    <template #icon>
      <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-red-500/15 text-red-500">
        <TriangleAlert class="size-4" />
      </div>
    </template>
    <template v-if="state.confirmDelete">
      <p>Die gespeicherte Verbindung wird entfernt.</p>
      <p class="mt-2 truncate font-mono text-xs text-muted-foreground">{{ address(state.confirmDelete) }}</p>
    </template>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="state.confirmDelete = null">Abbrechen</button>
      <button class="btn bg-red-600 text-white hover:bg-red-600/85" @click="state.confirmDelete && remove(state.confirmDelete)">
        Löschen
      </button>
    </template>
  </Dialog>
</template>
