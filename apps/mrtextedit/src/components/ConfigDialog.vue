<script setup lang="ts">
import { ref, watch } from "vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { toastError } from "@mrtools/ui/lib/toast";
import { applyConfig, config, doc } from "@/lib/doc";
import { DEFAULT_CONFIG, TOOLBAR_HELP } from "@/lib/defaults";

const json = ref("");
const error = ref("");

watch(
  () => doc.configOpen,
  (open) => {
    if (!open) return;
    json.value = JSON.stringify(config.value, null, 2);
    error.value = "";
  },
);

async function apply() {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json.value);
  } catch (e) {
    error.value = `Kein gültiges JSON: ${(e as Error).message}`;
    return;
  }
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    error.value = "Erwartet wird ein Objekt { … }.";
    return;
  }
  try {
    await applyConfig(parsed, true);
    doc.configOpen = false;
  } catch (e) {
    toastError("Einstellungen nicht gespeichert", e);
  }
}
</script>

<template>
  <Dialog
    v-model:open="doc.configOpen"
    title="Toolleiste und Formate"
    description="Gleiches JSON wie beim Einbinden in Web und VCL (siehe README)"
    size="lg"
  >
    <textarea
      v-model="json"
      class="input h-[55vh] resize-none py-2 font-mono text-xs leading-relaxed"
      spellcheck="false"
      aria-label="Konfiguration als JSON"
      @input="error = ''"
      @keydown.ctrl.enter.prevent="apply"
    />
    <p v-if="error" class="mt-2 text-xs text-red-500">{{ error }}</p>
    <p v-else class="mt-2 text-xs text-muted-foreground">{{ TOOLBAR_HELP }}</p>
    <template #footer>
      <button class="btn btn-ghost" @click="json = JSON.stringify(DEFAULT_CONFIG, null, 2)">Standard</button>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="doc.configOpen = false">Abbrechen</button>
      <button class="btn btn-primary" @click="apply">Übernehmen</button>
    </template>
  </Dialog>
</template>
