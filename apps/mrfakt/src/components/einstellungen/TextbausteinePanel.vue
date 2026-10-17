<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Plus, Save, Trash2 } from "@lucide/vue";
import RichText from "../RichText.vue";
import { api } from "@/lib/api";
import { frage } from "@/lib/nav";
import type { Textbaustein } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const liste = ref<Textbaustein[]>([]);
const aktiv = ref<Textbaustein | null>(null);
const kategorien = [
  { value: "vortext", label: "Vortext" },
  { value: "nachtext", label: "Nachtext" },
  { value: "brief", label: "Brief" },
  { value: "position", label: "Position" },
];

async function laden() {
  liste.value = await api.textbausteine().catch(() => []);
}
onMounted(laden);

async function speichern() {
  if (!aktiv.value) return;
  try {
    aktiv.value.id = await api.textbausteinSpeichern(aktiv.value);
    toast("success", "Textbaustein gespeichert");
    laden();
  } catch (e) {
    toastError("Speichern fehlgeschlagen", e);
  }
}

async function loeschen() {
  if (!aktiv.value?.id || !(await frage("Textbaustein löschen", aktiv.value.name, "Löschen"))) return;
  await api.textbausteinLoeschen(aktiv.value.id);
  aktiv.value = null;
  laden();
}
</script>

<template>
  <div class="grid grid-cols-[16rem_1fr] gap-3">
    <div class="rounded-xl border border-border bg-card">
      <button
        v-for="t in liste"
        :key="t.id"
        class="block w-full truncate border-b border-border/60 px-3 py-1.5 text-left text-[13px]"
        :class="aktiv?.id === t.id ? 'bg-accent/12' : 'hover:bg-foreground/4'"
        @click="aktiv = { ...t }"
      >
        {{ t.name }} <span class="text-[11px] text-muted-foreground">{{ t.kategorie }}</span>
      </button>
      <button class="btn btn-ghost m-1 h-7 text-xs" @click="aktiv = { id: 0, kategorie: 'nachtext', name: 'Neuer Baustein', text: '' }"><Plus />Neu</button>
    </div>
    <div v-if="aktiv" class="flex flex-col gap-3 rounded-xl border border-border bg-card p-4">
      <div class="grid grid-cols-[1fr_10rem] gap-3">
        <label><span class="label">Name</span><input v-model="aktiv.name" class="input" /></label>
        <label>
          <span class="label">Verwendung</span>
          <select v-model="aktiv.kategorie" class="input"><option v-for="k in kategorien" :key="k.value" :value="k.value">{{ k.label }}</option></select>
        </label>
      </div>
      <RichText :key="aktiv.id" v-model="aktiv.text" />
      <div class="flex gap-2">
        <button class="btn btn-primary h-8" @click="speichern"><Save />Speichern</button>
        <button v-if="aktiv.id" class="btn btn-ghost h-8 text-red-500" @click="loeschen"><Trash2 />Löschen</button>
      </div>
    </div>
  </div>
</template>
