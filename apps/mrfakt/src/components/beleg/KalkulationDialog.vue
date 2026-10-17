<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { Box, Plus, Trash2 } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import ZahlFeld from "../ZahlFeld.vue";
import ArtikelWahl from "./ArtikelWahl.vue";
import { api } from "@/lib/api";
import { betrag, kurztext, zahl } from "@/lib/format";
import type { Artikel, KalkErgebnis, Kalkulation, Kalkulationssaetze, Mitarbeiter, Position } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ position: Position }>();
const emit = defineEmits<{ close: []; gespeichert: [] }>();

const offen = ref(true);
const zeilen = ref<Kalkulation[]>([]);
const saetze = ref<Kalkulationssaetze>();
const ergebnis = ref<KalkErgebnis>({ ep: 0, lohn: 0, minuten: 0, zeilen: [] });
const gesperrt = ref(false);
const mitarbeiter = ref<Mitarbeiter[]>([]);
const artikelWahl = ref(false);

const arten = [
  { value: "L", label: "Lohn (Minuten)" },
  { value: "M", label: "Material" },
  { value: "G", label: "Gerät" },
  { value: "F", label: "Fremdleistung" },
  { value: "S", label: "Sonstiges" },
];

onMounted(async () => {
  try {
    const d = await api.kalkulation(props.position.id);
    zeilen.value = d.zeilen;
    saetze.value = d.saetze;
    ergebnis.value = d.ergebnis;
    gesperrt.value = d.gesperrt;
    mitarbeiter.value = (await api.mitarbeiter()).filter((m) => m.aktiv);
  } catch (e) {
    toastError("Kalkulation konnte nicht geladen werden", e);
  }
});

let timer: ReturnType<typeof setTimeout> | undefined;
watch(
  zeilen,
  () => {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      ergebnis.value = await api.kalkulationRechnen(zeilen.value).catch(() => ergebnis.value);
    }, 200);
  },
  { deep: true },
);

function neu(art: Kalkulation["art"]) {
  zeilen.value.push({ id: 0, position_id: props.position.id, pos: zeilen.value.length, art, text: art === "L" ? "Lohn" : "", menge: art === "L" ? 10 : 1, einheit: art === "L" ? "min" : "", preis: 0, zuschlag: "", artikel_id: null });
}

async function artikel(a: Artikel) {
  artikelWahl.value = false;
  zeilen.value.push({ ...(await api.kalkulationArtikel(a.id)), position_id: props.position.id, pos: zeilen.value.length });
}

function satzVon(m: number | null) {
  return mitarbeiter.value.find((x) => x.id === m)?.stundensatz ?? 0;
}

async function speichern(uebernehmen: boolean) {
  try {
    await api.kalkulationSpeichern(props.position.id, zeilen.value, uebernehmen);
    toast("success", uebernehmen ? `EP ${betrag(ergebnis.value.ep)} übernommen` : "Kalkulation gespeichert");
    emit("gespeichert");
  } catch (e) {
    toastError("Kalkulation", e);
  }
}

const zuschlagVon = (art: string) => {
  const s = saetze.value;
  if (!s) return 0;
  return { L: s.zuschlag_lohn, M: s.zuschlag_material, G: s.zuschlag_geraet, F: s.zuschlag_fremd, S: s.zuschlag_sonstig }[art] ?? 0;
};
</script>

<template>
  <Dialog v-model:open="offen" :title="`Kalkulation – Pos. ${position.posnr} ${kurztext(position.text)}`" size="xl" @update:open="(v: boolean) => !v && emit('close')">
    <p class="mb-2 text-xs text-muted-foreground">
      Je {{ position.einheit || "Einheit" }}. Lohn in Minuten × Stundensatz ({{ betrag(saetze?.stundensatz ?? 0) }} €, ohne Angabe), Material/Gerät/Fremd: Menge × Preis. Zuschläge aus den
      Einstellungen, wenn das Feld leer ist.
    </p>
    <div class="grid grid-cols-[8rem_1fr_5rem_4rem_6rem_5rem_6rem_6rem_2rem] gap-1 px-1 pb-1 text-[11px] font-medium text-muted-foreground">
      <span>Art</span><span>Text</span><span class="text-right">Menge</span><span>Einh.</span><span class="text-right">Preis</span><span class="text-right">Zuschl. %</span>
      <span class="text-right">Kosten</span><span class="text-right">Preis</span><span />
    </div>
    <div class="max-h-[50vh] overflow-y-auto">
      <div v-for="(z, i) in zeilen" :key="i" class="grid grid-cols-[8rem_1fr_5rem_4rem_6rem_5rem_6rem_6rem_2rem] items-center gap-1 px-1 py-0.5">
        <select v-model="z.art" class="input h-7 px-1 text-xs" :disabled="gesperrt">
          <option v-for="a in arten" :key="a.value" :value="a.value">{{ a.label }}</option>
        </select>
        <input v-model="z.text" class="input h-7 text-xs" :disabled="gesperrt" />
        <ZahlFeld v-model="z.menge" class="h-7 text-xs" :disabled="gesperrt" />
        <input v-model="z.einheit" class="input h-7 text-xs" :disabled="gesperrt" />
        <div class="relative">
          <ZahlFeld v-model="z.preis" class="h-7 text-xs" :disabled="gesperrt" :placeholder="z.art === 'L' ? betrag(saetze?.stundensatz ?? 0) : ''" />
          <select
            v-if="z.art === 'L' && mitarbeiter.length && !gesperrt"
            class="absolute top-0 left-0 h-7 w-5 cursor-pointer opacity-0"
            title="Stundensatz eines Mitarbeiters"
            @change="z.preis = satzVon(Number(($event.target as HTMLSelectElement).value))"
          >
            <option value="">–</option>
            <option v-for="m in mitarbeiter" :key="m.id" :value="m.id">{{ m.name }} ({{ betrag(m.stundensatz) }})</option>
          </select>
        </div>
        <input v-model="z.zuschlag" class="input h-7 text-right text-xs" :placeholder="zahl(zuschlagVon(z.art))" :disabled="gesperrt" />
        <span class="text-right text-xs tabular-nums">{{ betrag(ergebnis.zeilen[i]?.[0]) }}</span>
        <span class="text-right text-xs tabular-nums">{{ betrag(ergebnis.zeilen[i]?.[1]) }}</span>
        <button v-if="!gesperrt" class="icon-btn size-6" @click="zeilen.splice(i, 1)"><Trash2 /></button>
      </div>
    </div>
    <div v-if="!gesperrt" class="mt-2 flex flex-wrap gap-1">
      <button v-for="a in arten" :key="a.value" class="btn btn-ghost h-7 px-2 text-xs" @click="neu(a.value as Kalkulation['art'])"><Plus />{{ a.label.split(" ")[0] }}</button>
      <button class="btn btn-ghost h-7 px-2 text-xs" @click="artikelWahl = true"><Box />Material aus Artikel</button>
    </div>
    <div class="mt-3 flex justify-end gap-6 border-t border-border pt-2 text-sm tabular-nums">
      <span class="text-muted-foreground">Lohnzeit {{ zahl(ergebnis.minuten, 1) }} min</span>
      <span class="text-muted-foreground">Lohnanteil {{ betrag(ergebnis.lohn) }} €</span>
      <span class="font-semibold">EP {{ betrag(ergebnis.ep) }} €</span>
      <span class="text-muted-foreground">bisher {{ betrag(position.ep) }} €</span>
    </div>
    <ArtikelWahl v-if="artikelWahl" @close="artikelWahl = false" @waehlen="artikel" />
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="emit('close')">Schließen</button>
      <template v-if="!gesperrt">
        <button class="btn btn-outline" @click="speichern(false)">Speichern</button>
        <button class="btn btn-primary" @click="speichern(true)">EP übernehmen</button>
      </template>
    </template>
  </Dialog>
</template>
