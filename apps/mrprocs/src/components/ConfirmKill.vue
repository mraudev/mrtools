<script setup lang="ts">
import { computed } from "vue";
import { TriangleAlert } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { confirm, confirmKill, isCritical } from "@/lib/actions";

const critical = computed(() => !!confirm.proc && isCritical(confirm.proc));
const title = computed(() =>
  confirm.proc ? `${confirm.tree ? "Prozessstruktur" : "Prozess"} „${confirm.proc.name}“ beenden?` : "",
);
</script>

<template>
  <Dialog v-model:open="confirm.open" :title="title" :description="confirm.proc ? `PID ${confirm.proc.pid}` : undefined" size="sm">
    <div class="space-y-3 text-[13px]">
      <p>
        Das Programm wird sofort beendet, nicht gespeicherte Daten gehen verloren.
        <template v-if="confirm.tree">
          {{
            confirm.children === 1
              ? "Ein untergeordneter Prozess wird ebenfalls beendet."
              : confirm.children
                ? `${confirm.children} untergeordnete Prozesse werden ebenfalls beendet.`
                : "Es gibt keine untergeordneten Prozesse."
          }}
        </template>
      </p>
      <p v-if="critical" class="flex gap-2 rounded-lg bg-red-500/10 p-3 text-red-700 dark:text-red-300">
        <TriangleAlert class="mt-0.5 size-4 shrink-0" />
        Das ist ein Windows-Systemprozess. Ihn zu beenden führt zu einem Absturz oder Neustart von Windows.
      </p>
    </div>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" :autofocus="critical" @click="confirm.open = false">Abbrechen</button>
      <button class="btn bg-red-600 text-white hover:bg-red-600/85" :autofocus="!critical" @click="confirmKill">
        Beenden
      </button>
    </template>
  </Dialog>
</template>
