<script setup lang="ts">
import { computed, ref } from "vue";
import { BellRing, Plus, X } from "@lucide/vue";
import ZahlFeld from "../ZahlFeld.vue";
import { api } from "@/lib/api";
import { betrag, datum, heute } from "@/lib/format";
import { frage, neuLaden } from "@/lib/nav";
import type { BelegVoll } from "@/lib/types";
import { toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ v: BelegVoll }>();
const emit = defineEmits<{ geaendert: []; drucken: [mahnungId: number] }>();

const offen = computed(() => Math.round((props.v.beleg.zahlbetrag - props.v.beleg.bezahlt) * 100) / 100);
const erfassen = ref(false);
const neu = ref({ datum: heute(), betrag: 0, abzug: 0, notiz: "" });

function start() {
  neu.value = { datum: heute(), betrag: offen.value, abzug: 0, notiz: "" };
  erfassen.value = true;
}

async function buchen() {
  try {
    await api.zahlungBuchen({ id: 0, beleg_id: props.v.beleg.id, ...neu.value });
    erfassen.value = false;
    neuLaden();
    emit("geaendert");
  } catch (e) {
    toastError("Zahlung konnte nicht gebucht werden", e);
  }
}

async function loeschen(id: number) {
  if (!(await frage("Zahlung löschen", "Diese Zahlung entfernen?", "Löschen"))) return;
  try {
    await api.zahlungLoeschen(id);
    neuLaden();
    emit("geaendert");
  } catch (e) {
    toastError("Löschen fehlgeschlagen", e);
  }
}

async function mahnen() {
  try {
    const m = await api.mahnen(props.v.beleg.id);
    neuLaden();
    emit("geaendert");
    emit("drucken", m.id);
  } catch (e) {
    toastError("Mahnung nicht möglich", e);
  }
}

async function mahnungLoeschen(id: number) {
  if (!(await frage("Mahnung löschen", "Diese Mahnung entfernen?", "Löschen"))) return;
  await api.mahnungLoeschen(id);
  emit("geaendert");
}

const stufen = ["Zahlungserinnerung", "1. Mahnung", "2. Mahnung", "3. Mahnung"];
</script>

<template>
  <div class="rounded-lg border border-border bg-card p-3 text-[13px]">
    <div class="mb-2 flex items-center gap-2">
      <span class="label mb-0 flex-1">Zahlungen</span>
      <span v-if="Math.abs(offen) < 0.005" class="rounded bg-emerald-500/12 px-1.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-300">bezahlt</span>
      <span v-else class="rounded bg-amber-500/12 px-1.5 text-[11px] font-medium text-amber-700 dark:text-amber-300">{{ betrag(offen) }} offen</span>
    </div>
    <div v-for="z in v.zahlungen" :key="z.id" class="group flex items-center gap-2 py-0.5 text-xs">
      <span class="w-20 text-muted-foreground tabular-nums">{{ datum(z.datum) }}</span>
      <span class="flex-1 tabular-nums">{{ betrag(z.betrag) }}<template v-if="z.abzug"> + {{ betrag(z.abzug) }} Abzug</template></span>
      <button class="icon-btn size-5 opacity-0 group-hover:opacity-100" @click="loeschen(z.id)"><X /></button>
    </div>
    <div v-if="erfassen" class="mt-2 grid grid-cols-2 gap-1.5">
      <label><span class="label">Datum</span><input v-model="neu.datum" type="date" class="input h-7 text-xs" /></label>
      <label><span class="label">Betrag</span><ZahlFeld v-model="neu.betrag" class="h-7 text-xs" /></label>
      <label><span class="label">Skonto/Abzug</span><ZahlFeld v-model="neu.abzug" class="h-7 text-xs" /></label>
      <label><span class="label">Notiz</span><input v-model="neu.notiz" class="input h-7 text-xs" /></label>
      <div class="col-span-2 flex justify-end gap-1">
        <button class="btn btn-ghost h-7 text-xs" @click="erfassen = false">Abbrechen</button>
        <button class="btn btn-primary h-7 text-xs" @click="buchen">Buchen</button>
      </div>
    </div>
    <div v-else-if="v.beleg.status !== 'storniert'" class="mt-1 flex gap-1">
      <button class="btn btn-ghost h-7 px-2 text-xs" @click="start"><Plus />Zahlung</button>
      <button v-if="offen > 0.004" class="btn btn-ghost h-7 px-2 text-xs" @click="mahnen"><BellRing />{{ v.mahnungen.length ? "Nächste Mahnstufe" : "Zahlungserinnerung" }}</button>
    </div>
    <div v-if="v.mahnungen.length" class="mt-2 border-t border-border pt-2">
      <div v-for="m in v.mahnungen" :key="m.id" class="group flex items-center gap-2 text-xs">
        <button class="flex-1 text-left text-accent-text hover:underline" @click="emit('drucken', m.id)">{{ stufen[m.stufe] ?? m.stufe + ". Mahnung" }}</button>
        <span class="text-muted-foreground tabular-nums">{{ datum(m.datum) }}</span>
        <button class="icon-btn size-5 opacity-0 group-hover:opacity-100" @click="mahnungLoeschen(m.id)"><X /></button>
      </div>
    </div>
    <p v-else-if="v.beleg.mahnstufe" class="mt-2 text-[11px] text-muted-foreground">Mahnstufe {{ v.beleg.mahnstufe }} (aus GHFakt)</p>
  </div>
</template>
