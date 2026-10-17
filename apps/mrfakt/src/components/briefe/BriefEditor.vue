<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { ArrowLeft, Eye, Printer, Save, Trash2 } from "@lucide/vue";
import PdfVorschau from "../PdfVorschau.vue";
import RichText from "../RichText.vue";
import TextbausteinMenu from "../TextbausteinMenu.vue";
import { api } from "@/lib/api";
import { frage, nav, neuLaden, verlassenOk } from "@/lib/nav";
import type { Brief } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ id: number }>();

const b = ref<Brief>();
const original = ref("");
const vorschau = ref(false);
const editor = ref<InstanceType<typeof RichText>>();
const geaendert = computed(() => !!b.value && JSON.stringify(b.value) !== original.value);
watch(geaendert, (g) => (nav.geaendert = g));

onMounted(async () => {
  try {
    b.value = await api.brief(props.id);
    original.value = JSON.stringify(b.value);
  } catch (e) {
    toastError("Brief konnte nicht geladen werden", e);
  }
});

async function speichern(stumm = false) {
  if (!b.value) return false;
  try {
    await api.briefSpeichern(b.value);
    original.value = JSON.stringify(b.value);
    nav.geaendert = false;
    neuLaden();
    if (!stumm) toast("success", "Brief gespeichert");
    return true;
  } catch (e) {
    toastError("Speichern fehlgeschlagen", e);
    return false;
  }
}

async function drucken() {
  if (!(await speichern(true)) || !b.value) return;
  try {
    await api.pdfSpeichern("brief", b.value.id, { oeffnen: true });
  } catch (e) {
    toastError("PDF konnte nicht erstellt werden", e);
  }
}

async function zeigen() {
  if (await speichern(true)) vorschau.value = true;
}

async function loeschen() {
  if (!b.value || !(await frage("Brief löschen", "Diesen Brief löschen?", "Löschen"))) return;
  await api.briefLoeschen(b.value.id);
  nav.geaendert = false;
  nav.briefId = 0;
  neuLaden();
}

async function schliessen() {
  if (await verlassenOk()) nav.briefId = 0;
}

function tasten(e: KeyboardEvent) {
  if (e.ctrlKey && e.key.toLowerCase() === "s") {
    e.preventDefault();
    speichern();
  }
}
onMounted(() => window.addEventListener("keydown", tasten));
onUnmounted(() => {
  window.removeEventListener("keydown", tasten);
  nav.geaendert = false;
});
</script>

<template>
  <div v-if="b" class="flex min-h-0 flex-1 flex-col">
    <div class="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-card px-3">
      <button class="icon-btn" aria-label="Zurück" @click="schliessen"><ArrowLeft /></button>
      <span class="text-[14px] font-semibold">Brief</span>
      <span v-if="geaendert" class="text-[11px] text-amber-600 dark:text-amber-400">● geändert</span>
      <div class="flex-1" />
      <button class="btn btn-outline h-8" :disabled="!geaendert" @click="speichern()"><Save />Speichern</button>
      <button class="btn btn-outline h-8" @click="zeigen"><Eye />Vorschau</button>
      <button class="btn btn-primary h-8" @click="drucken"><Printer />Drucken</button>
      <button class="icon-btn size-8" title="Löschen" @click="loeschen"><Trash2 /></button>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto p-4">
      <div class="mx-auto flex max-w-4xl flex-col gap-3">
        <div class="grid grid-cols-[1fr_12rem] gap-3">
          <label><span class="label">Anschrift</span><textarea v-model="b.adresse" class="input h-28 py-1.5" /></label>
          <label><span class="label">Datum</span><input v-model="b.datum" type="date" class="input" /></label>
        </div>
        <label><span class="label">Betreff</span><input v-model="b.betreff" class="input" /></label>
        <div>
          <div class="mb-1 flex items-center">
            <span class="label mb-0 flex-1">Text</span>
            <TextbausteinMenu kategorie="brief" @waehlen="(t: string) => editor?.einfuegen(t)" />
          </div>
          <RichText ref="editor" v-model="b.text" hoch />
        </div>
      </div>
    </div>
    <PdfVorschau v-if="vorschau" art="brief" :id="b.id" :titel="b.betreff || 'Brief'" @close="vorschau = false" />
  </div>
</template>
