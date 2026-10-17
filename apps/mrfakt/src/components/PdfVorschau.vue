<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { Download, LoaderCircle } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { api, type Druckart } from "@/lib/api";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ art: Druckart; id: number; titel: string; ids?: number[] }>();
const emit = defineEmits<{ close: [] }>();

const offen = ref(true);
const url = ref("");
const fehler = ref("");

onMounted(async () => {
  try {
    const b64 = await api.pdfVorschau(props.art, props.id, props.ids);
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    url.value = URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
  } catch (e) {
    fehler.value = String(e);
  }
});
onBeforeUnmount(() => url.value && URL.revokeObjectURL(url.value));

async function speichern() {
  try {
    const pfad = await api.pdfSpeichern(props.art, props.id, { ids: props.ids, oeffnen: true });
    toast("success", "PDF gespeichert", pfad);
  } catch (e) {
    toastError("PDF konnte nicht gespeichert werden", e);
  }
}
</script>

<template>
  <Dialog v-model:open="offen" :title="titel" size="xl" @update:open="(v: boolean) => !v && emit('close')">
    <div class="h-[75vh]">
      <iframe v-if="url" :src="url" class="size-full rounded-md border border-border bg-white" title="PDF-Vorschau" />
      <p v-else-if="fehler" class="text-sm text-red-500">{{ fehler }}</p>
      <div v-else class="grid size-full place-items-center text-muted-foreground"><LoaderCircle class="size-6 animate-spin" /></div>
    </div>
    <template #footer>
      <span class="text-[11px] text-muted-foreground">Drucken über das geöffnete PDF (Strg+P im PDF-Programm).</span>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="emit('close')">Schließen</button>
      <button class="btn btn-primary" @click="speichern"><Download />Speichern & öffnen</button>
    </template>
  </Dialog>
</template>
