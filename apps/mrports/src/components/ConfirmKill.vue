<script setup lang="ts">
import { computed } from "vue";
import { TriangleAlert } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { kill } from "@/lib/actions";
import { snapshot, state } from "@/lib/store";

const open = computed({
  get: () => state.confirmKill !== null,
  set: (value) => !value && (state.confirmKill = null),
});

/** Other ports of the same process – they are freed too. */
const others = computed(() => {
  const row = state.confirmKill;
  if (!row) return [];
  const ports = snapshot.value.sockets.filter((s) => s.pid === row.pid && s.localPort !== row.port && (s.protocol === "UDP" || s.state === "Lauscht"));
  return [...new Set(ports.map((s) => s.localPort))].sort((a, b) => a - b);
});
</script>

<template>
  <Dialog v-model:open="open" :title="state.confirmKill ? `${state.confirmKill.process.name} beenden?` : ''" size="sm">
    <template #icon>
      <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-red-500/15 text-red-500">
        <TriangleAlert class="size-4" />
      </div>
    </template>
    <template v-if="state.confirmKill">
      <p>
        Der Prozess (PID {{ state.confirmKill.pid }}) wird sofort beendet, ungespeicherte Daten gehen verloren.
        Danach ist Port <b class="tabular-nums">{{ state.confirmKill.port }}</b> frei<template v-if="others.length">
          – ebenso {{ others.slice(0, 8).join(", ") }}<template v-if="others.length > 8"> und weitere</template></template>.
      </p>
      <p v-if="state.confirmKill.process.cwd" class="mt-2 truncate font-mono text-xs text-muted-foreground">
        {{ state.confirmKill.process.cwd }}
      </p>
    </template>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="state.confirmKill = null">Abbrechen</button>
      <button
        class="btn bg-red-600 text-white hover:bg-red-600/85"
        @click="state.confirmKill && kill(state.confirmKill)"
      >
        Beenden
      </button>
    </template>
  </Dialog>
</template>
