<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Globe, Monitor, SquareTerminal } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import Segmented from "@mrtools/ui/components/Segmented";
import { saveDraft } from "@/lib/actions";
import { defaultPorts, validate } from "@/lib/list";
import { groupNames, state } from "@/lib/store";
import type { Kind } from "@/lib/types";

const open = computed({
  get: () => state.draft !== null,
  set: (value) => !value && (state.draft = null),
});

const kinds = [
  { value: "rdp" as Kind, label: "Remotedesktop", icon: Monitor },
  { value: "ssh" as Kind, label: "SSH", icon: SquareTerminal },
  { value: "web" as Kind, label: "Web", icon: Globe },
];

const kind = computed({
  get: () => state.draft?.kind ?? "rdp",
  set: (value: Kind) => state.draft && (state.draft.kind = value),
});

/** The port field as text, so it can be empty (= default port). */
const port = computed({
  get: () => (state.draft?.port == null ? "" : String(state.draft.port)),
  set: (value: string) => {
    if (!state.draft) return;
    const text = value.trim();
    state.draft.port = text ? Number(text) : null;
  },
});

const tried = ref(false);
const error = computed(() => (state.draft ? validate(state.draft) : null));

watch(open, (value) => value && (tried.value = false));

function submit() {
  tried.value = true;
  if (!error.value) saveDraft();
}
</script>

<template>
  <Dialog v-model:open="open" :title="state.isNew ? 'Neue Verbindung' : 'Verbindung bearbeiten'" size="md">
    <form v-if="state.draft" id="edit-form" class="flex flex-col gap-4" @submit.prevent="submit">
      <Segmented v-model="kind" :options="kinds" class="self-start" />

      <div class="grid grid-cols-2 gap-3">
        <label class="block">
          <span class="label">Name</span>
          <input v-model="state.draft.name" class="input" placeholder="z. B. Server Büro" spellcheck="false" autofocus />
        </label>
        <label class="block">
          <span class="label">Gruppe</span>
          <input v-model="state.draft.group" class="input" list="group-names" placeholder="optional" spellcheck="false" />
          <datalist id="group-names">
            <option v-for="name in groupNames" :key="name" :value="name" />
          </datalist>
        </label>
      </div>

      <div v-if="state.draft.kind === 'web'">
        <label class="block">
          <span class="label">Adresse</span>
          <input v-model="state.draft.host" class="input font-mono" placeholder="https://nas.local:5001" spellcheck="false" />
        </label>
      </div>
      <template v-else>
        <div class="grid grid-cols-[1fr_7rem] gap-3">
          <label class="block">
            <span class="label">Host oder IP-Adresse</span>
            <input v-model="state.draft.host" class="input font-mono" placeholder="server.local" spellcheck="false" />
          </label>
          <label class="block">
            <span class="label">Port</span>
            <input v-model="port" class="input font-mono" inputmode="numeric" :placeholder="String(defaultPorts[state.draft.kind])" />
          </label>
        </div>
        <label class="block">
          <span class="label">Benutzer</span>
          <input
            v-model="state.draft.user"
            class="input font-mono"
            :placeholder="state.draft.kind === 'rdp' ? 'DOMÄNE\\benutzer oder benutzer@domäne (optional)' : 'optional'"
            spellcheck="false"
          />
        </label>
      </template>

      <div v-if="state.draft.kind === 'rdp'" class="flex flex-wrap gap-x-6 gap-y-2 text-sm">
        <label class="inline-flex items-center gap-2">
          <input v-model="state.draft.fullscreen" type="checkbox" class="size-4 accent-[var(--accent)]" />Vollbild
        </label>
        <label class="inline-flex items-center gap-2">
          <input v-model="state.draft.multimon" type="checkbox" class="size-4 accent-[var(--accent)]" />Alle Monitore verwenden
        </label>
      </div>

      <label v-if="state.draft.kind === 'ssh'" class="block">
        <span class="label">Schlüsseldatei</span>
        <input v-model="state.draft.keyFile" class="input font-mono" placeholder="optional, z. B. C:\Users\…\.ssh\id_ed25519" spellcheck="false" />
      </label>

      <label class="block">
        <span class="label">Notizen</span>
        <textarea v-model="state.draft.notes" class="input h-20 resize-none py-1.5" placeholder="optional" />
      </label>
    </form>

    <template #footer>
      <p v-if="tried && error" class="text-xs text-red-500">{{ error }}</p>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="state.draft = null">Abbrechen</button>
      <button class="btn btn-primary" type="submit" form="edit-form">Speichern</button>
    </template>
  </Dialog>
</template>
