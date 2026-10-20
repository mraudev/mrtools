<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { FileDown, PackagePlus, Plus, Save, Search, ShoppingCart, Tag, Trash2, X } from "@lucide/vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import Segmented from "@mrtools/ui/components/Segmented";
import Dialog from "@mrtools/ui/components/Dialog";
import ZahlFeld from "../ZahlFeld.vue";
import { api } from "@/lib/api";
import { betrag, datum, kurztext, menge } from "@/lib/format";
import { frage } from "@/lib/nav";
import type { Artikel, LagerBewegung } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const suche = ref("");
const ansicht = ref<"alle" | "lager" | "bestellen">("alle");
const liste = ref<Artikel[]>([]);
const a = ref<Artikel | null>(null);
const original = ref("");
const bewegungen = ref<LagerBewegung[]>([]);
const auswahl = ref<Set<number>>(new Set());
const buchung = ref({ menge: 0, text: "" });
const datanorm = ref<{ pfade: string[]; lieferant: string; aufschlag: number } | null>(null);
const einheiten = ref<string[]>([]);

const geaendert = computed(() => !!a.value && JSON.stringify(a.value) !== original.value);

async function laden() {
  try {
    liste.value = ansicht.value === "bestellen" ? await api.bestellvorschlag() : await api.artikel(suche.value, ansicht.value === "lager");
  } catch (e) {
    toastError("Artikel konnten nicht geladen werden", e);
  }
}
let timer: ReturnType<typeof setTimeout> | undefined;
watch([suche, ansicht], () => {
  clearTimeout(timer);
  timer = setTimeout(laden, 150);
});
onMounted(async () => {
  laden();
  einheiten.value = (await api.einstellungen()).allgemein.einheiten;
});

async function waehlen(x: Artikel) {
  if (geaendert.value && !(await frage("Änderungen verwerfen?", "Der Artikel wurde geändert und nicht gespeichert.", "Verwerfen"))) return;
  a.value = JSON.parse(JSON.stringify(x));
  original.value = JSON.stringify(a.value);
  bewegungen.value = x.lager_aktiv ? await api.lagerBewegungen(x.id) : [];
}

function neu() {
  waehlen({
    id: 0, nummer: "", suchtext: "", text: "", gewerk: "", einheit: "St.", ek: 0, vk1: 0, vk2: 0, vk3: 0, lohn: 0, rabattgruppe: "", rabatt: 0, lieferant_id: null,
    lieferant: "", lief_nr: "", ean: "", gewicht: "", abmessung: "", warengruppe: "", preiseinheit: 1, lager_aktiv: false, bestand: 0, mindestbestand: 0, bestellmenge: 0,
    lagerplatz: "", datum_preis: "", notiz: "", quelle: "manuell", alt_id: "",
  });
}

async function speichern() {
  if (!a.value) return;
  try {
    a.value.id = await api.artikelSpeichern(a.value);
    original.value = JSON.stringify(a.value);
    toast("success", "Artikel gespeichert");
    laden();
  } catch (e) {
    toastError("Speichern fehlgeschlagen", e);
  }
}

async function loeschen() {
  if (!a.value?.id || !(await frage("Artikel löschen", `${kurztext(a.value.text)} löschen?`, "Löschen"))) return;
  await api.artikelLoeschen(a.value.id);
  a.value = null;
  laden();
}

async function buchen(vorzeichen: number) {
  if (!a.value?.id || !buchung.value.menge) return;
  try {
    await api.lagerBuchen(a.value.id, vorzeichen * Math.abs(buchung.value.menge), buchung.value.text || (vorzeichen > 0 ? "Zugang" : "Entnahme"));
    a.value.bestand += vorzeichen * Math.abs(buchung.value.menge);
    original.value = JSON.stringify(a.value);
    buchung.value = { menge: 0, text: "" };
    bewegungen.value = await api.lagerBewegungen(a.value.id);
    laden();
  } catch (e) {
    toastError("Buchung fehlgeschlagen", e);
  }
}

async function etiketten() {
  const ids = auswahl.value.size ? [...auswahl.value] : a.value?.id ? [a.value.id] : [];
  if (!ids.length) return;
  const pfad = await save({ defaultPath: "Artikeletiketten.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!pfad) return;
  try {
    await api.artikelEtiketten(ids, pfad, 3, 37);
    await api.oeffnen(pfad);
  } catch (e) {
    toastError("Etiketten konnten nicht erstellt werden", e);
  }
}

async function datanormStart() {
  const pfade = await open({ multiple: true, filters: [{ name: "DATANORM", extensions: ["*", "001", "002", "003", "dat", "txt"] }] });
  if (!pfade || (Array.isArray(pfade) && !pfade.length)) return;
  datanorm.value = { pfade: Array.isArray(pfade) ? pfade : [pfade], lieferant: "", aufschlag: 30 };
}

async function datanormLos() {
  const d = datanorm.value;
  if (!d) return;
  try {
    const e = await api.datanorm(d.pfade, d.lieferant, d.aufschlag);
    datanorm.value = null;
    toast("success", `${e.version || "DATANORM"} eingelesen`, `${e.neu} neu, ${e.geaendert} aktualisiert, ${e.preise} Preise, ${e.texte} Textzeilen`);
    laden();
  } catch (e) {
    toastError("DATANORM-Import fehlgeschlagen", e);
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-1">
    <section class="flex w-[28rem] shrink-0 flex-col border-r border-border bg-card">
      <div class="flex flex-col gap-2 border-b border-border p-3">
        <div class="flex gap-2">
          <div class="relative flex-1">
            <Search class="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
            <input v-model="suche" class="input pl-8" placeholder="Artikel suchen …" />
            <button v-if="suche" class="icon-btn absolute top-0.5 right-0.5" @click="suche = ''"><X /></button>
          </div>
          <button class="btn btn-primary px-2.5" @click="neu"><Plus />Neu</button>
        </div>
        <div class="flex items-center gap-2">
          <Segmented v-model="ansicht" :options="[{ value: 'alle', label: 'Alle' }, { value: 'lager', label: 'Lager' }, { value: 'bestellen', label: 'Bestellen', icon: ShoppingCart }]" />
          <div class="flex-1" />
          <button class="icon-btn" title="DATANORM einlesen" @click="datanormStart"><FileDown /></button>
          <button class="icon-btn" title="Etiketten drucken (Auswahl)" @click="etiketten"><Tag /></button>
        </div>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto">
        <div
          v-for="x in liste"
          :key="x.id"
          class="flex items-center gap-2 border-b border-border/60 px-3 py-1.5"
          :class="a?.id === x.id ? 'bg-accent/12' : 'hover:bg-foreground/4'"
        >
          <input type="checkbox" :checked="auswahl.has(x.id)" @change="auswahl.has(x.id) ? auswahl.delete(x.id) : auswahl.add(x.id)" />
          <button class="flex min-w-0 flex-1 items-baseline gap-2 text-left" @click="waehlen(x)">
            <span class="w-14 shrink-0 text-[11px] text-muted-foreground">{{ x.nummer }}</span>
            <span class="min-w-0 flex-1 truncate text-[13px]">{{ kurztext(x.text) || x.suchtext }}</span>
            <span v-if="x.lager_aktiv" class="text-[11px] tabular-nums" :class="x.bestand < x.mindestbestand ? 'text-red-500' : 'text-muted-foreground'">{{ menge(x.bestand) }}</span>
            <span class="w-24 shrink-0 text-right text-xs tabular-nums">{{ betrag(x.vk1) }} / {{ x.einheit }}</span>
          </button>
        </div>
        <p v-if="!liste.length" class="p-4 text-center text-xs text-muted-foreground">Keine Artikel.</p>
      </div>
      <div class="border-t border-border px-3 py-1.5 text-[11px] text-muted-foreground">{{ liste.length }} Artikel<template v-if="auswahl.size"> · {{ auswahl.size }} ausgewählt</template></div>
    </section>

    <section class="min-w-0 flex-1 overflow-y-auto">
      <div v-if="a" class="mx-auto flex max-w-4xl flex-col gap-3 p-5">
        <div class="flex items-center gap-2">
          <h1 class="flex-1 truncate text-[15px] font-semibold">{{ a.id ? kurztext(a.text) || a.suchtext : "Neuer Artikel" }}</h1>
          <button class="btn btn-primary h-8" :disabled="!geaendert" @click="speichern"><Save />Speichern</button>
          <button v-if="a.id" class="icon-btn size-8" title="Löschen" @click="loeschen"><Trash2 /></button>
        </div>
        <div class="grid grid-cols-6 gap-3 rounded-xl border border-border bg-card p-4">
          <label class="col-span-2"><span class="label">Artikel-Nr.</span><input v-model="a.nummer" class="input" /></label>
          <label class="col-span-2"><span class="label">Suchtext</span><input v-model="a.suchtext" class="input" /></label>
          <label class="col-span-2"><span class="label">Einheit</span><input v-model="a.einheit" class="input" list="artikel-einheiten" /></label>
          <datalist id="artikel-einheiten"><option v-for="e in einheiten" :key="e" :value="e" /></datalist>
          <label class="col-span-6"><span class="label">Text (erste Zeile = Kurztext, weitere = Langtext)</span><textarea v-model="a.text" class="input h-28 py-1.5" /></label>
          <label><span class="label">EK</span><ZahlFeld v-model="a.ek" /></label>
          <label><span class="label">VK 1</span><ZahlFeld v-model="a.vk1" /></label>
          <label><span class="label">VK 2</span><ZahlFeld v-model="a.vk2" /></label>
          <label><span class="label">VK 3</span><ZahlFeld v-model="a.vk3" /></label>
          <label><span class="label">Lohnanteil</span><ZahlFeld v-model="a.lohn" /></label>
          <label><span class="label">Preiseinheit</span><ZahlFeld v-model="a.preiseinheit" :stellen="0" /></label>
          <label class="col-span-2"><span class="label">Lieferant</span><input v-model="a.lieferant" class="input" /></label>
          <label class="col-span-2"><span class="label">Bestell-Nr. Lieferant</span><input v-model="a.lief_nr" class="input" /></label>
          <label class="col-span-2"><span class="label">EAN</span><input v-model="a.ean" class="input" /></label>
          <label class="col-span-2"><span class="label">Warengruppe</span><input v-model="a.warengruppe" class="input" /></label>
          <label class="col-span-2"><span class="label">Rabattgruppe</span><input v-model="a.rabattgruppe" class="input" /></label>
          <label><span class="label">Gewicht</span><input v-model="a.gewicht" class="input" /></label>
          <label><span class="label">Abmessung</span><input v-model="a.abmessung" class="input" /></label>
          <label class="col-span-6"><span class="label">Notiz</span><textarea v-model="a.notiz" class="input h-16 py-1.5" /></label>
          <p class="col-span-6 text-[11px] text-muted-foreground">Quelle: {{ a.quelle }}<template v-if="a.datum_preis"> · Preis vom {{ datum(a.datum_preis) }}</template></p>
        </div>
        <div class="rounded-xl border border-border bg-card p-4">
          <label class="flex items-center gap-2 text-[13px] font-medium"><input v-model="a.lager_aktiv" type="checkbox" />Lagerartikel</label>
          <div v-if="a.lager_aktiv" class="mt-3 grid grid-cols-4 gap-3">
            <div><span class="label">Bestand</span><div class="text-lg font-semibold tabular-nums" :class="a.bestand < a.mindestbestand ? 'text-red-500' : ''">{{ menge(a.bestand) }} {{ a.einheit }}</div></div>
            <label><span class="label">Mindestbestand</span><ZahlFeld v-model="a.mindestbestand" /></label>
            <label><span class="label">Bestellmenge</span><ZahlFeld v-model="a.bestellmenge" /></label>
            <label><span class="label">Lagerplatz</span><input v-model="a.lagerplatz" class="input" /></label>
            <template v-if="a.id">
              <label><span class="label">Menge</span><ZahlFeld v-model="buchung.menge" /></label>
              <label class="col-span-2"><span class="label">Text</span><input v-model="buchung.text" class="input" placeholder="z. B. Lieferung Großhandel" /></label>
              <div class="flex items-end gap-1">
                <button class="btn btn-outline h-8 px-2" @click="buchen(1)"><PackagePlus />Zugang</button>
                <button class="btn btn-outline h-8 px-2" @click="buchen(-1)">Entnahme</button>
              </div>
            </template>
          </div>
          <div v-if="bewegungen.length" class="mt-3 max-h-48 overflow-y-auto border-t border-border pt-2 text-xs">
            <div v-for="b in bewegungen" :key="b.id" class="flex gap-3 py-0.5">
              <span class="w-20 tabular-nums">{{ datum(b.datum) }}</span>
              <span class="w-16 text-right tabular-nums" :class="b.menge < 0 ? 'text-red-500' : 'text-emerald-600'">{{ b.menge > 0 ? "+" : "" }}{{ menge(b.menge) }}</span>
              <span class="flex-1 truncate">{{ b.text }}</span>
            </div>
          </div>
        </div>
      </div>
      <div v-else class="grid h-full place-items-center text-sm text-muted-foreground">Artikel links auswählen oder neu anlegen.</div>
    </section>

    <Dialog v-if="datanorm" :open="true" title="DATANORM einlesen" size="sm" @update:open="(v: boolean) => !v && (datanorm = null)">
      <p class="mb-3 text-xs text-muted-foreground">{{ datanorm.pfade.length }} Datei(en). Vorhandene Artikel desselben Lieferanten werden anhand der Artikelnummer aktualisiert.</p>
      <div class="grid grid-cols-2 gap-3">
        <label class="col-span-2"><span class="label">Lieferant</span><input v-model="datanorm.lieferant" class="input" placeholder="z. B. Großhandel Müller" /></label>
        <label><span class="label">Aufschlag auf EK, wenn kein VK (%)</span><ZahlFeld v-model="datanorm.aufschlag" :stellen="0" /></label>
      </div>
      <template #footer>
        <div class="flex-1" />
        <button class="btn btn-outline" @click="datanorm = null">Abbrechen</button>
        <button class="btn btn-primary" :disabled="!datanorm.lieferant.trim()" @click="datanormLos">Einlesen</button>
      </template>
    </Dialog>
  </div>
</template>
