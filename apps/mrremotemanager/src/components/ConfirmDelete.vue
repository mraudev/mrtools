<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { TriangleAlert } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { remove } from "@/lib/actions";
import { address, displayName } from "@/lib/list";
import { passwords, state } from "@/lib/store";

const open = computed({
  get: () => state.confirmDelete !== null,
  set: (value) => !value && (state.confirmDelete = null),
});

/** Only offered if no other saved Remotedesktop connection uses the same host. */
const hasPassword = computed(() => {
  const c = state.confirmDelete;
  if (!c || c.kind !== "rdp" || !passwords.get(c.host.trim())) return false;
  return !state.connections.some((x) => x.id !== c.id && x.kind === "rdp" && x.host.trim().toLowerCase() === c.host.trim().toLowerCase());
});
const deletePassword = ref(true);
watch(open, (value) => value && (deletePassword.value = true));
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
      <label v-if="hasPassword" class="mt-3 inline-flex items-center gap-2 text-sm">
        <input v-model="deletePassword" type="checkbox" class="size-4 accent-[var(--accent)]" />
        Gespeichertes Passwort auch aus Windows entfernen
      </label>
    </template>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="state.confirmDelete = null">Abbrechen</button>
      <button
        class="btn bg-red-600 text-white hover:bg-red-600/85"
        @click="state.confirmDelete && remove(state.confirmDelete, hasPassword && deletePassword)"
      >
        Löschen
      </button>
    </template>
  </Dialog>
</template>
