<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Plus, Trash2 } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import ZahlFeld from "../ZahlFeld.vue";
import { api } from "@/lib/api";
import { betrag, datum, heute, zahl } from "@/lib/format";
import type { Mitarbeiter, Nachkalkulation, Zeit } from "@/lib/types";
import { toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ projektId: number }>();
const emit = defineEmits<{ close: [] }>();

const offen = ref(true);
const zeiten = ref<Zeit[]>([]);
const mitarbeiter = ref<Mitarbeiter[]>([]);
const nk = ref<Nachkalkulation>();
const neu = ref<Zeit>({ id: 0, projekt_id: props.projektId, mitarbeiter_id: null, datum: heute(), stunden: 0, posnr: "", text: "" });

async function laden() {
  try {
    zeiten.value = await api.zeiten(props.projektId);
    mitarbeiter.value = await api.mitarbeiter();
    nk.value = await api.nachkalkulation(props.projektId);
  } catch (e) {
    toastError("Zeiten konnten nicht geladen werden", e);
  }
}
onMounted(laden);

async function hinzufuegen() {
  if (!neu.value.stunden) return;
  try {
    await api.zeitSpeichern(neu.value);
    neu.value = { ...neu.value, id: 0, stunden: 0, text: "" };
    await laden();
  } catch (e) {
    toastError("Speichern fehlgeschlagen", e);
  }
}

async function loeschen(id: number) {
  await api.zeitLoeschen(id);
  await laden();
}

const name = (id: number | null) => mitarbeiter.value.find((m) => m.id === id)?.name ?? "–";
</script>

<template>
  <Dialog v-model:open="offen" title="Zeiten & Nachkalkulation" size="xl" @update:open="(v: boolean) => !v && emit('close')">
    <div class="grid grid-cols-[1fr_18rem] gap-4">
      <div class="flex flex-col gap-2">
        <div class="grid grid-cols-[8.5rem_10rem_5rem_4.5rem_1fr_2rem] items-end gap-1">
          <label><span class="label">Datum</span><input v-model="neu.datum" type="date" class="input h-7 text-xs" /></label>
          <label>
            <span class="label">Mitarbeiter</span>
            <select v-model="neu.mitarbeiter_id" class="input h-7 px-1 text-xs">
              <option :value="null">–</option>
              <option v-for="m in mitarbeiter" :key="m.id" :value="m.id">{{ m.name }}</option>
            </select>
          </label>
          <label><span class="label">Stunden</span><ZahlFeld v-model="neu.stunden" class="h-7 text-xs" @enter="hinzufuegen" /></label>
          <label><span class="label">Pos.</span><input v-model="neu.posnr" class="input h-7 text-xs" /></label>
          <label><span class="label">Tätigkeit</span><input v-model="neu.text" class="input h-7 text-xs" @keydown.enter="hinzufuegen" /></label>
          <button class="icon-btn size-7" title="Hinzufügen" @click="hinzufuegen"><Plus /></button>
        </div>
        <div class="max-h-[50vh] overflow-y-auto rounded-md border border-border">
          <div v-for="z in zeiten" :key="z.id" class="group flex items-center gap-3 border-b border-border/60 px-3 py-1 text-xs">
            <span class="w-20 tabular-nums">{{ datum(z.datum) }}</span>
            <span class="w-28 truncate">{{ name(z.mitarbeiter_id) }}</span>
            <span class="w-14 text-right tabular-nums">{{ zahl(z.stunden, 2) }} h</span>
            <span class="w-12 text-muted-foreground">{{ z.posnr }}</span>
            <span class="min-w-0 flex-1 truncate">{{ z.text }}</span>
            <button class="icon-btn size-6 opacity-0 group-hover:opacity-100" @click="loeschen(z.id)"><Trash2 /></button>
          </div>
          <p v-if="!zeiten.length" class="p-4 text-center text-xs text-muted-foreground">Noch keine Stunden erfasst.</p>
        </div>
      </div>
      <div v-if="nk" class="rounded-lg border border-border bg-card p-3 text-[13px]">
        <div class="label">Nachkalkulation</div>
        <p class="mb-2 text-[11px] text-muted-foreground">{{ nk.beleg || "Kein Beleg im Projekt" }}</p>
        <div class="grid grid-cols-[1fr_auto] gap-y-1 tabular-nums">
          <span class="text-muted-foreground">Erlös netto</span><span class="text-right">{{ betrag(nk.erloes) }}</span>
          <span class="text-muted-foreground">Stunden kalkuliert</span><span class="text-right">{{ zahl(nk.soll_stunden, 2) }}</span>
          <span class="text-muted-foreground">Stunden gearbeitet</span>
          <span class="text-right" :class="nk.ist_stunden > nk.soll_stunden && nk.soll_stunden ? 'text-red-500' : ''">{{ zahl(nk.ist_stunden, 2) }}</span>
          <span class="text-muted-foreground">Lohn kalkuliert</span><span class="text-right">{{ betrag(nk.soll_lohn) }}</span>
          <span class="text-muted-foreground">Lohnkosten</span><span class="text-right">{{ betrag(nk.lohnkosten) }}</span>
          <span class="border-t border-border pt-1 font-semibold">Erlös − Lohnkosten</span>
          <span class="border-t border-border pt-1 text-right font-semibold" :class="nk.ergebnis < 0 ? 'text-red-500' : ''">{{ betrag(nk.ergebnis) }}</span>
        </div>
        <div v-if="nk.je_mitarbeiter.length" class="mt-3 border-t border-border pt-2 text-xs">
          <div v-for="[n, s, k] in nk.je_mitarbeiter" :key="n" class="flex gap-2 tabular-nums">
            <span class="flex-1 truncate">{{ n }}</span><span>{{ zahl(s, 2) }} h</span><span class="w-20 text-right">{{ betrag(k) }}</span>
          </div>
        </div>
      </div>
    </div>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="emit('close')">Schließen</button>
    </template>
  </Dialog>
</template>
