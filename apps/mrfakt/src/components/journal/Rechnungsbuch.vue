<script setup lang="ts">
import { onMounted, reactive, ref, watch } from "vue";
import { BellRing, FileSpreadsheet, HandCoins, Search, X } from "@lucide/vue";
import { save } from "@tauri-apps/plugin-dialog";
import Segmented from "@mrtools/ui/components/Segmented";
import Dialog from "@mrtools/ui/components/Dialog";
import ZahlFeld from "../ZahlFeld.vue";
import PdfVorschau from "../PdfVorschau.vue";
import BelegBadge from "../beleg/BelegBadge.vue";
import { api } from "@/lib/api";
import { artTitel, betrag, datum, heute } from "@/lib/format";
import { nav, neuLaden, oeffneBeleg, oeffneKunde } from "@/lib/nav";
import type { JournalFilter, JournalListe, JournalZeile } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const filter = reactive<JournalFilter>({ jahr: String(new Date().getFullYear()), art: "", status: "alle", suche: "" });
const liste = ref<JournalListe>({ zeilen: [], netto: 0, mwst: 0, betrag: 0, offen: 0, jahre: [] });
const laedt = ref(false);
let erstesLaden = false;
const zahlung = ref<{ zeile: JournalZeile; datum: string; betrag: number; abzug: number; notiz: string } | null>(null);
const mahnung = ref<number | null>(null);

async function laden() {
  laedt.value = true;
  try {
    liste.value = await api.rechnungsbuch({ ...filter });
    // Im neuen Jahr noch nichts gebucht: mit dem letzten Jahr mit Belegen beginnen
    if (!erstesLaden && filter.jahr && !liste.value.jahre.includes(filter.jahr) && liste.value.jahre.length) {
      erstesLaden = true;
      filter.jahr = liste.value.jahre[0];
      return;
    }
    erstesLaden = true;
  } catch (e) {
    toastError("Rechnungsbuch konnte nicht geladen werden", e);
  } finally {
    laedt.value = false;
  }
}
let timer: ReturnType<typeof setTimeout> | undefined;
watch(filter, () => {
  clearTimeout(timer);
  timer = setTimeout(laden, 150);
});
watch(() => nav.version, laden);
onMounted(laden);

async function oeffnen(z: JournalZeile) {
  await oeffneKunde(z.kunde_id, z.projekt_id);
  await oeffneBeleg(z.id);
}

function zahlungErfassen(z: JournalZeile) {
  zahlung.value = { zeile: z, datum: heute(), betrag: z.offen, abzug: 0, notiz: "" };
}

async function buchen() {
  const zl = zahlung.value;
  if (!zl) return;
  try {
    await api.zahlungBuchen({ id: 0, beleg_id: zl.zeile.id, datum: zl.datum, betrag: zl.betrag, abzug: zl.abzug, notiz: zl.notiz });
    zahlung.value = null;
    neuLaden();
    toast("success", "Zahlung gebucht");
  } catch (e) {
    toastError("Zahlung konnte nicht gebucht werden", e);
  }
}

async function mahnen(z: JournalZeile) {
  try {
    const m = await api.mahnen(z.id);
    neuLaden();
    mahnung.value = m.id;
  } catch (e) {
    toastError("Mahnung nicht möglich", e);
  }
}

async function csv() {
  const pfad = await save({ defaultPath: `Rechnungsbuch ${filter.jahr || "alle"}.csv`, filters: [{ name: "CSV", extensions: ["csv"] }] });
  if (!pfad) return;
  try {
    await api.rechnungsbuchCsv({ ...filter }, pfad);
    toast("success", "Exportiert", pfad);
  } catch (e) {
    toastError("Export fehlgeschlagen", e);
  }
}

const stufen = ["", "1. Mahnung", "2. Mahnung", "3. Mahnung", "Stufe 4"];
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="flex flex-wrap items-center gap-2 border-b border-border bg-card px-4 py-2.5">
      <h1 class="mr-2 text-[15px] font-semibold">Rechnungsbuch</h1>
      <select v-model="filter.jahr" class="input h-8 w-28">
        <option value="">Alle Jahre</option>
        <option v-for="j in liste.jahre" :key="j" :value="j">{{ j }}</option>
      </select>
      <select v-model="filter.art" class="input h-8 w-48">
        <option value="">Alle Rechnungen</option>
        <option value="rechnung">Rechnungen</option>
        <option value="abschlag">Abschlagsrechnungen</option>
        <option value="schluss">Schlussrechnungen</option>
        <option value="gutschrift">Gutschriften</option>
        <option value="angebot">Angebote</option>
        <option value="auftrag">Auftragsbestätigungen</option>
        <option value="lieferschein">Lieferscheine</option>
        <option value="alle">Alle Belege</option>
      </select>
      <Segmented
        v-model="filter.status"
        :options="[{ value: 'alle', label: 'Alle' }, { value: 'offen', label: 'Offen' }, { value: 'faellig', label: 'Überfällig' }, { value: 'bezahlt', label: 'Bezahlt' }]"
      />
      <div class="relative w-64">
        <Search class="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
        <input v-model="filter.suche" class="input h-8 pl-8" placeholder="Nummer, Kunde, Projekt …" />
        <button v-if="filter.suche" class="icon-btn absolute top-0.5 right-0.5" @click="filter.suche = ''"><X /></button>
      </div>
      <div class="flex-1" />
      <button class="btn btn-outline h-8" @click="csv"><FileSpreadsheet />CSV</button>
    </div>
    <div class="min-h-0 flex-1 overflow-auto">
      <table class="w-full text-[13px]">
        <thead class="sticky top-0 z-10 bg-background text-[11px] text-muted-foreground">
          <tr class="border-b border-border">
            <th class="px-3 py-2 text-left font-medium">Datum</th>
            <th class="px-2 py-2 text-left font-medium">Nummer</th>
            <th class="px-2 py-2 text-left font-medium">Beleg</th>
            <th class="px-2 py-2 text-left font-medium">Kunde / Projekt</th>
            <th class="px-2 py-2 text-right font-medium">Netto</th>
            <th class="px-2 py-2 text-right font-medium">Betrag</th>
            <th class="px-2 py-2 text-right font-medium">Offen</th>
            <th class="px-2 py-2 text-left font-medium">Fällig / bezahlt</th>
            <th class="px-2 py-2" />
          </tr>
        </thead>
        <tbody>
          <tr v-for="z in liste.zeilen" :key="z.id" class="group cursor-default border-b border-border/50 hover:bg-foreground/4" @dblclick="oeffnen(z)">
            <td class="px-3 py-1.5 tabular-nums">{{ datum(z.datum) }}</td>
            <td class="px-2 py-1.5 tabular-nums">{{ z.nummer }}</td>
            <td class="px-2 py-1.5">
              <button class="text-left hover:underline" @click="oeffnen(z)">{{ z.titel || artTitel[z.art] }}</button>
              <BelegBadge v-if="z.status === 'storniert'" status="storniert" class="ml-1" />
            </td>
            <td class="max-w-72 px-2 py-1.5">
              <div class="truncate">{{ z.kunde }}</div>
              <div class="truncate text-[11px] text-muted-foreground">{{ z.projekt }}</div>
            </td>
            <td class="px-2 py-1.5 text-right tabular-nums">{{ betrag(z.netto) }}</td>
            <td class="px-2 py-1.5 text-right tabular-nums">{{ betrag(z.betrag) }}<span v-if="z.waehrung !== 'EUR'" class="text-[10px] text-muted-foreground"> {{ z.waehrung }}</span></td>
            <td class="px-2 py-1.5 text-right font-medium tabular-nums" :class="z.ueberfaellig ? 'text-red-600 dark:text-red-400' : z.offen ? 'text-amber-600 dark:text-amber-400' : ''">
              {{ z.offen ? betrag(z.offen) : "" }}
            </td>
            <td class="px-2 py-1.5 text-xs whitespace-nowrap text-muted-foreground">
              <template v-if="z.offen">fällig {{ datum(z.faellig) }}<span v-if="z.mahnstufe" class="ml-1 rounded bg-red-500/12 px-1 text-red-600">{{ stufen[z.mahnstufe] ?? z.mahnstufe }}</span></template>
              <template v-else-if="z.bezahlt_am">bezahlt {{ datum(z.bezahlt_am) }}</template>
            </td>
            <td class="px-2 py-1 whitespace-nowrap">
              <span v-if="z.offen > 0" class="flex justify-end gap-0.5 opacity-0 group-hover:opacity-100">
                <button class="icon-btn" title="Zahlung erfassen" @click="zahlungErfassen(z)"><HandCoins /></button>
                <button class="icon-btn" title="Mahnen (nächste Stufe)" @click="mahnen(z)"><BellRing /></button>
              </span>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-if="!liste.zeilen.length && !laedt" class="p-8 text-center text-sm text-muted-foreground">Keine Belege für diese Auswahl.</p>
    </div>
    <div class="flex gap-6 border-t border-border bg-card px-4 py-2 text-[13px] tabular-nums">
      <span class="text-muted-foreground">{{ liste.zeilen.length.toLocaleString("de-DE") }} Belege</span>
      <div class="flex-1" />
      <span>Netto <strong>{{ betrag(liste.netto) }}</strong></span>
      <span>MwSt. <strong>{{ betrag(liste.mwst) }}</strong></span>
      <span>Brutto <strong>{{ betrag(liste.betrag) }}</strong></span>
      <span class="text-amber-600 dark:text-amber-400">Offen <strong>{{ betrag(liste.offen) }}</strong></span>
    </div>

    <Dialog v-if="zahlung" :open="true" :title="`Zahlung – ${zahlung.zeile.titel} ${zahlung.zeile.nummer}`" size="sm" @update:open="(v: boolean) => !v && (zahlung = null)">
      <div class="grid grid-cols-2 gap-3">
        <label><span class="label">Datum</span><input v-model="zahlung.datum" type="date" class="input" /></label>
        <label><span class="label">Betrag</span><ZahlFeld v-model="zahlung.betrag" @enter="buchen" /></label>
        <label><span class="label">Skonto / Abzug</span><ZahlFeld v-model="zahlung.abzug" /></label>
        <label><span class="label">Notiz</span><input v-model="zahlung.notiz" class="input" /></label>
      </div>
      <p class="mt-2 text-xs text-muted-foreground">Offen: {{ betrag(zahlung.zeile.offen) }} €. Ein Abzug (Skonto, Einbehalt) gilt als ausgeglichen.</p>
      <template #footer>
        <div class="flex-1" />
        <button class="btn btn-outline" @click="zahlung = null">Abbrechen</button>
        <button class="btn btn-primary" @click="buchen">Buchen</button>
      </template>
    </Dialog>
    <PdfVorschau v-if="mahnung" art="mahnung" :id="mahnung" titel="Mahnung" @close="mahnung = null" />
  </div>
</template>
