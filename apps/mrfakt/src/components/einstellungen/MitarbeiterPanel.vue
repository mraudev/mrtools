<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Plus, Save, Trash2 } from "@lucide/vue";
import ZahlFeld from "../ZahlFeld.vue";
import { api } from "@/lib/api";
import { frage } from "@/lib/nav";
import type { Mitarbeiter } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const liste = ref<Mitarbeiter[]>([]);

async function laden() {
  liste.value = await api.mitarbeiter().catch(() => []);
}
onMounted(laden);

async function speichern() {
  try {
    for (const m of liste.value) m.id = await api.mitarbeiterSpeichern(m);
    toast("success", "Mitarbeiter gespeichert");
  } catch (e) {
    toastError("Speichern fehlgeschlagen", e);
  }
}

async function loeschen(i: number) {
  const m = liste.value[i];
  if (m.id && !(await frage("Mitarbeiter löschen", m.name, "Löschen"))) return;
  if (m.id) await api.mitarbeiterLoeschen(m.id);
  liste.value.splice(i, 1);
}
</script>

<template>
  <div class="rounded-xl border border-border bg-card p-4">
    <div class="label">Mitarbeiter (für Regieberichte, Kalkulation und Nachkalkulation)</div>
    <div class="grid grid-cols-[1fr_12rem_7rem_7rem_4rem_2rem] gap-2 px-1 pb-1 text-[11px] font-medium text-muted-foreground">
      <span>Name</span><span>Qualifikation</span><span class="text-right">Verrechnung/Std.</span><span class="text-right">Kosten/Std.</span><span>aktiv</span><span />
    </div>
    <div v-for="(m, i) in liste" :key="i" class="grid grid-cols-[1fr_12rem_7rem_7rem_4rem_2rem] items-center gap-2 py-0.5">
      <input v-model="m.name" class="input h-7 text-xs" />
      <input v-model="m.qualifikation" class="input h-7 text-xs" placeholder="z. B. Facharbeiter" />
      <ZahlFeld v-model="m.stundensatz" class="h-7 text-xs" />
      <ZahlFeld v-model="m.kosten" class="h-7 text-xs" />
      <input v-model="m.aktiv" type="checkbox" />
      <button class="icon-btn size-7" @click="loeschen(i)"><Trash2 /></button>
    </div>
    <div class="mt-2 flex gap-2">
      <button class="btn btn-outline h-8" @click="liste.push({ id: 0, name: '', qualifikation: '', stundensatz: 0, kosten: 0, aktiv: true })"><Plus />Mitarbeiter</button>
      <button class="btn btn-primary h-8" @click="speichern"><Save />Mitarbeiter speichern</button>
    </div>
  </div>
</template>
