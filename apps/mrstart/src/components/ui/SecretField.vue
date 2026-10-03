<script setup lang="ts">
import { onMounted, ref } from "vue";
import { KeyRound, Trash2 } from "@lucide/vue";
import Tip from "./Tip.vue";
import { api } from "@/lib/api";
import { refresh } from "@/lib/store";
import { toast, toastError } from "@/lib/toast";
import type { SecretName } from "@/lib/types";

/**
 * Write-only token input: the value goes straight to the credential store and
 * is never shown again; only whether a token exists is displayed.
 */
const props = defineProps<{ name: SecretName; id: string }>();

const stored = ref(false);
const draft = ref("");
const busy = ref(false);

async function loadStatus() {
  stored.value = (await api.secretStatus())[props.name];
}
onMounted(loadStatus);

async function save() {
  busy.value = true;
  try {
    await api.setSecret(props.name, draft.value);
    draft.value = "";
    await loadStatus();
    toast("success", "Token sicher gespeichert");
    refresh();
  } catch (e) {
    toastError("Token konnte nicht gespeichert werden", e);
  } finally {
    busy.value = false;
  }
}

async function remove() {
  busy.value = true;
  try {
    await api.deleteSecret(props.name);
    await loadStatus();
    toast("success", "Token entfernt");
    refresh();
  } catch (e) {
    toastError("Token konnte nicht entfernt werden", e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <form class="flex items-center gap-2" @submit.prevent="save">
    <input
      :id="id"
      v-model="draft"
      type="password"
      class="input font-mono text-[13px]"
      :placeholder="stored ? 'Hinterlegt – zum Ersetzen neues Token eingeben' : 'Nicht hinterlegt (optional)'"
      autocomplete="off"
      spellcheck="false"
    />
    <span
      v-if="stored"
      class="inline-flex h-8 shrink-0 items-center gap-1.5 rounded-md bg-emerald-500/10 px-2 text-xs text-emerald-500"
    >
      <KeyRound class="size-3.5" />hinterlegt
    </span>
    <button type="submit" class="btn btn-outline" :disabled="busy || !draft.trim()">Speichern</button>
    <Tip v-if="stored" text="Token entfernen">
      <button type="button" class="icon-btn" aria-label="Token entfernen" :disabled="busy" @click="remove">
        <Trash2 />
      </button>
    </Tip>
  </form>
</template>
