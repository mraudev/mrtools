<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ImageMinus, ImagePlus, Plus, Save, Trash2 } from "@lucide/vue";
import { open } from "@tauri-apps/plugin-dialog";
import ZahlFeld from "../ZahlFeld.vue";
import RichText from "../RichText.vue";
import DatenPanel from "./DatenPanel.vue";
import TextbausteinePanel from "./TextbausteinePanel.vue";
import FormelnPanel from "./FormelnPanel.vue";
import MitarbeiterPanel from "./MitarbeiterPanel.vue";
import { api } from "@/lib/api";
import { artTitel } from "@/lib/format";
import { nav } from "@/lib/nav";
import type { Einstellungen } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

type Bereich = "firma" | "papier" | "belege" | "zahlung" | "mahnung" | "kalkulation" | "formeln" | "texte" | "daten";
const bereiche: { id: Bereich; text: string }[] = [
  { id: "firma", text: "Firma" },
  { id: "papier", text: "Briefpapier" },
  { id: "belege", text: "Belege & Nummern" },
  { id: "zahlung", text: "Zahlungsbedingungen" },
  { id: "mahnung", text: "Mahnwesen" },
  { id: "kalkulation", text: "Kalkulation & Mitarbeiter" },
  { id: "formeln", text: "Aufmaß-Formeln" },
  { id: "texte", text: "Textbausteine" },
  { id: "daten", text: "Daten & GHFakt-Import" },
];
const bereich = ref<Bereich>("firma");
const e = ref<Einstellungen>();
const original = ref("");
const bilder = ref<{ logo: string | null; fusszeile: string | null }>({ logo: null, fusszeile: null });
const vorgabeArt = ref("rechnung");
const geaendert = computed(() => !!e.value && JSON.stringify(e.value) !== original.value);
const speicherbar = computed(() => !["formeln", "texte", "daten"].includes(bereich.value) || geaendert.value);

const kreisTitel: Record<string, string> = {
  kunde: "Kunden", projekt: "Projekte/Aufträge", angebot: "Angebote", auftrag: "Auftragsbestätigungen", lieferschein: "Lieferscheine", rechnung: "Rechnungen (auch Abschlag/Schluss)", gutschrift: "Gutschriften",
};

async function laden() {
  try {
    e.value = await api.einstellungen();
    original.value = JSON.stringify(e.value);
    bilder.value = { logo: await api.bildLaden("logo"), fusszeile: await api.bildLaden("fusszeile") };
  } catch (err) {
    toastError("Einstellungen konnten nicht geladen werden", err);
  }
}
onMounted(laden);

async function speichern() {
  if (!e.value) return;
  try {
    await api.einstellungenSpeichern(e.value.firma, e.value.allgemein, e.value.nummernkreise);
    original.value = JSON.stringify(e.value);
    toast("success", "Einstellungen gespeichert");
  } catch (err) {
    toastError("Speichern fehlgeschlagen", err);
  }
}

async function bild(name: "logo" | "fusszeile", entfernen = false) {
  let pfad = "";
  if (!entfernen) {
    const p = await open({ filters: [{ name: "Bild", extensions: ["png", "jpg", "jpeg"] }] });
    if (typeof p !== "string") return;
    pfad = p;
  }
  try {
    await api.bildSetzen(name, pfad);
    bilder.value[name] = await api.bildLaden(name);
  } catch (err) {
    toastError("Bild konnte nicht übernommen werden", err);
  }
}

function vorgabe(art: string) {
  const v = e.value!.allgemein.belegvorgaben;
  if (!v[art]) v[art] = { vortext: "", nachtext: "", zahlungsbedingung: "" };
  return v[art];
}

async function pdfOrdner() {
  const p = await open({ directory: true });
  if (typeof p === "string" && e.value) e.value.allgemein.pdf_ordner = p;
}

function nachImport() {
  laden();
  nav.version++;
}
</script>

<template>
  <div class="flex min-h-0 flex-1">
    <nav class="w-56 shrink-0 border-r border-border bg-card p-2">
      <button
        v-for="b in bereiche"
        :key="b.id"
        class="flex h-8 w-full items-center rounded-md px-2.5 text-left text-[13px]"
        :class="bereich === b.id ? 'bg-accent/15 font-medium text-accent-text' : 'hover:bg-foreground/5'"
        @click="bereich = b.id"
      >
        {{ b.text }}
      </button>
    </nav>
    <div v-if="e" class="min-w-0 flex-1 overflow-y-auto">
      <div class="mx-auto flex max-w-4xl flex-col gap-4 p-5">
        <div class="flex items-center gap-2">
          <h1 class="flex-1 text-[15px] font-semibold">{{ bereiche.find((b) => b.id === bereich)?.text }}</h1>
          <button v-if="speicherbar" class="btn btn-primary h-8" :disabled="!geaendert" @click="speichern"><Save />Speichern</button>
        </div>

        <!-- Firma -->
        <div v-if="bereich === 'firma'" class="grid grid-cols-6 gap-3 rounded-xl border border-border bg-card p-4">
          <label class="col-span-3"><span class="label">Firmenname</span><input v-model="e.firma.name" class="input" /></label>
          <label class="col-span-3"><span class="label">Inhaber</span><input v-model="e.firma.inhaber" class="input" /></label>
          <label class="col-span-6"><span class="label">Zusatz (z. B. Maler- und Lackierermeister)</span><input v-model="e.firma.zusatz" class="input" /></label>
          <label class="col-span-3"><span class="label">Straße</span><input v-model="e.firma.strasse" class="input" /></label>
          <label class="col-span-1"><span class="label">PLZ</span><input v-model="e.firma.plz" class="input" /></label>
          <label class="col-span-2"><span class="label">Ort</span><input v-model="e.firma.ort" class="input" /></label>
          <label class="col-span-2"><span class="label">Telefon</span><input v-model="e.firma.telefon" class="input" /></label>
          <label class="col-span-2"><span class="label">Telefax</span><input v-model="e.firma.telefax" class="input" /></label>
          <label class="col-span-2"><span class="label">Mobil</span><input v-model="e.firma.mobil" class="input" /></label>
          <label class="col-span-3"><span class="label">E-Mail</span><input v-model="e.firma.email" class="input" /></label>
          <label class="col-span-3"><span class="label">Internet</span><input v-model="e.firma.internet" class="input" /></label>
          <label class="col-span-2"><span class="label">Steuernummer</span><input v-model="e.firma.steuernummer" class="input" /></label>
          <label class="col-span-2"><span class="label">USt-IdNr.</span><input v-model="e.firma.ustid" class="input" /></label>
          <label class="col-span-2"><span class="label">Handwerkskammer / Register</span><input v-model="e.firma.registergericht" class="input" /></label>
          <div class="col-span-6">
            <div class="mb-1 flex items-center">
              <span class="label mb-0 flex-1">Bankverbindungen (für Rechnungen und E-Rechnung)</span>
              <button class="icon-btn size-6" @click="e.firma.banken.push({ name: '', iban: '', bic: '' })"><Plus /></button>
            </div>
            <div v-for="(b, i) in e.firma.banken" :key="i" class="mb-1.5 grid grid-cols-[1fr_1.4fr_9rem_2rem] gap-2">
              <input v-model="b.name" class="input" placeholder="Bank" />
              <input v-model="b.iban" class="input" placeholder="IBAN" />
              <input v-model="b.bic" class="input" placeholder="BIC" />
              <button class="icon-btn" @click="e.firma.banken.splice(i, 1)"><Trash2 /></button>
            </div>
            <p v-if="!e.firma.banken.length" class="text-xs text-amber-600 dark:text-amber-400">Noch keine Bankverbindung – für E-Rechnungen (ZUGFeRD) bitte eintragen.</p>
          </div>
          <label class="col-span-2"><span class="label">MwSt. Regelsatz %</span><ZahlFeld v-model="e.firma.mwst_satz" :stellen="0" /></label>
          <label class="col-span-2"><span class="label">MwSt. ermäßigt %</span><ZahlFeld v-model="e.firma.mwst_ermaessigt" :stellen="0" /></label>
          <label class="col-span-2"><span class="label">Währung</span><input v-model="e.firma.waehrung" class="input" /></label>
          <label class="col-span-3"><span class="label">Grußformel</span><input v-model="e.firma.gruss" class="input" /></label>
          <label class="col-span-3"><span class="label">Unterschrift</span><input v-model="e.firma.unterschrift" class="input" /></label>
        </div>

        <!-- Briefpapier -->
        <template v-if="bereich === 'papier'">
          <div class="grid grid-cols-2 gap-3">
            <div class="rounded-xl border border-border bg-card p-4">
              <div class="label">Logo / Briefkopf (rechts oben)</div>
              <div class="grid h-28 place-items-center rounded-md border border-dashed border-border bg-white p-2">
                <img v-if="bilder.logo" :src="bilder.logo" alt="Logo" class="max-h-24 object-contain" />
                <span v-else class="text-xs text-muted-foreground">ohne Logo: Firmenname als Text</span>
              </div>
              <div class="mt-2 flex gap-1">
                <button class="btn btn-outline h-7 text-xs" @click="bild('logo')"><ImagePlus />Bild wählen</button>
                <button v-if="bilder.logo" class="btn btn-ghost h-7 text-xs" @click="bild('logo', true)"><ImageMinus />Entfernen</button>
              </div>
            </div>
            <div class="rounded-xl border border-border bg-card p-4">
              <div class="label">Fußzeile als Bild (statt Text)</div>
              <div class="grid h-28 place-items-center rounded-md border border-dashed border-border bg-white p-2">
                <img v-if="bilder.fusszeile" :src="bilder.fusszeile" alt="Fußzeile" class="max-h-24 object-contain" />
                <span v-else class="text-xs text-muted-foreground">Fußzeile aus Firmendaten/Text</span>
              </div>
              <div class="mt-2 flex gap-1">
                <button class="btn btn-outline h-7 text-xs" @click="bild('fusszeile')"><ImagePlus />Bild wählen</button>
                <button v-if="bilder.fusszeile" class="btn btn-ghost h-7 text-xs" @click="bild('fusszeile', true)"><ImageMinus />Entfernen</button>
              </div>
            </div>
          </div>
          <div class="grid grid-cols-4 gap-3 rounded-xl border border-border bg-card p-4">
            <label class="col-span-4"><span class="label">Absenderzeile über der Anschrift</span><input v-model="e.firma.absenderzeile" class="input" /></label>
            <label class="col-span-2"><span class="label">Fußzeile (Text, mehrzeilig)</span><textarea v-model="e.firma.fusszeile" class="input h-24 py-1.5 text-xs" /></label>
            <label class="col-span-2"><span class="label">Leistungen am rechten Rand (eine je Zeile)</span><textarea v-model="e.firma.seitenleiste" class="input h-24 py-1.5 text-xs" /></label>
            <label><span class="label">Rand links (mm)</span><ZahlFeld v-model="e.allgemein.rand_links" :stellen="0" /></label>
            <label><span class="label">Rand rechts (mm)</span><ZahlFeld v-model="e.allgemein.rand_rechts" :stellen="0" /></label>
            <label><span class="label">Rand oben (mm)</span><ZahlFeld v-model="e.allgemein.rand_oben" :stellen="0" /></label>
            <label><span class="label">Rand unten (mm)</span><ZahlFeld v-model="e.allgemein.rand_unten" :stellen="0" /></label>
            <label class="col-span-3"><span class="label">Ordner für PDFs (leer = Dokumente\mrfakt)</span><input v-model="e.allgemein.pdf_ordner" class="input" /></label>
            <div class="flex items-end"><button class="btn btn-outline h-8 w-full" @click="pdfOrdner">Ordner wählen …</button></div>
            <label class="col-span-4 flex items-center gap-2 text-[13px]">
              <input v-model="e.allgemein.zugferd" type="checkbox" />E-Rechnung (ZUGFeRD 2 / Factur-X, EN 16931) in Rechnungen einbetten
            </label>
          </div>
        </template>

        <!-- Belege -->
        <template v-if="bereich === 'belege'">
          <div class="rounded-xl border border-border bg-card p-4">
            <div class="mb-3 flex items-center gap-2">
              <span class="label mb-0">Vorgaben für neue Belege:</span>
              <select v-model="vorgabeArt" class="input h-7 w-56">
                <option v-for="(t, k) in artTitel" :key="k" :value="k">{{ t }}</option>
              </select>
            </div>
            <div class="flex flex-col gap-3">
              <div><span class="label">Vortext</span><RichText :key="vorgabeArt + 'v'" v-model="vorgabe(vorgabeArt).vortext" /></div>
              <div><span class="label">Nachtext</span><RichText :key="vorgabeArt + 'n'" v-model="vorgabe(vorgabeArt).nachtext" /></div>
              <label><span class="label">Zahlungsbedingung (leer = Standard aus „Zahlungsbedingungen“)</span><textarea v-model="vorgabe(vorgabeArt).zahlungsbedingung" class="input h-16 py-1.5 text-xs" /></label>
            </div>
          </div>
          <div class="rounded-xl border border-border bg-card p-4">
            <div class="label">Nummernkreise (nächste Nummer, Format mit {n} und {jahr})</div>
            <div v-for="k in e.nummernkreise" :key="k.name" class="mb-1.5 grid grid-cols-[1fr_8rem_10rem] items-center gap-2">
              <span class="text-[13px]">{{ kreisTitel[k.name] ?? k.name }}</span>
              <input v-model.number="k.naechste" type="number" class="input h-7 text-right" />
              <input v-model="k.format" class="input h-7" />
            </div>
          </div>
          <div class="rounded-xl border border-border bg-card p-4">
            <label><span class="label">Einheiten (eine je Zeile)</span>
              <textarea :value="e.allgemein.einheiten.join('\n')" class="input h-32 py-1.5 text-xs" @input="e.allgemein.einheiten = ($event.target as HTMLTextAreaElement).value.split('\n').map((s) => s.trim()).filter(Boolean)" />
            </label>
          </div>
        </template>

        <!-- Zahlungsbedingungen -->
        <div v-if="bereich === 'zahlung'" class="flex flex-col gap-3">
          <p class="text-xs text-muted-foreground">Platzhalter: #ZN# Zahlungsziel (Datum), #ZS# Skontofrist, #SK# Skontosatz, #BN# Betrag, #SB# Skontobetrag, #BS# Betrag nach Skonto, #WE# Währung.</p>
          <div v-for="(z, i) in e.allgemein.zahlungsbedingungen" :key="i" class="grid grid-cols-6 gap-3 rounded-xl border border-border bg-card p-4">
            <label class="col-span-3"><span class="label">Name</span><input v-model="z.name" class="input" /></label>
            <label><span class="label">Ziel (Tage)</span><input v-model.number="z.ziel_tage" type="number" class="input" /></label>
            <label><span class="label">Skonto %</span><ZahlFeld v-model="z.skonto" :stellen="0" /></label>
            <label><span class="label">Skonto Tage</span><input v-model.number="z.skonto_tage" type="number" class="input" /></label>
            <label class="col-span-6"><span class="label">Text</span><textarea v-model="z.text" class="input h-20 py-1.5 text-xs" /></label>
            <label class="col-span-5 flex items-center gap-2 text-[13px]">
              <input type="radio" :checked="e.allgemein.zahlungsbedingung_standard === i" @change="e.allgemein.zahlungsbedingung_standard = i" />Standard für neue Rechnungen
            </label>
            <button class="btn btn-ghost h-7 text-xs text-red-500" @click="e.allgemein.zahlungsbedingungen.splice(i, 1)"><Trash2 />Entfernen</button>
          </div>
          <button class="btn btn-outline h-8 self-start" @click="e.allgemein.zahlungsbedingungen.push({ name: 'Neu', text: 'Zahlbar bis zum #ZN# ohne Abzug.', skonto: 0, skonto_tage: 0, ziel_tage: 14 })"><Plus />Zahlungsbedingung</button>
        </div>

        <!-- Mahnwesen -->
        <div v-if="bereich === 'mahnung'" class="flex flex-col gap-3">
          <p class="text-xs text-muted-foreground">#ZN# im Text wird durch die neue Zahlungsfrist ersetzt.</p>
          <div v-for="(m, i) in e.allgemein.mahnstufen" :key="i" class="grid grid-cols-4 gap-3 rounded-xl border border-border bg-card p-4">
            <label class="col-span-2"><span class="label">Stufe {{ i }} – Überschrift</span><input v-model="m.titel" class="input" /></label>
            <label><span class="label">Gebühr</span><ZahlFeld v-model="m.gebuehr" /></label>
            <label><span class="label">Frist (Tage)</span><input v-model.number="m.frist_tage" type="number" class="input" /></label>
            <label class="col-span-4"><span class="label">Einleitung</span><textarea v-model="m.einleitung" class="input h-20 py-1.5 text-xs" /></label>
            <label class="col-span-4"><span class="label">Schluss</span><textarea v-model="m.schluss" class="input h-20 py-1.5 text-xs" /></label>
          </div>
        </div>

        <!-- Kalkulation -->
        <template v-if="bereich === 'kalkulation'">
          <div class="grid grid-cols-3 gap-3 rounded-xl border border-border bg-card p-4">
            <label><span class="label">Verrechnungslohn je Stunde</span><ZahlFeld v-model="e.allgemein.kalkulation.stundensatz" /></label>
            <label><span class="label">Zuschlag Lohn %</span><ZahlFeld v-model="e.allgemein.kalkulation.zuschlag_lohn" :stellen="0" /></label>
            <label><span class="label">Zuschlag Material %</span><ZahlFeld v-model="e.allgemein.kalkulation.zuschlag_material" :stellen="0" /></label>
            <label><span class="label">Zuschlag Gerät %</span><ZahlFeld v-model="e.allgemein.kalkulation.zuschlag_geraet" :stellen="0" /></label>
            <label><span class="label">Zuschlag Fremdleistung %</span><ZahlFeld v-model="e.allgemein.kalkulation.zuschlag_fremd" :stellen="0" /></label>
            <label><span class="label">Zuschlag Sonstiges %</span><ZahlFeld v-model="e.allgemein.kalkulation.zuschlag_sonstig" :stellen="0" /></label>
          </div>
          <MitarbeiterPanel />
        </template>

        <FormelnPanel v-if="bereich === 'formeln'" />
        <TextbausteinePanel v-if="bereich === 'texte'" />
        <DatenPanel v-if="bereich === 'daten'" v-model:sicherungen="e.allgemein.sicherungen" @importiert="nachImport" />
      </div>
    </div>
  </div>
</template>
