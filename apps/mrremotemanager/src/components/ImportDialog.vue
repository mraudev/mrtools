<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FilePlus, LoaderCircle, SearchX } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import KindIcon from "./KindIcon.vue";
import { toastError } from "@mrtools/ui/lib/toast";
import { addImported } from "@/lib/actions";
import { api } from "@/lib/api";
import { defaultPorts, fromCandidate, isSaved } from "@/lib/list";
import { groupNames, state } from "@/lib/store";
import type { Candidate } from "@/lib/types";

const open = computed({
  get: () => state.importing,
  set: (value) => (state.importing = value),
});

interface Row {
  key: string;
  candidate: Candidate;
  saved: boolean;
  checked: boolean;
}

const rows = ref<Row[]>([]);
const loading = ref(false);
const group = ref("");

const keyOf = (c: Candidate) => `${c.kind}|${c.host.toLowerCase()}|${c.port ?? defaultPorts[c.kind]}`;

/** Adds found connections; the same server from two sources is listed once. */
function add(found: Candidate[]) {
  const keys = new Set(rows.value.map((r) => r.key));
  for (const candidate of found) {
    const key = keyOf(candidate);
    if (keys.has(key)) continue;
    keys.add(key);
    const saved = isSaved(candidate, state.connections);
    rows.value.push({ key, candidate, saved, checked: !saved });
  }
}

watch(open, async (value) => {
  if (!value) return;
  rows.value = [];
  group.value = "";
  loading.value = true;
  try {
    add(await api.importSources());
  } catch (e) {
    toastError("Suche fehlgeschlagen", e);
  } finally {
    loading.value = false;
  }
});

async function addRdpFiles() {
  try {
    const paths = await openDialog({ multiple: true, filters: [{ name: "Remotedesktop", extensions: ["rdp"] }] });
    if (!paths?.length) return;
    add(await api.importRdpFiles(paths));
  } catch (e) {
    toastError("RDP-Dateien konnten nicht gelesen werden", e);
  }
}

const sources = computed(() => {
  const map = new Map<string, Row[]>();
  for (const row of rows.value) map.set(row.candidate.source, [...(map.get(row.candidate.source) ?? []), row]);
  return [...map.entries()];
});

const chosen = computed(() => rows.value.filter((r) => r.checked && !r.saved));

function setAll(list: Row[], checked: boolean) {
  for (const row of list) if (!row.saved) row.checked = checked;
}

function submit() {
  addImported(chosen.value.map((r) => fromCandidate(r.candidate, group.value.trim())));
}
</script>

<template>
  <Dialog v-model:open="open" title="Von diesem PC importieren" description="Remotedesktop-Verlauf, ~/.ssh/config, PuTTY und RDP-Dateien" size="lg">
    <div v-if="loading" class="grid h-40 place-items-center text-muted-foreground"><LoaderCircle class="size-5 animate-spin" /></div>
    <div v-else-if="!rows.length" class="flex h-40 flex-col items-center justify-center gap-2 text-muted-foreground">
      <SearchX class="size-8 opacity-60" />
      <p>Nichts gefunden – RDP-Dateien lassen sich unten hinzufügen.</p>
    </div>
    <div v-else class="flex flex-col gap-4">
      <section v-for="[source, list] in sources" :key="source">
        <h3 class="mb-1 flex items-center gap-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {{ source }}
          <span class="font-normal tabular-nums opacity-70">{{ list.length }}</span>
          <div class="flex-1" />
          <button class="font-normal normal-case hover:text-foreground" @click="setAll(list, true)">alle</button>
          <button class="font-normal normal-case hover:text-foreground" @click="setAll(list, false)">keine</button>
        </h3>
        <label
          v-for="row in list"
          :key="row.key"
          class="flex h-10 items-center gap-3 rounded-md px-2"
          :class="row.saved ? 'opacity-50' : 'cursor-pointer hover:bg-foreground/5'"
        >
          <input v-model="row.checked" type="checkbox" class="size-4 accent-[var(--accent)]" :disabled="row.saved" />
          <KindIcon :kind="row.candidate.kind" class="size-6" />
          <div class="min-w-0 flex-1">
            <p class="truncate text-[13px] font-medium">{{ row.candidate.name }}</p>
            <p class="truncate font-mono text-[11px] text-muted-foreground">
              <template v-if="row.candidate.user">{{ row.candidate.user }}@</template>{{ row.candidate.host
              }}<template v-if="row.candidate.port">:{{ row.candidate.port }}</template>
              <template v-if="row.candidate.detail"> · {{ row.candidate.detail }}</template>
            </p>
          </div>
          <span v-if="row.saved" class="shrink-0 text-[11px] text-muted-foreground">schon gespeichert</span>
        </label>
      </section>
    </div>

    <template #footer>
      <button class="btn btn-outline" :disabled="loading" @click="addRdpFiles"><FilePlus />RDP-Dateien …</button>
      <div class="flex-1" />
      <input v-model="group" class="input h-8 w-44" list="import-groups" placeholder="In Gruppe (optional)" spellcheck="false" />
      <datalist id="import-groups">
        <option v-for="name in groupNames" :key="name" :value="name" />
      </datalist>
      <button class="btn btn-primary" :disabled="!chosen.length" @click="submit">
        {{ chosen.length ? `${chosen.length} importieren` : "Importieren" }}
      </button>
    </template>
  </Dialog>
</template>
