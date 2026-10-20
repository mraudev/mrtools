<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "reka-ui";
import {
  ArrowLeft, Ban, Eye, EllipsisVertical, FileCode2, FileOutput, Lock, Printer, Repeat2, Ruler, Save, ScrollText, Trash2,
} from "@lucide/vue";
import { save } from "@tauri-apps/plugin-dialog";
import BelegBadge from "./BelegBadge.vue";
import Positionen from "./Positionen.vue";
import SummenPanel from "./SummenPanel.vue";
import KonditionenPanel from "./KonditionenPanel.vue";
import ZahlungenPanel from "./ZahlungenPanel.vue";
import AufmassDialog from "./AufmassDialog.vue";
import KalkulationDialog from "./KalkulationDialog.vue";
import ArtikelWahl from "./ArtikelWahl.vue";
import PdfVorschau from "../PdfVorschau.vue";
import RichText from "../RichText.vue";
import TextbausteinMenu from "../TextbausteinMenu.vue";
import { api, type Druckart } from "@/lib/api";
import { artTitel, datum, istRechnung } from "@/lib/format";
import { frage, nav, neuLaden, oeffneBeleg, verlassenOk } from "@/lib/nav";
import type { Artikel, BelegVoll, Summen } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ id: number }>();

const v = ref<BelegVoll>();
const original = ref("");
const summen = ref<Summen | null>(null);
const einheiten = ref<string[]>([]);
const vorschau = ref<{ art: Druckart; id: number; titel: string } | null>(null);
const aufmass = ref(false);
const kalkIndex = ref<number | null>(null);
const artikelIndex = ref<number | null>(null);
const vortextRef = ref<InstanceType<typeof RichText>>();
const nachtextRef = ref<InstanceType<typeof RichText>>();
const arbeitet = ref(false);

const gesperrt = computed(() => v.value?.beleg.status === "festgeschrieben" || v.value?.beleg.status === "storniert");
const rechnung = computed(() => (v.value ? istRechnung(v.value.beleg.art) : false));
const geaendert = computed(() => !!v.value && JSON.stringify(stand(v.value)) !== original.value);
watch(geaendert, (g) => (nav.geaendert = g));

function stand(x: BelegVoll) {
  return { beleg: x.beleg, positionen: x.positionen, zuschlaege: x.zuschlaege, abzuege: x.abzuege };
}

function uebernehmen(x: BelegVoll) {
  v.value = x;
  summen.value = x.summen;
  original.value = JSON.stringify(stand(x));
}

async function laden() {
  try {
    uebernehmen(await api.beleg(props.id));
    const e = await api.einstellungen();
    einheiten.value = e.allgemein.einheiten;
  } catch (e) {
    toastError("Beleg konnte nicht geladen werden", e);
  }
}
onMounted(laden);

// Summen beim Bearbeiten neu berechnen (im Backend, damit sie exakt dem Druck entsprechen)
let timer: ReturnType<typeof setTimeout> | undefined;
watch(
  () => v.value && JSON.stringify(stand(v.value)),
  () => {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      if (!v.value) return;
      try {
        summen.value = await api.belegSummen(v.value);
      } catch {
        // Eingaben unvollständig – letzte Summen bleiben stehen
      }
    }, 250);
  },
);

async function speichern(stumm = false): Promise<boolean> {
  if (!v.value) return false;
  if (!geaendert.value) return true;
  arbeitet.value = true;
  try {
    uebernehmen(await api.belegSpeichern(v.value));
    nav.geaendert = false;
    neuLaden();
    if (!stumm) toast("success", "Gespeichert");
    return true;
  } catch (e) {
    toastError("Speichern fehlgeschlagen", e);
    return false;
  } finally {
    arbeitet.value = false;
  }
}

async function schliessen() {
  if (await verlassenOk()) nav.belegId = 0;
}

async function zeigen(art: Druckart = "beleg") {
  if (!(await speichern(true)) || !v.value) return;
  vorschau.value = { art, id: art === "beleg" || art === "aufmass" ? v.value.beleg.id : v.value.beleg.id, titel: `${v.value.beleg.titel} ${v.value.beleg.nummer}` };
}

/** Drucken: Rechnungen werden dabei festgeschrieben (Nummer, keine Änderungen mehr), andere Belege als gedruckt markiert. */
async function drucken() {
  if (!v.value || !(await speichern(true))) return;
  const b = v.value.beleg;
  if (b.status === "entwurf" || (!rechnung.value && b.status !== "storniert")) {
    if (rechnung.value) {
      const ok = await frage(
        `${b.titel} festschreiben`,
        "Beim Drucken erhält die Rechnung ihre endgültige Nummer und kann danach nicht mehr geändert werden (Korrekturen nur per Storno/Gutschrift).",
        "Festschreiben und drucken",
        false,
      );
      if (!ok) return;
    }
    try {
      uebernehmen(await api.belegFestschreiben(b.id));
      neuLaden();
    } catch (e) {
      toastError("Festschreiben fehlgeschlagen", e);
      return;
    }
  }
  try {
    const pfad = await api.pdfSpeichern("beleg", b.id, { oeffnen: true });
    toast("success", "PDF gespeichert", pfad);
  } catch (e) {
    toastError("PDF konnte nicht erstellt werden", e);
  }
}

async function pdfUnter() {
  if (!v.value || !(await speichern(true))) return;
  const b = v.value.beleg;
  const pfad = await save({ defaultPath: `${b.titel} ${b.nummer}.pdf`.trim(), filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!pfad) return;
  try {
    await api.pdfSpeichern("beleg", b.id, { pfad, oeffnen: true });
  } catch (e) {
    toastError("PDF konnte nicht erstellt werden", e);
  }
}

async function umwandeln(art: string) {
  if (!v.value || !(await speichern(true))) return;
  try {
    const neu = await api.belegUmwandeln(v.value.beleg.id, art);
    nav.geaendert = false;
    neuLaden();
    toast("success", `${artTitel[art]} erstellt`);
    await oeffneBeleg(neu.beleg.id);
  } catch (e) {
    toastError("Umwandeln fehlgeschlagen", e);
  }
}

async function festschreiben() {
  if (!v.value || !(await speichern(true))) return;
  if (!(await frage("Festschreiben", "Der Beleg erhält seine Nummer und kann danach nicht mehr geändert werden.", "Festschreiben", false))) return;
  try {
    uebernehmen(await api.belegFestschreiben(v.value.beleg.id));
    neuLaden();
  } catch (e) {
    toastError("Festschreiben fehlgeschlagen", e);
  }
}

async function stornieren() {
  if (!v.value) return;
  if (!(await frage("Stornieren", `${v.value.beleg.titel} ${v.value.beleg.nummer} stornieren? Es wird eine Stornorechnung mit negativen Beträgen erstellt.`, "Stornieren"))) return;
  try {
    const s = await api.belegStornieren(v.value.beleg.id);
    neuLaden();
    await oeffneBeleg(s.beleg.id);
  } catch (e) {
    toastError("Stornieren fehlgeschlagen", e);
  }
}

async function loeschen() {
  if (!v.value) return;
  if (!(await frage("Beleg löschen", `${v.value.beleg.titel} ${v.value.beleg.nummer} löschen?`, "Löschen"))) return;
  try {
    await api.belegLoeschen(v.value.beleg.id);
    nav.geaendert = false;
    nav.belegId = 0;
    neuLaden();
  } catch (e) {
    toastError("Löschen nicht möglich", e);
  }
}

async function exportieren(art: "xrechnung" | "gaeb") {
  if (!v.value || !(await speichern(true))) return;
  const b = v.value.beleg;
  const pfad = await save({
    defaultPath: art === "gaeb" ? `${b.titel} ${b.nummer}.x84` : `${b.titel} ${b.nummer}.xml`,
    filters: art === "gaeb" ? [{ name: "GAEB DA XML", extensions: ["x84"] }] : [{ name: "XML", extensions: ["xml"] }],
  });
  if (!pfad) return;
  try {
    if (art === "gaeb") await api.gaebExport(b.id, pfad);
    else await api.erechnungXml(b.id, pfad);
    toast("success", "Exportiert", pfad);
  } catch (e) {
    toastError("Export fehlgeschlagen", e);
  }
}

function artikelUebernehmen(a: Artikel) {
  if (!v.value || artikelIndex.value === null) return;
  const i = artikelIndex.value;
  const p = {
    id: 0, beleg_id: v.value.beleg.id, pos: 0, art: "N" as const, posnr: "", text: a.text, menge: 1, einheit: a.einheit, ep: a.vk1, rabatt: 0, gp: 0,
    lohn: a.lohn, artikel_id: a.id, menge_aus_aufmass: false,
  };
  const vorhanden = v.value.positionen[i];
  if (vorhanden && !vorhanden.text.trim() && vorhanden.ep === 0) v.value.positionen.splice(i, 1, { ...p, posnr: vorhanden.posnr, menge: vorhanden.menge || 1 });
  else v.value.positionen.splice(i, 0, p);
  artikelIndex.value = null;
}

async function nachAufmass() {
  aufmass.value = false;
  await laden();
}

async function nachKalkulation() {
  kalkIndex.value = null;
  await laden();
}

async function kalkulationOeffnen(i: number) {
  if (!v.value) return;
  if (!v.value.positionen[i].id || geaendert.value) {
    if (!(await speichern(true))) return;
  }
  kalkIndex.value = i;
}

async function aufmassOeffnen() {
  if (geaendert.value && !(await speichern(true))) return;
  aufmass.value = true;
}

function tasten(e: KeyboardEvent) {
  if (e.ctrlKey && e.key.toLowerCase() === "s") {
    e.preventDefault();
    speichern();
  } else if (e.ctrlKey && e.key.toLowerCase() === "p") {
    e.preventDefault();
    drucken();
  }
}
onMounted(() => window.addEventListener("keydown", tasten));
onUnmounted(() => {
  window.removeEventListener("keydown", tasten);
  nav.geaendert = false;
});

const umwandelnZiele = computed(() => {
  const art = v.value?.beleg.art;
  const alle = ["angebot", "auftrag", "lieferschein", "abschlag", "schluss", "rechnung", "gutschrift"];
  return alle.filter((a) => a !== art);
});
</script>

<template>
  <div v-if="v" class="flex min-h-0 flex-1 flex-col">
    <!-- Werkzeugleiste -->
    <div class="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-card px-3">
      <button class="icon-btn" aria-label="Zurück" title="Zurück (Esc)" @click="schliessen"><ArrowLeft /></button>
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <span class="truncate text-[14px] font-semibold">{{ v.beleg.titel }} {{ v.beleg.nummer }}</span>
          <BelegBadge :status="v.beleg.status" />
          <span v-if="geaendert" class="text-[11px] text-amber-600 dark:text-amber-400">● geändert</span>
        </div>
        <div class="truncate text-[11px] text-muted-foreground">
          {{ v.kunde?.name1 || v.kunde?.suchname }} · Projekt {{ v.projekt?.nummer }} {{ v.projekt?.bezeichnung }}
        </div>
      </div>
      <div class="flex-1" />
      <button v-if="!gesperrt" class="btn btn-outline h-8" :disabled="!geaendert || arbeitet" title="Speichern (Strg+S)" @click="speichern()"><Save />Speichern</button>
      <button class="btn btn-outline h-8" @click="zeigen()"><Eye />Vorschau</button>
      <button class="btn btn-primary h-8" title="Drucken (Strg+P)" @click="drucken"><Printer />{{ rechnung && v.beleg.status === "entwurf" ? "Festschreiben & drucken" : "Drucken" }}</button>
      <DropdownMenuRoot>
        <DropdownMenuTrigger as-child>
          <button class="btn btn-outline h-8"><Repeat2 />Weiterführen</button>
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent align="end" :side-offset="6" class="menu anim-fade w-60">
            <DropdownMenuLabel class="px-2 py-1 text-[11px] text-muted-foreground">Neuen Beleg aus diesem erstellen</DropdownMenuLabel>
            <DropdownMenuItem v-for="a in umwandelnZiele" :key="a" class="menu-item" @select="umwandeln(a)">{{ artTitel[a] }}</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
      <DropdownMenuRoot>
        <DropdownMenuTrigger as-child>
          <button class="icon-btn size-8" aria-label="Weitere Aktionen"><EllipsisVertical /></button>
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent align="end" :side-offset="6" class="menu anim-fade w-64">
            <DropdownMenuItem class="menu-item" @select="aufmassOeffnen"><Ruler />Aufmaß {{ v.aufmass ? `(${v.aufmass} Zeilen)` : "" }}</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" :disabled="!v.aufmass" @select="zeigen('aufmass')"><ScrollText />Aufmaßblatt drucken</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" @select="pdfUnter"><FileOutput />PDF speichern unter …</DropdownMenuItem>
            <DropdownMenuSeparator class="menu-separator" />
            <DropdownMenuItem v-if="rechnung" class="menu-item" :disabled="v.beleg.status === 'entwurf'" @select="exportieren('xrechnung')"><FileCode2 />E-Rechnung (XML) exportieren …</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" @select="exportieren('gaeb')"><FileCode2 />GAEB X84 exportieren …</DropdownMenuItem>
            <DropdownMenuSeparator class="menu-separator" />
            <DropdownMenuItem v-if="!gesperrt" class="menu-item" @select="festschreiben"><Lock />Festschreiben</DropdownMenuItem>
            <DropdownMenuItem v-if="rechnung && v.beleg.status === 'festgeschrieben'" class="menu-item text-red-500" @select="stornieren"><Ban />Stornieren …</DropdownMenuItem>
            <DropdownMenuItem v-if="!gesperrt" class="menu-item text-red-500" @select="loeschen"><Trash2 />Löschen</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
    </div>

    <div v-if="gesperrt" class="flex items-center gap-2 border-b border-emerald-500/20 bg-emerald-500/8 px-4 py-1.5 text-xs text-emerald-800 dark:text-emerald-300">
      <Lock class="size-3.5" />
      {{ v.beleg.status === "storniert" ? "Storniert." : "Festgeschrieben – nur noch lesen, drucken und Zahlungen erfassen." }}
      <template v-if="v.beleg.alt_slot"> Aus GHFakt übernommen.</template>
    </div>

    <div class="flex min-h-0 flex-1">
      <!-- Hauptbereich -->
      <div class="min-w-0 flex-1 overflow-y-auto p-4">
        <div class="mx-auto flex max-w-5xl flex-col gap-3">
          <div class="grid grid-cols-6 gap-3 rounded-xl border border-border bg-card p-3">
            <label class="col-span-2"><span class="label">Bezeichnung</span><input v-model="v.beleg.titel" class="input" :disabled="gesperrt" /></label>
            <label class="col-span-1"><span class="label">Nummer</span><input v-model="v.beleg.nummer" class="input" :disabled="gesperrt" :placeholder="rechnung ? 'beim Festschreiben' : ''" /></label>
            <label class="col-span-1"><span class="label">Datum</span><input v-model="v.beleg.datum" type="date" class="input" :disabled="gesperrt" /></label>
            <label class="col-span-2"><span class="label">Leistungszeitraum / Lieferdatum</span><input v-model="v.beleg.leistung" class="input" :disabled="gesperrt" placeholder="z. B. Mai 2024" /></label>
            <label class="col-span-6"><span class="label">Betreff</span><input v-model="v.beleg.betreff" class="input" :disabled="gesperrt" /></label>
          </div>

          <div>
            <div class="mb-1 flex items-center gap-2">
              <span class="label mb-0">Vortext</span>
              <div class="flex-1" />
              <TextbausteinMenu v-if="!gesperrt" kategorie="vortext" @waehlen="(t: string) => vortextRef?.einfuegen(t)" />
            </div>
            <RichText ref="vortextRef" v-model="v.beleg.vortext" :read-only="gesperrt" placeholder="z. B. für Malerarbeiten an Ihrem Wohnhaus …" />
          </div>

          <Positionen
            v-model="v.positionen"
            :gesperrt="gesperrt"
            :einheiten="einheiten"
            :summen="summen"
            :preise="v.beleg.art !== 'lieferschein'"
            :waehrung="v.beleg.waehrung"
            @kalkulation="kalkulationOeffnen"
            @artikel="(i: number) => (artikelIndex = i)"
            @aufmass="aufmassOeffnen"
          />

          <div>
            <div class="mb-1 flex items-center gap-2">
              <span class="label mb-0">Nachtext</span>
              <div class="flex-1" />
              <TextbausteinMenu v-if="!gesperrt" kategorie="nachtext" @waehlen="(t: string) => nachtextRef?.einfuegen(t)" />
            </div>
            <RichText ref="nachtextRef" v-model="v.beleg.nachtext" :read-only="gesperrt" placeholder="z. B. Wir bedanken uns für den Auftrag …" />
          </div>
          <p class="pb-6 text-[11px] text-muted-foreground">
            Angelegt {{ datum(v.beleg.angelegt) }}<template v-if="v.beleg.gedruckt_am"> · gedruckt {{ datum(v.beleg.gedruckt_am) }}</template>
            <template v-if="v.beleg.alt_betrag"> · Betrag laut GHFakt-Rechnungsbuch {{ v.beleg.alt_betrag }}</template>
          </p>
        </div>
      </div>

      <!-- Seitenleiste -->
      <aside class="w-80 shrink-0 overflow-y-auto border-l border-border bg-card/60 p-3">
        <div class="flex flex-col gap-3">
          <label><span class="label">Anschrift</span><textarea v-model="v.beleg.adresse" class="input h-28 py-1.5 text-[13px]" :disabled="gesperrt" /></label>
          <SummenPanel :beleg="v.beleg" :summen="summen" />
          <ZahlungenPanel v-if="rechnung && v.beleg.status !== 'entwurf'" :v="v" @geaendert="laden" @drucken="(id: number) => (vorschau = { art: 'mahnung', id, titel: 'Mahnung' })" />
          <KonditionenPanel v-model="v" :gesperrt="gesperrt" :rechnung="rechnung" />
          <div v-if="v.beleg.vorgaenger_id || v.nachfolger.length" class="rounded-lg border border-border bg-card p-3 text-xs">
            <div class="label">Verlauf</div>
            <button v-if="v.beleg.vorgaenger_id" class="block text-left text-accent-text hover:underline" @click="oeffneBeleg(v.beleg.vorgaenger_id!)">← Vorgänger öffnen</button>
            <button v-for="n in v.nachfolger" :key="n.id" class="block text-left text-accent-text hover:underline" @click="oeffneBeleg(n.id)">→ {{ n.titel }} {{ n.nummer }} ({{ datum(n.datum) }})</button>
          </div>
        </div>
      </aside>
    </div>

    <PdfVorschau v-if="vorschau" :art="vorschau.art" :id="vorschau.id" :titel="vorschau.titel" @close="vorschau = null" />
    <AufmassDialog v-if="aufmass" :beleg="v" @close="aufmass = false" @gespeichert="nachAufmass" />
    <KalkulationDialog v-if="kalkIndex !== null" :position="v.positionen[kalkIndex]" @close="kalkIndex = null" @gespeichert="nachKalkulation" />
    <ArtikelWahl v-if="artikelIndex !== null" @close="artikelIndex = null" @waehlen="artikelUebernehmen" />
  </div>
</template>
