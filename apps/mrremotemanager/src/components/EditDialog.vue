<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { KeyRound, TriangleAlert } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import KindIcon from "./KindIcon.vue";
import { removePassword, saveDraft } from "@/lib/actions";
import { DEFAULT_BAUD, defaultPorts, kindLabels, kinds, validate } from "@/lib/list";
import { groupNames, passwords, state, tools } from "@/lib/store";

const open = computed({
  get: () => state.draft !== null,
  set: (value) => !value && (state.draft = null),
});

const draft = computed(() => state.draft!);

/** A number field as text, so it can be empty (= default). */
function numberField(key: "port" | "width" | "height" | "baud") {
  return computed({
    get: () => (state.draft?.[key] == null ? "" : String(state.draft[key])),
    set: (value: string) => {
      if (!state.draft) return;
      const text = value.trim();
      state.draft[key] = text ? Number(text) : null;
    },
  });
}
const port = numberField("port");
const width = numberField("width");
const height = numberField("height");
const baud = numberField("baud");

/** Besides the default (9600). */
const otherBauds = [1200, 2400, 4800, 19200, 38400, 57600, 115200];

const password = ref("");
const changingPassword = ref(false);
const passwordStored = computed(() => draft.value.kind === "rdp" && !!passwords.get(draft.value.host.trim()));

const tried = ref(false);
const saving = ref(false);
const error = computed(() => {
  if (!state.draft) return null;
  if (draft.value.kind === "rdp" && password.value && !draft.value.user.trim())
    return "Für das Passwort wird ein Benutzername gebraucht.";
  return validate(draft.value);
});

const missingTool = computed(() => {
  const kind = state.draft?.kind;
  if (kind === "vnc" && !tools.value.vnc) return "Kein VNC-Viewer gefunden – TigerVNC, RealVNC, UltraVNC oder TightVNC installieren.";
  if ((kind === "telnet" || kind === "serial") && !tools.value.putty) return "PuTTY nicht gefunden – zum Verbinden bitte installieren.";
  return null;
});

watch(open, (value) => {
  if (!value) return;
  tried.value = false;
  password.value = "";
  changingPassword.value = false;
});

async function submit() {
  tried.value = true;
  if (error.value || saving.value) return;
  saving.value = true;
  try {
    await saveDraft(password.value);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open" :title="state.isNew ? 'Neue Verbindung' : 'Verbindung bearbeiten'" size="lg">
    <form v-if="state.draft" id="edit-form" class="flex flex-col gap-4" @submit.prevent="submit">
      <div class="grid grid-cols-7 gap-1.5" role="radiogroup" aria-label="Art">
        <button
          v-for="kind in kinds"
          :key="kind"
          type="button"
          role="radio"
          :aria-checked="draft.kind === kind"
          class="flex flex-col items-center gap-1 rounded-lg border px-1 py-2 text-[11px] transition-colors"
          :class="draft.kind === kind ? 'border-accent bg-accent/10 font-medium text-foreground' : 'border-border text-muted-foreground hover:bg-foreground/5'"
          @click="draft.kind = kind"
        >
          <KindIcon :kind="kind" class="size-7" />
          {{ kindLabels[kind] }}
        </button>
      </div>

      <p v-if="missingTool" class="flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 p-2.5 text-xs text-amber-700 dark:text-amber-400">
        <TriangleAlert class="mt-0.5 size-4 shrink-0" />{{ missingTool }}
      </p>

      <div class="grid grid-cols-2 gap-3">
        <label class="block">
          <span class="label">Name</span>
          <input v-model="draft.name" class="input" placeholder="z. B. Server Büro" spellcheck="false" autofocus />
        </label>
        <label class="block">
          <span class="label">Gruppe</span>
          <input v-model="draft.group" class="input" list="group-names" placeholder="optional" spellcheck="false" />
          <datalist id="group-names">
            <option v-for="name in groupNames" :key="name" :value="name" />
          </datalist>
        </label>
      </div>

      <!-- Address -->
      <label v-if="draft.kind === 'web'" class="block">
        <span class="label">Adresse</span>
        <input v-model="draft.host" class="input font-mono" placeholder="https://nas.local:5001" spellcheck="false" />
      </label>
      <div v-else-if="draft.kind === 'serial'" class="grid grid-cols-2 gap-3">
        <label class="block">
          <span class="label">Schnittstelle</span>
          <input v-model="draft.host" class="input font-mono" placeholder="COM3" spellcheck="false" />
        </label>
        <label class="block">
          <span class="label">Baudrate</span>
          <select v-model="baud" class="input">
            <option value="">{{ DEFAULT_BAUD }} (Standard)</option>
            <option v-for="rate in otherBauds" :key="rate" :value="String(rate)">{{ rate }}</option>
          </select>
        </label>
      </div>
      <div v-else-if="draft.kind === 'hyperv'" class="grid grid-cols-2 gap-3">
        <label class="block">
          <span class="label">Name der VM</span>
          <input v-model="draft.vmName" class="input" placeholder="z. B. Windows 11 Dev" spellcheck="false" />
        </label>
        <label class="block">
          <span class="label">Hyper-V-Server</span>
          <input v-model="draft.host" class="input font-mono" placeholder="leer: dieser PC" spellcheck="false" />
        </label>
      </div>
      <template v-else>
        <div class="grid grid-cols-[1fr_7rem] gap-3">
          <label class="block">
            <span class="label">Host oder IP-Adresse</span>
            <input v-model="draft.host" class="input font-mono" placeholder="server.local" spellcheck="false" />
          </label>
          <label class="block">
            <span class="label">Port</span>
            <input v-model="port" class="input font-mono" inputmode="numeric" :placeholder="String(defaultPorts[draft.kind])" />
          </label>
        </div>
        <label v-if="draft.kind !== 'vnc'" class="block">
          <span class="label">Benutzer</span>
          <input
            v-model="draft.user"
            class="input font-mono"
            :placeholder="draft.kind === 'rdp' ? 'DOMÄNE\\benutzer oder benutzer@domäne (optional)' : 'optional'"
            spellcheck="false"
          />
        </label>
      </template>

      <!-- Remotedesktop -->
      <template v-if="draft.kind === 'rdp'">
        <div class="rounded-lg border border-border p-3">
          <p class="label flex items-center gap-1.5"><KeyRound class="size-3.5" />Passwort</p>
          <div v-if="passwordStored && !changingPassword" class="flex items-center gap-2 text-sm">
            <span class="flex-1 text-muted-foreground">In der Windows-Anmeldeinformationsverwaltung gespeichert.</span>
            <button type="button" class="btn btn-outline h-7 text-xs" @click="changingPassword = true">Ändern</button>
            <button type="button" class="btn btn-danger h-7 text-xs" @click="removePassword(draft.host.trim())">Entfernen</button>
          </div>
          <template v-else>
            <input
              v-model="password"
              type="password"
              autocomplete="new-password"
              class="input"
              :placeholder="passwordStored ? 'Neues Passwort' : 'optional – sonst fragt Remotedesktop nach'"
            />
            <p class="mt-1 text-[11px] text-muted-foreground">
              Wird nur in Windows gespeichert (wie „Anmeldedaten speichern“ in Remotedesktop), nicht in mrremotemanager.
            </p>
          </template>
        </div>

        <div class="grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
          <label class="inline-flex items-center gap-2"><input v-model="draft.fullscreen" type="checkbox" class="size-4 accent-[var(--accent)]" />Vollbild</label>
          <label class="inline-flex items-center gap-2"><input v-model="draft.multimon" type="checkbox" class="size-4 accent-[var(--accent)]" />Alle Monitore verwenden</label>
          <label class="inline-flex items-center gap-2"><input v-model="draft.clipboard" type="checkbox" class="size-4 accent-[var(--accent)]" />Zwischenablage teilen</label>
          <label class="inline-flex items-center gap-2"><input v-model="draft.drives" type="checkbox" class="size-4 accent-[var(--accent)]" />Laufwerke durchreichen</label>
          <label class="inline-flex items-center gap-2"><input v-model="draft.admin" type="checkbox" class="size-4 accent-[var(--accent)]" />Admin-Sitzung (/admin)</label>
        </div>

        <div class="grid grid-cols-[1fr_1fr_2fr] gap-3">
          <label class="block">
            <span class="label">Breite</span>
            <input v-model="width" class="input font-mono" inputmode="numeric" placeholder="Standard" :disabled="draft.fullscreen" />
          </label>
          <label class="block">
            <span class="label">Höhe</span>
            <input v-model="height" class="input font-mono" inputmode="numeric" placeholder="Standard" :disabled="draft.fullscreen" />
          </label>
          <label class="block">
            <span class="label">RD-Gateway</span>
            <input v-model="draft.gateway" class="input font-mono" placeholder="optional, z. B. gw.firma.de" spellcheck="false" />
          </label>
        </div>
      </template>

      <!-- SSH -->
      <template v-if="draft.kind === 'ssh'">
        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="label">Schlüsseldatei</span>
            <input v-model="draft.keyFile" class="input font-mono" placeholder="optional, z. B. C:\Users\…\.ssh\id_ed25519" spellcheck="false" />
          </label>
          <label class="block">
            <span class="label">Jump-Host (-J)</span>
            <input v-model="draft.jumpHost" class="input font-mono" placeholder="optional, z. B. admin@bastion" spellcheck="false" />
          </label>
        </div>
        <label class="block">
          <span class="label">Portweiterleitungen – eine pro Zeile</span>
          <textarea
            v-model="draft.forwards"
            class="input h-16 resize-none py-1.5 font-mono"
            placeholder="L 8080:localhost:80&#10;D 1080"
            spellcheck="false"
          />
        </label>
      </template>

      <label class="block">
        <span class="label">Notizen</span>
        <textarea v-model="draft.notes" class="input h-16 resize-none py-1.5" placeholder="optional" />
      </label>
    </form>

    <template #footer>
      <p v-if="tried && error" class="text-xs text-red-500">{{ error }}</p>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="state.draft = null">Abbrechen</button>
      <button class="btn btn-primary" type="submit" form="edit-form" :disabled="saving">Speichern</button>
    </template>
  </Dialog>
</template>
