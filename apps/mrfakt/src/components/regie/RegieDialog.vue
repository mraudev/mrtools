<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Eye, FileText, Plus, Trash2 } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import ZahlFeld from "../ZahlFeld.vue";
import PdfVorschau from "../PdfVorschau.vue";
import { api } from "@/lib/api";
import { datum, heute } from "@/lib/format";
import { frage, oeffneBeleg } from "@/lib/nav";
import type { BelegKurz, Mitarbeiter, RegieVoll, RegieZeile } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ projektId: number }>();
const emit = defineEmits<{ close: [] }>();

const offen = ref(true);
const berichte = ref<RegieVoll[]>([]);
const aktiv = ref<RegieVoll | null>(null);
const auswahl = ref<Set<number>>(new Set());
const mitarbeiter = ref<Mitarbeiter[]>([]);
const belege = ref<BelegKurz[]>([]);
const vorschau = ref<number[] | null>(null);

async function laden() {
  try {
    berichte.value = await api.regie(props.projektId);
    mitarbeiter.value = await api.mitarbeiter();
    belege.value = await api.belegeProjekt(props.projektId);
  } catch (e) {
    toastError("Regieberichte konnten nicht geladen werden", e);
  }
}
onMounted(laden);

const artText: Record<string, string> = { T: "Text", L: "Lohn (Std.)", M: "Material", G: "Gerät", P: "Menge zu LV-Pos." };

function neu() {
  aktiv.value = {
    regie: { id: 0, projekt_id: props.projektId, datum: heute(), titel: "", notiz: "", beleg_id: belege.value.find((b) => b.art === "auftrag" || b.art === "angebot")?.id ?? null, abgerechnet_id: null },
    zeilen: [{ id: 0, regie_id: 0, pos: 0, art: "T", posnr: "", text: "", menge: 0, einheit: "", ep: 0, mitarbeiter_id: null, artikel_id: null }],
  };
}

function zeile(art: RegieZeile["art"]) {
  aktiv.value?.zeilen.push({ id: 0, regie_id: aktiv.value.regie.id, pos: 0, art, posnr: "", text: "", menge: art === "T" ? 0 : 1, einheit: art === "L" ? "Std." : "", ep: 0, mitarbeiter_id: null, artikel_id: null });
}

async function speichern() {
  if (!aktiv.value) return;
  try {
    await api.regieSpeichern(aktiv.value);
    aktiv.value = null;
    await laden();
  } catch (e) {
    toastError("Speichern fehlgeschlagen", e);
  }
}

async function loeschen(r: RegieVoll) {
  if (!(await frage("Regiebericht löschen", `Bericht vom ${datum(r.regie.datum)} löschen?`, "Löschen"))) return;
  await api.regieLoeschen(r.regie.id);
  await laden();
}

async function abrechnen() {
  try {
    const id = await api.regieAbrechnen(props.projektId, [...auswahl.value]);
    toast("success", "Regierechnung erstellt");
    emit("close");
    await oeffneBeleg(id);
  } catch (e) {
    toastError("Abrechnung fehlgeschlagen", e);
  }
}

const offeneAuswahl = computed(() => [...auswahl.value]);
function stunden(r: RegieVoll) {
  return r.zeilen.filter((z) => z.art === "L").reduce((s, z) => s + z.menge, 0);
}
</script>

<template>
  <Dialog v-model:open="offen" title="Regieberichte" size="xl" @update:open="(v: boolean) => !v && emit('close')">
    <div v-if="!aktiv" class="flex flex-col gap-2">
      <div class="flex gap-2">
        <button class="btn btn-primary h-8" @click="neu"><Plus />Neuer Bericht</button>
        <div class="flex-1" />
        <button class="btn btn-outline h-8" :disabled="!auswahl.size" @click="vorschau = offeneAuswahl"><Eye />Drucken</button>
        <button class="btn btn-outline h-8" :disabled="!auswahl.size" @click="abrechnen"><FileText />Als Rechnung abrechnen</button>
      </div>
      <div class="max-h-[60vh] overflow-y-auto rounded-md border border-border">
        <div v-for="r in berichte" :key="r.regie.id" class="flex items-center gap-3 border-b border-border/60 px-3 py-2">
          <input type="checkbox" :checked="auswahl.has(r.regie.id)" @change="auswahl.has(r.regie.id) ? auswahl.delete(r.regie.id) : auswahl.add(r.regie.id)" />
          <button class="flex min-w-0 flex-1 items-baseline gap-3 text-left" @click="aktiv = JSON.parse(JSON.stringify(r))">
            <span class="w-20 shrink-0 text-xs tabular-nums">{{ datum(r.regie.datum) }}</span>
            <span class="text-[13px] font-medium">{{ r.regie.titel }}</span>
            <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">{{ r.zeilen.filter((z) => z.art === "T").map((z) => z.text).join(" ") }}</span>
          </button>
          <span v-if="stunden(r)" class="text-xs tabular-nums">{{ stunden(r) }} Std.</span>
          <span v-if="r.regie.abgerechnet_id" class="rounded bg-emerald-500/12 px-1.5 text-[10px] text-emerald-700 dark:text-emerald-300">abgerechnet</span>
          <button class="icon-btn" @click="loeschen(r)"><Trash2 /></button>
        </div>
        <p v-if="!berichte.length" class="p-6 text-center text-sm text-muted-foreground">Noch keine Regieberichte.</p>
      </div>
    </div>
    <div v-else class="flex flex-col gap-3">
      <div class="grid grid-cols-4 gap-3">
        <label><span class="label">Datum</span><input v-model="aktiv.regie.datum" type="date" class="input" /></label>
        <label><span class="label">Titel (z. B. Wochentag)</span><input v-model="aktiv.regie.titel" class="input" /></label>
        <label class="col-span-2">
          <span class="label">Zugehöriges LV (für Mengen zu Positionen)</span>
          <select v-model="aktiv.regie.beleg_id" class="input">
            <option :value="null">–</option>
            <option v-for="b in belege" :key="b.id" :value="b.id">{{ b.titel }} {{ b.nummer }} ({{ datum(b.datum) }})</option>
          </select>
        </label>
      </div>
      <div class="max-h-[45vh] overflow-y-auto">
        <div v-for="(z, i) in aktiv.zeilen" :key="i" class="grid grid-cols-[8rem_5rem_1fr_6rem_5rem_6rem_10rem_2rem] items-center gap-1 py-0.5">
          <select v-model="z.art" class="input h-7 px-1 text-xs">
            <option v-for="(t, k) in artText" :key="k" :value="k">{{ t }}</option>
          </select>
          <input v-model="z.posnr" class="input h-7 text-xs" placeholder="Pos." :disabled="z.art !== 'P'" />
          <input v-model="z.text" class="input h-7 text-xs" :placeholder="z.art === 'T' ? 'Ausgeführte Arbeiten' : 'Bezeichnung'" />
          <ZahlFeld v-model="z.menge" class="h-7 text-xs" :disabled="z.art === 'T'" />
          <input v-model="z.einheit" class="input h-7 text-xs" :disabled="z.art === 'T'" />
          <ZahlFeld v-model="z.ep" class="h-7 text-xs" :disabled="z.art === 'T' || z.art === 'P'" placeholder="Preis" />
          <select v-model="z.mitarbeiter_id" class="input h-7 px-1 text-xs" :disabled="z.art !== 'L'">
            <option :value="null">Mitarbeiter …</option>
            <option v-for="m in mitarbeiter" :key="m.id" :value="m.id">{{ m.name }}</option>
          </select>
          <button class="icon-btn size-6" @click="aktiv.zeilen.splice(i, 1)"><Trash2 /></button>
        </div>
      </div>
      <div class="flex flex-wrap gap-1">
        <button v-for="(t, k) in artText" :key="k" class="btn btn-ghost h-7 px-2 text-xs" @click="zeile(k as RegieZeile['art'])"><Plus />{{ t }}</button>
      </div>
      <label><span class="label">Notiz</span><textarea v-model="aktiv.regie.notiz" class="input h-16 py-1.5" /></label>
    </div>
    <PdfVorschau v-if="vorschau" art="regie" :id="projektId" :ids="vorschau" titel="Regiebericht" @close="vorschau = null" />
    <template #footer>
      <div class="flex-1" />
      <template v-if="aktiv">
        <button class="btn btn-outline" @click="aktiv = null">Zurück</button>
        <button class="btn btn-primary" @click="speichern">Speichern</button>
      </template>
      <button v-else class="btn btn-outline" @click="emit('close')">Schließen</button>
    </template>
  </Dialog>
</template>
