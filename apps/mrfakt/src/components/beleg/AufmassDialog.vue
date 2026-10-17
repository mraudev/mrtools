<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Minus, Plus, Trash2 } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import ZahlFeld from "../ZahlFeld.vue";
import { api } from "@/lib/api";
import { kurztext, menge } from "@/lib/format";
import type { Aufmass, BelegVoll, Formel } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ beleg: BelegVoll }>();
const emit = defineEmits<{ close: []; gespeichert: [] }>();

const offen = ref(true);
const zeilen = ref<Aufmass[]>([]);
const formeln = ref<Formel[]>([]);
const gesperrt = ref(false);
const posnr = ref("");
const hinweise = ref<string[]>([]);

const positionen = computed(() => props.beleg.positionen.filter((p) => ["N", "A", "B"].includes(p.art)));
const aktuell = computed(() => zeilen.value.filter((z) => z.posnr === posnr.value));
const summe = (nr: string) => zeilen.value.filter((z) => z.posnr === nr && z.art === "M").reduce((s, z) => s + z.ergebnis, 0);

onMounted(async () => {
  try {
    const d = await api.aufmass(props.beleg.beleg.id);
    zeilen.value = d.zeilen;
    formeln.value = d.formeln;
    gesperrt.value = d.gesperrt;
    posnr.value = positionen.value.find((p) => p.posnr)?.posnr ?? "";
  } catch (e) {
    toastError("Aufmaß konnte nicht geladen werden", e);
  }
});

function neu(art: "M" | "K" = "M") {
  const z: Aufmass = { id: 0, beleg_id: props.beleg.beleg.id, pos: 0, posnr: posnr.value, art, text: "", anzahl: 1, formel: 0, m1: "", m2: "", m3: "", ergebnis: 0, abzug: false };
  // hinter der letzten Zeile dieser Position einfügen
  const letzte = zeilen.value.map((x) => x.posnr).lastIndexOf(posnr.value);
  zeilen.value.splice(letzte < 0 ? zeilen.value.length : letzte + 1, 0, z);
}

let timer: ReturnType<typeof setTimeout> | undefined;
function rechnen(z: Aufmass) {
  clearTimeout(timer);
  timer = setTimeout(async () => {
    try {
      z.ergebnis = await api.aufmassRechnen(z);
    } catch {
      // ungültige Maße – wird beim Speichern gemeldet
    }
  }, 200);
}

async function speichern(uebernehmen: boolean) {
  try {
    hinweise.value = await api.aufmassSpeichern(props.beleg.beleg.id, zeilen.value, uebernehmen);
    if (hinweise.value.length) return;
    toast("success", uebernehmen ? "Aufmaß gespeichert und Mengen übernommen" : "Aufmaß gespeichert");
    emit("gespeichert");
  } catch (e) {
    toastError("Aufmaß", e);
  }
}

function formelHinweis(nr: number) {
  return formeln.value.find((f) => f.nummer === nr)?.hinweis ?? "Maß 1 × Maß 2 × Maß 3";
}
</script>

<template>
  <Dialog v-model:open="offen" :title="`Aufmaß – ${beleg.beleg.titel} ${beleg.beleg.nummer}`" size="xl" @update:open="(v: boolean) => !v && emit('close')">
    <div class="grid h-[65vh] grid-cols-[16rem_1fr] gap-3">
      <div class="overflow-y-auto rounded-md border border-border">
        <button
          v-for="p in positionen"
          :key="p.id"
          class="flex w-full flex-col border-b border-border/60 px-2 py-1.5 text-left text-xs"
          :class="posnr === p.posnr ? 'bg-accent/12' : 'hover:bg-foreground/4'"
          :disabled="!p.posnr"
          :title="p.posnr ? '' : 'Position ohne Nummer – für das Aufmaß bitte eine Positionsnummer vergeben'"
          @click="posnr = p.posnr"
        >
          <span class="flex gap-2"><span class="w-10 shrink-0 font-medium">{{ p.posnr || "–" }}</span><span class="truncate">{{ kurztext(p.text) }}</span></span>
          <span class="flex justify-end gap-2 text-[11px] text-muted-foreground tabular-nums">
            <span v-if="summe(p.posnr)">Aufmaß {{ menge(summe(p.posnr)) }}</span>
            <span>LV {{ menge(p.menge) }} {{ p.einheit }}</span>
          </span>
        </button>
      </div>
      <div class="flex min-h-0 flex-col">
        <div class="grid grid-cols-[1fr_4rem_9rem_5.5rem_5.5rem_5.5rem_6rem_3.5rem] gap-1 px-1 pb-1 text-[11px] font-medium text-muted-foreground">
          <span>Bezeichnung / Raum</span><span class="text-right">Anzahl</span><span>Formel</span><span>Maß 1</span><span>Maß 2</span><span>Maß 3</span><span class="text-right">Ergebnis</span><span />
        </div>
        <div class="min-h-0 flex-1 overflow-y-auto">
          <div v-for="(z, i) in aktuell" :key="i" class="grid grid-cols-[1fr_4rem_9rem_5.5rem_5.5rem_5.5rem_6rem_3.5rem] items-center gap-1 px-1 py-0.5">
            <input v-model="z.text" class="input h-7 text-xs" :class="z.art !== 'M' ? 'col-span-7 italic' : ''" :disabled="gesperrt" :placeholder="z.art === 'K' ? 'Kommentar' : 'z. B. Wohnzimmer'" />
            <template v-if="z.art === 'M'">
              <ZahlFeld v-model="z.anzahl" class="h-7 text-xs" :stellen="0" :disabled="gesperrt" @update:model-value="rechnen(z)" />
              <select v-model.number="z.formel" class="input h-7 px-1 text-xs" :disabled="gesperrt" :title="formelHinweis(z.formel)" @change="rechnen(z)">
                <option v-for="f in formeln" :key="f.nummer" :value="f.nummer">{{ f.name }}</option>
                <option v-if="!formeln.length" :value="0">Produkt</option>
              </select>
              <input v-model="z.m1" class="input h-7 text-right text-xs tabular-nums" :disabled="gesperrt" @input="rechnen(z)" />
              <input v-model="z.m2" class="input h-7 text-right text-xs tabular-nums" :disabled="gesperrt" @input="rechnen(z)" />
              <input v-model="z.m3" class="input h-7 text-right text-xs tabular-nums" :disabled="gesperrt" @input="rechnen(z)" />
              <span class="text-right text-xs tabular-nums" :class="z.abzug ? 'text-red-500' : ''">{{ menge(z.ergebnis) }}</span>
            </template>
            <span class="flex justify-end">
              <button v-if="z.art === 'M' && !gesperrt" class="icon-btn size-6" :class="z.abzug ? 'text-red-500' : ''" title="Abzug (z. B. Fenster, Türen)" @click="z.abzug = !z.abzug; rechnen(z)"><Minus /></button>
              <button v-if="!gesperrt" class="icon-btn size-6" @click="zeilen.splice(zeilen.indexOf(z), 1)"><Trash2 /></button>
            </span>
          </div>
          <p v-if="!aktuell.length" class="p-4 text-center text-xs text-muted-foreground">Noch keine Maße für diese Position. Maße dürfen Rechnungen enthalten, z. B. „4,5+2,25“.</p>
        </div>
        <div class="flex items-center gap-2 border-t border-border pt-2">
          <button v-if="!gesperrt" class="btn btn-ghost h-7 px-2 text-xs" :disabled="!posnr" @click="neu('M')"><Plus />Maßzeile</button>
          <button v-if="!gesperrt" class="btn btn-ghost h-7 px-2 text-xs" :disabled="!posnr" @click="neu('K')"><Plus />Kommentar</button>
          <div class="flex-1" />
          <span class="text-sm font-medium tabular-nums">Summe Pos. {{ posnr }}: {{ menge(summe(posnr)) }}</span>
        </div>
        <p v-for="h in hinweise" :key="h" class="text-xs text-red-500">{{ h }}</p>
      </div>
    </div>
    <template #footer>
      <span v-if="gesperrt" class="text-xs text-muted-foreground">Beleg festgeschrieben – Aufmaß nur lesbar.</span>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="emit('close')">Schließen</button>
      <template v-if="!gesperrt">
        <button class="btn btn-outline" @click="speichern(false)">Speichern</button>
        <button class="btn btn-primary" @click="speichern(true)">Speichern & Mengen übernehmen</button>
      </template>
    </template>
  </Dialog>
</template>
