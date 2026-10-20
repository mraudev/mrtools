<script setup lang="ts">
import { ref } from "vue";
import Dialog from "@mrtools/ui/components/Dialog";
import Segmented from "@mrtools/ui/components/Segmented";
import { api } from "@/lib/api";
import type { Kunde, Plz } from "@/lib/types";
import { toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ kunde: Kunde | null }>();
const emit = defineEmits<{ close: []; saved: [id: number] }>();

const leer: Kunde = {
  id: 0, nummer: "", typ: "Kunde", suchname: "", anrede: "", name1: "", name2: "", name3: "", strasse: "", ortsteil: "", plz: "", ort: "", land: "",
  telefon: "", telefon_privat: "", telefax: "", mobil: "", mobil_privat: "", email: "", email_privat: "", internet: "", ansprechpartner: "", position: "",
  geschlecht: "", geburtstag: "", bank: "", blz: "", konto: "", iban: "", bic: "", ustid: "", leitweg_id: "", unsere_kdnr: "", ihre_kdnr: "", gruppe: "",
  skonto: "", rabatt: "", zahlungsbedingung: "", rechnungsart: "", ust_art: "", sonstiges1: "", sonstiges2: "", sonstiges3: "", sonstiges4: "", sonstiges5: "",
  bemerkung1: "", bemerkung2: "", notiz: "", alt_id: "", angelegt: "", geaendert: "", zugriff: "",
};
const k = ref<Kunde>({ ...leer, ...(props.kunde ?? {}) });
const offen = ref(true);
const reiter = ref<"adresse" | "kontakt" | "rechnung" | "sonstiges">("adresse");
const orte = ref<Plz[]>([]);
const anreden = ["Herrn", "Frau", "Frau und Herrn", "Familie", "Firma", "Eheleute"];

async function plzSuchen() {
  if (k.value.plz.trim().length < 4) return;
  try {
    orte.value = await api.plz(k.value.plz, "");
    const eindeutig = [...new Set(orte.value.map((o) => o.ort))];
    if (eindeutig.length === 1 && !k.value.ort) k.value.ort = eindeutig[0];
  } catch {
    orte.value = [];
  }
}

async function speichern() {
  if (!k.value.name1.trim() && !k.value.suchname.trim()) {
    toastError("Bitte einen Namen angeben", "Name oder Suchname fehlt.");
    return;
  }
  try {
    emit("saved", await api.kundeSpeichern(k.value));
  } catch (e) {
    toastError("Kunde konnte nicht gespeichert werden", e);
  }
}
</script>

<template>
  <Dialog v-model:open="offen" :title="k.id ? 'Kunde bearbeiten' : 'Neuer Kunde'" size="lg" @update:open="(v: boolean) => !v && emit('close')">
    <div class="mb-4">
      <Segmented
        v-model="reiter"
        :options="[{ value: 'adresse', label: 'Anschrift' }, { value: 'kontakt', label: 'Kontakt' }, { value: 'rechnung', label: 'Rechnung & Bank' }, { value: 'sonstiges', label: 'Sonstiges' }]"
      />
    </div>
    <form id="kunde-form" class="grid grid-cols-6 gap-3" @submit.prevent="speichern">
      <template v-if="reiter === 'adresse'">
        <label class="col-span-2">
          <span class="label">Art</span>
          <select v-model="k.typ" class="input">
            <option>Kunde</option><option>Lieferant</option><option>Interessent</option><option>Sonstige</option>
          </select>
        </label>
        <label class="col-span-2"><span class="label">Kunden-Nr.</span><input v-model="k.nummer" class="input" placeholder="automatisch" /></label>
        <label class="col-span-2"><span class="label">Suchname</span><input v-model="k.suchname" class="input" placeholder="aus dem Namen" /></label>
        <label class="col-span-2">
          <span class="label">Anrede (1. Adresszeile)</span>
          <input v-model="k.anrede" class="input" list="anreden" />
          <datalist id="anreden"><option v-for="a in anreden" :key="a" :value="a" /></datalist>
        </label>
        <label class="col-span-4"><span class="label">Name</span><input v-model="k.name1" class="input" autofocus /></label>
        <label class="col-span-3"><span class="label">Zusatz</span><input v-model="k.name2" class="input" /></label>
        <label class="col-span-3"><span class="label">Zusatz 2</span><input v-model="k.name3" class="input" /></label>
        <label class="col-span-4"><span class="label">Straße</span><input v-model="k.strasse" class="input" /></label>
        <label class="col-span-2"><span class="label">Ortsteil</span><input v-model="k.ortsteil" class="input" /></label>
        <label class="col-span-1"><span class="label">PLZ</span><input v-model="k.plz" class="input" @blur="plzSuchen" /></label>
        <label class="col-span-3">
          <span class="label">Ort</span>
          <input v-model="k.ort" class="input" list="orte" />
          <datalist id="orte"><option v-for="o in orte" :key="o.id" :value="o.ort">{{ o.zusatz }}</option></datalist>
        </label>
        <label class="col-span-2"><span class="label">Land</span><input v-model="k.land" class="input" placeholder="DE" /></label>
      </template>
      <template v-else-if="reiter === 'kontakt'">
        <label class="col-span-3"><span class="label">Telefon</span><input v-model="k.telefon" class="input" /></label>
        <label class="col-span-3"><span class="label">Telefon privat</span><input v-model="k.telefon_privat" class="input" /></label>
        <label class="col-span-3"><span class="label">Mobil</span><input v-model="k.mobil" class="input" /></label>
        <label class="col-span-3"><span class="label">Telefax</span><input v-model="k.telefax" class="input" /></label>
        <label class="col-span-3"><span class="label">E-Mail</span><input v-model="k.email" class="input" type="email" /></label>
        <label class="col-span-3"><span class="label">E-Mail privat</span><input v-model="k.email_privat" class="input" type="email" /></label>
        <label class="col-span-3"><span class="label">Internet</span><input v-model="k.internet" class="input" /></label>
        <label class="col-span-3"><span class="label">Ansprechpartner</span><input v-model="k.ansprechpartner" class="input" /></label>
        <label class="col-span-2">
          <span class="label">Briefanrede</span>
          <select v-model="k.geschlecht" class="input"><option value="">neutral</option><option>Herr</option><option>Frau</option></select>
        </label>
        <label class="col-span-2"><span class="label">Position</span><input v-model="k.position" class="input" /></label>
        <label class="col-span-2"><span class="label">Geburtstag</span><input v-model="k.geburtstag" type="date" class="input" /></label>
      </template>
      <template v-else-if="reiter === 'rechnung'">
        <label class="col-span-3">
          <span class="label">Umsatzsteuer</span>
          <select v-model="k.ust_art" class="input">
            <option value="">normal (MwSt. ausweisen)</option>
            <option value="13b">§ 13b UStG – Leistungsempfänger schuldet die Steuer</option>
          </select>
        </label>
        <label class="col-span-3"><span class="label">USt-IdNr. des Kunden</span><input v-model="k.ustid" class="input" /></label>
        <label class="col-span-3"><span class="label">Leitweg-ID (XRechnung an Behörden)</span><input v-model="k.leitweg_id" class="input" /></label>
        <label class="col-span-3"><span class="label">Zahlungsbedingung (Name, leer = Standard)</span><input v-model="k.zahlungsbedingung" class="input" /></label>
        <label class="col-span-3"><span class="label">Bank</span><input v-model="k.bank" class="input" /></label>
        <label class="col-span-3"><span class="label">IBAN</span><input v-model="k.iban" class="input" /></label>
        <label class="col-span-2"><span class="label">BIC</span><input v-model="k.bic" class="input" /></label>
        <label class="col-span-2"><span class="label">BLZ (alt)</span><input v-model="k.blz" class="input" /></label>
        <label class="col-span-2"><span class="label">Konto (alt)</span><input v-model="k.konto" class="input" /></label>
        <label class="col-span-3"><span class="label">Unsere Kunden-Nr. beim Lieferanten</span><input v-model="k.unsere_kdnr" class="input" /></label>
        <label class="col-span-3"><span class="label">Debitorenkonto</span><input v-model="k.ihre_kdnr" class="input" /></label>
      </template>
      <template v-else>
        <label class="col-span-2"><span class="label">Gruppe</span><input v-model="k.gruppe" class="input" /></label>
        <label class="col-span-2"><span class="label">Skonto</span><input v-model="k.skonto" class="input" /></label>
        <label class="col-span-2"><span class="label">Rabatt</span><input v-model="k.rabatt" class="input" /></label>
        <label class="col-span-3"><span class="label">Bemerkung 1</span><input v-model="k.bemerkung1" class="input" /></label>
        <label class="col-span-3"><span class="label">Bemerkung 2</span><input v-model="k.bemerkung2" class="input" /></label>
        <label v-for="n in 5" :key="n" class="col-span-2">
          <span class="label">Sonstiges {{ n }}</span>
          <input v-model="(k as unknown as Record<string, string>)['sonstiges' + n]" class="input" />
        </label>
        <label class="col-span-6"><span class="label">Notiz</span><textarea v-model="k.notiz" class="input h-24 py-1.5" /></label>
      </template>
    </form>
    <template #footer>
      <span v-if="k.alt_id" class="text-[11px] text-muted-foreground">GHFakt-ID {{ k.alt_id }}</span>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="emit('close')">Abbrechen</button>
      <button class="btn btn-primary" type="submit" form="kunde-form">Speichern</button>
    </template>
  </Dialog>
</template>
