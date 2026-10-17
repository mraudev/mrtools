<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Plus, Save, Trash2 } from "@lucide/vue";
import { api } from "@/lib/api";
import type { Formel } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const formeln = ref<Formel[]>([]);

onMounted(async () => {
  formeln.value = await api.formeln().catch(() => []);
});

function neu() {
  const nummer = Math.max(0, ...formeln.value.map((f) => f.nummer)) + 1;
  formeln.value.push({ id: 0, nummer, name: "Neue Formel", ausdruck: "a*b", hinweis: "Maß 1 / Maß 2" });
}

async function speichern() {
  try {
    await api.formelnSpeichern(formeln.value);
    toast("success", "Formeln gespeichert");
  } catch (e) {
    toastError("Formeln", e);
  }
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <p class="text-xs text-muted-foreground">
      Formeln für Maßzeilen im Aufmaß. Variablen: a = Maß 1, b = Maß 2, c = Maß 3, pi. Erlaubt sind + − * / ^ und Klammern. Das Ergebnis wird mit der Anzahl multipliziert.
      Nummer 0 = Produkt der angegebenen Maße.
    </p>
    <div class="rounded-xl border border-border bg-card p-3">
      <div class="grid grid-cols-[4rem_1fr_12rem_1fr_2rem] gap-2 px-1 pb-1 text-[11px] font-medium text-muted-foreground">
        <span>Nr.</span><span>Name</span><span>Formel</span><span>Bedeutung der Maße</span><span />
      </div>
      <div v-for="(f, i) in formeln" :key="i" class="grid grid-cols-[4rem_1fr_12rem_1fr_2rem] gap-2 py-0.5">
        <input v-model.number="f.nummer" type="number" class="input h-7 text-xs" />
        <input v-model="f.name" class="input h-7 text-xs" />
        <input v-model="f.ausdruck" class="input h-7 font-mono text-xs" />
        <input v-model="f.hinweis" class="input h-7 text-xs" />
        <button class="icon-btn size-7" @click="formeln.splice(i, 1)"><Trash2 /></button>
      </div>
    </div>
    <div class="flex gap-2">
      <button class="btn btn-outline h-8" @click="neu"><Plus />Formel</button>
      <button class="btn btn-primary h-8" @click="speichern"><Save />Speichern</button>
    </div>
  </div>
</template>
