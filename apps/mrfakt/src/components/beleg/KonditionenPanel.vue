<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Plus, RefreshCw, X } from "@lucide/vue";
import ZahlFeld from "../ZahlFeld.vue";
import { api } from "@/lib/api";
import type { BelegVoll, Zahlungsbedingung, ZuschlagArt } from "@/lib/types";
import { toastError } from "@mrtools/ui/lib/toast";

const v = defineModel<BelegVoll>({ required: true });
defineProps<{ gesperrt: boolean; rechnung: boolean }>();

const bedingungen = ref<Zahlungsbedingung[]>([]);
onMounted(async () => {
  try {
    bedingungen.value = (await api.einstellungen()).allgemein.zahlungsbedingungen;
  } catch {
    bedingungen.value = [];
  }
});

function bedingungWaehlen(e: Event) {
  const b = bedingungen.value[Number((e.target as HTMLSelectElement).value)];
  if (!b) return;
  v.value.beleg.zahlungsbedingung = b.text;
  v.value.beleg.skonto_prozent = b.skonto;
  v.value.beleg.skonto_tage = b.skonto_tage;
  v.value.beleg.zahlungsziel_tage = b.ziel_tage;
}

const zuschlagArten: { value: ZuschlagArt; label: string }[] = [
  { value: "prozent_netto", label: "% vom Netto" },
  { value: "betrag_netto", label: "Betrag netto" },
  { value: "prozent_brutto", label: "% vom Brutto" },
  { value: "betrag_brutto", label: "Betrag brutto" },
];

function zuschlagNeu() {
  v.value.zuschlaege.push({ id: 0, beleg_id: v.value.beleg.id, pos: v.value.zuschlaege.length, text: "Zuschlag", wert: 0, art: "prozent_netto" });
}

async function abzuegeNeu() {
  try {
    v.value.abzuege = await api.abzuegeVorschlagen(v.value.beleg.projekt_id, v.value.beleg.id);
  } catch (e) {
    toastError("Abschläge konnten nicht ermittelt werden", e);
  }
}

function abzugNeu() {
  v.value.abzuege.push({ id: 0, beleg_id: v.value.beleg.id, pos: v.value.abzuege.length, text: "Abschlagszahlung", nummer: "", datum: "", netto: 0, mwst: 0, brutto: 0, bezahlt: 0, quelle_id: null });
}
</script>

<template>
  <div class="flex flex-col gap-3 rounded-lg border border-border bg-card p-3">
    <div class="grid grid-cols-2 gap-2">
      <label><span class="label">MwSt. %</span><ZahlFeld v-model="v.beleg.mwst_satz" :stellen="0" :disabled="gesperrt" /></label>
      <label><span class="label">Rabatt %</span><ZahlFeld v-model="v.beleg.rabatt_prozent" :stellen="0" :disabled="gesperrt" /></label>
      <label v-if="v.beleg.rabatt_prozent" class="col-span-2"><span class="label">Rabatt-Bezeichnung</span><input v-model="v.beleg.rabatt_text" class="input" :disabled="gesperrt" /></label>
    </div>
    <label class="flex items-start gap-2 text-[13px]">
      <input v-model="v.beleg.netto_rechnung" type="checkbox" class="mt-0.5" :disabled="gesperrt" />
      <span>§ 13b UStG – Leistungsempfänger schuldet die Umsatzsteuer</span>
    </label>
    <label class="flex items-start gap-2 text-[13px]">
      <input v-model="v.beleg.lohn_ausweisen" type="checkbox" class="mt-0.5" :disabled="gesperrt" />
      <span>Lohnanteil nach § 35a EStG ausweisen</span>
    </label>

    <template v-if="rechnung">
      <div>
        <div class="mb-1 flex items-center">
          <span class="label mb-0 flex-1">Zahlungsbedingung</span>
          <select v-if="!gesperrt && bedingungen.length" class="input h-6 w-auto px-1 text-[11px]" @change="bedingungWaehlen">
            <option value="">Vorlage …</option>
            <option v-for="(b, i) in bedingungen" :key="i" :value="i">{{ b.name }}</option>
          </select>
        </div>
        <textarea v-model="v.beleg.zahlungsbedingung" class="input h-20 py-1.5 text-xs" :disabled="gesperrt" />
        <p class="mt-1 text-[10px] text-muted-foreground">Platzhalter: #ZN# Zahlungsziel, #ZS# Skontofrist, #SK# Skonto %, #BN# Betrag, #SB# Skontobetrag, #BS# Betrag nach Skonto, #WE# Währung</p>
      </div>
      <div class="grid grid-cols-3 gap-2">
        <label><span class="label">Ziel (Tage)</span><input v-model.number="v.beleg.zahlungsziel_tage" type="number" class="input" :disabled="gesperrt" /></label>
        <label><span class="label">Skonto %</span><ZahlFeld v-model="v.beleg.skonto_prozent" :stellen="0" :disabled="gesperrt" /></label>
        <label><span class="label">Skonto Tage</span><input v-model.number="v.beleg.skonto_tage" type="number" class="input" :disabled="gesperrt" /></label>
      </div>
    </template>

    <div>
      <div class="mb-1 flex items-center">
        <span class="label mb-0 flex-1">Zu- und Abschläge</span>
        <button v-if="!gesperrt" class="icon-btn size-6" title="Zuschlag hinzufügen" @click="zuschlagNeu"><Plus /></button>
      </div>
      <div v-for="(z, i) in v.zuschlaege" :key="i" class="mb-1.5 grid grid-cols-[1fr_5rem_1.5rem] gap-1">
        <input v-model="z.text" class="input h-7 text-xs" :disabled="gesperrt" />
        <ZahlFeld v-model="z.wert" class="h-7 text-xs" :disabled="gesperrt" />
        <button v-if="!gesperrt" class="icon-btn size-6" @click="v.zuschlaege.splice(i, 1)"><X /></button>
        <select v-model="z.art" class="input col-span-2 h-7 text-xs" :disabled="gesperrt">
          <option v-for="a in zuschlagArten" :key="a.value" :value="a.value">{{ a.label }}</option>
        </select>
      </div>
      <p v-if="!v.zuschlaege.length" class="text-[11px] text-muted-foreground">Keine. Negative Werte sind Abschläge (z. B. −3 % Nachlass).</p>
    </div>

    <div v-if="v.beleg.art === 'schluss' || v.beleg.art === 'abschlag' || v.abzuege.length">
      <div class="mb-1 flex items-center">
        <span class="label mb-0 flex-1">Abzüge (Abschlagsrechnungen)</span>
        <button v-if="!gesperrt" class="icon-btn size-6" title="Abschläge des Projekts übernehmen" @click="abzuegeNeu"><RefreshCw /></button>
        <button v-if="!gesperrt" class="icon-btn size-6" title="Abzug hinzufügen" @click="abzugNeu"><Plus /></button>
      </div>
      <div v-for="(a, i) in v.abzuege" :key="i" class="mb-1.5 grid grid-cols-[1fr_5.5rem_1.5rem] gap-1">
        <input v-model="a.text" class="input h-7 text-xs" :disabled="gesperrt" />
        <ZahlFeld v-model="a.brutto" class="h-7 text-xs" :disabled="gesperrt" />
        <button v-if="!gesperrt" class="icon-btn size-6" @click="v.abzuege.splice(i, 1)"><X /></button>
        <input v-model="a.nummer" class="input h-7 text-xs" placeholder="Rechnungs-Nr." :disabled="gesperrt" />
        <input v-model="a.datum" type="date" class="input h-7 text-xs" :disabled="gesperrt" />
      </div>
    </div>
  </div>
</template>
