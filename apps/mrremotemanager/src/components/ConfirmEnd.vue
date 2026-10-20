<script setup lang="ts">
import { computed } from "vue";
import { Unplug } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { endSession } from "@/lib/actions";
import { state } from "@/lib/store";

const open = computed({
  get: () => state.confirmEnd !== null,
  set: (value) => !value && (state.confirmEnd = null),
});
</script>

<template>
  <Dialog v-model:open="open" :title="state.confirmEnd ? `Sitzung zu ${state.confirmEnd.host} beenden?` : ''" size="sm">
    <template #icon>
      <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-red-500/15 text-red-500">
        <Unplug class="size-4" />
      </div>
    </template>
    <p v-if="state.confirmEnd">
      {{ state.confirmEnd.process }} (PID {{ state.confirmEnd.pid }}) wird sofort beendet. Die Sitzung auf dem Server bleibt
      bei Remotedesktop meist erhalten und lässt sich später wieder aufnehmen; ungespeicherte Eingaben im Fenster gehen verloren.
    </p>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="state.confirmEnd = null">Abbrechen</button>
      <button class="btn bg-red-600 text-white hover:bg-red-600/85" @click="state.confirmEnd && endSession(state.confirmEnd)">Beenden</button>
    </template>
  </Dialog>
</template>
