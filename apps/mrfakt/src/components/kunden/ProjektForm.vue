<script setup lang="ts">
import { ref } from "vue";
import Dialog from "@mrtools/ui/components/Dialog";
import Segmented from "@mrtools/ui/components/Segmented";
import { api } from "@/lib/api";
import type { Projekt } from "@/lib/types";
import { toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ projekt: Projekt }>();
const emit = defineEmits<{ close: []; saved: [id: number] }>();

const p = ref<Projekt>({
  id: props.projekt.id,
  kunde_id: props.projekt.kunde_id,
  nummer: props.projekt.nummer,
  bezeichnung: props.projekt.bezeichnung,
  datum: props.projekt.datum,
  baustelle: props.projekt.baustelle,
  status: props.projekt.status || "offen",
  notiz: props.projekt.notiz,
});
const offen = ref(true);

async function speichern() {
  try {
    emit("saved", await api.projektSpeichern(p.value));
  } catch (e) {
    toastError("Projekt konnte nicht gespeichert werden", e);
  }
}
</script>

<template>
  <Dialog v-model:open="offen" :title="p.id ? 'Projekt bearbeiten' : 'Neues Projekt'" @update:open="(v: boolean) => !v && emit('close')">
    <form id="projekt-form" class="grid grid-cols-4 gap-3" @submit.prevent="speichern">
      <label class="col-span-1"><span class="label">Auftrags-Nr.</span><input v-model="p.nummer" class="input" placeholder="automatisch" /></label>
      <label class="col-span-3"><span class="label">Bezeichnung</span><input v-model="p.bezeichnung" class="input" placeholder="z. B. Wohnhaus und Garage" autofocus /></label>
      <label class="col-span-1"><span class="label">Datum</span><input v-model="p.datum" type="date" class="input" /></label>
      <label class="col-span-3"><span class="label">Baustelle (falls abweichend)</span><input v-model="p.baustelle" class="input" /></label>
      <div class="col-span-4">
        <span class="label">Status</span>
        <Segmented v-model="p.status" :options="[{ value: 'offen', label: 'Offen' }, { value: 'abgeschlossen', label: 'Abgeschlossen' }]" />
      </div>
      <label class="col-span-4"><span class="label">Notiz</span><textarea v-model="p.notiz" class="input h-20 py-1.5" /></label>
    </form>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="emit('close')">Abbrechen</button>
      <button class="btn btn-primary" type="submit" form="projekt-form">Speichern</button>
    </template>
  </Dialog>
</template>
