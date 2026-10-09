<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Activity, FileText, FolderSearch, Globe, Network, Package, Pause, Play, X } from "@lucide/vue";
import ConnectionList from "./ConnectionList.vue";
import ModuleList from "./ModuleList.vue";
import ProcIcon from "./ProcIcon.vue";
import Sparkline from "./Sparkline.vue";
import Segmented from "@mrtools/ui/components/Segmented";
import Tip from "@mrtools/ui/components/Tip";
import { api } from "@/lib/api";
import {
  PRIORITIES,
  askKill,
  searchOnline,
  setPriority,
  setSuspended,
  showInExplorer,
  showProperties,
} from "@/lib/actions";
import { formatBytes, formatCount, formatCpu, formatDateTime, formatElapsed, formatRate } from "@/lib/format";
import { byPid, historyOf, isEnded, reveal, selectedProc, state } from "@/lib/store";
import type { Details } from "@/lib/types";

type Tab = "overview" | "modules" | "network";
const tab = ref<Tab>("overview");
const tabs = [
  { value: "overview" as const, label: "Übersicht", icon: Activity },
  { value: "modules" as const, label: "Module", icon: Package },
  { value: "network" as const, label: "Netzwerk", icon: Network },
];

const proc = computed(() => selectedProc.value);
const ended = computed(() => !!proc.value && isEnded(proc.value.pid));
const history = computed(() => (proc.value ? historyOf(proc.value.pid) : undefined));
const parent = computed(() => (proc.value?.parent ? byPid.value.get(proc.value.parent) : undefined));

const details = ref<Details | null>(null);
async function loadDetails(pid: number) {
  const result = await api.details(pid).catch(() => null);
  if (proc.value?.pid === pid) details.value = result;
}
watch(
  () => proc.value?.pid,
  (pid) => {
    details.value = null;
    if (pid !== undefined) loadDetails(pid);
  },
  { immediate: true },
);

async function changePriority(event: Event) {
  const select = event.target as HTMLSelectElement;
  if (!proc.value) return;
  await setPriority(proc.value, Number(select.value));
  loadDetails(proc.value.pid);
}

const memoryMax = computed(() => {
  const peak = Math.max(...(history.value?.memory ?? [0]), 1);
  // Round the axis up to 1×, 1.5× or 2× a power of two so it does not jump with every sample.
  const magnitude = 2 ** Math.floor(Math.log2(peak));
  return [1, 1.5, 2].map((f) => f * magnitude).find((m) => m >= peak * 1.1) ?? magnitude * 2;
});

const runtime = computed(() => {
  void state.tick;
  return proc.value?.startTime ? formatElapsed(Date.now() - proc.value.startTime * 1000) : "–";
});
</script>

<template>
  <div v-if="proc" class="flex h-full min-w-0 flex-col gap-3 p-3">
    <div class="flex items-start gap-2.5">
      <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-foreground/5 [&>*]:!size-6">
        <ProcIcon :exe="proc.exe" />
      </div>
      <div class="min-w-0 flex-1">
        <p class="flex items-center gap-2 font-semibold">
          <span class="truncate" :title="proc.name">{{ proc.name }}</span>
          <span v-if="ended" class="shrink-0 rounded bg-red-500/15 px-1.5 text-[11px] font-medium text-red-600 dark:text-red-400">
            Beendet
          </span>
          <span v-else-if="proc.suspended" class="shrink-0 rounded bg-foreground/8 px-1.5 text-[11px] font-medium text-muted-foreground">
            Angehalten
          </span>
        </p>
        <p class="truncate text-xs text-muted-foreground tabular-nums">PID {{ proc.pid }} · {{ proc.user || "unbekannter Benutzer" }}</p>
      </div>
      <Tip text="Auswahl aufheben">
        <button class="icon-btn" aria-label="Auswahl aufheben" @click="state.selected = null"><X /></button>
      </Tip>
    </div>

    <div class="flex flex-wrap items-center gap-1.5">
      <button class="btn h-7 bg-red-600 px-2.5 text-white hover:bg-red-600/85" :disabled="ended" @click="askKill(proc)">
        <X />Beenden
      </button>
      <button v-if="proc.suspended" class="btn btn-outline h-7 px-2.5" :disabled="ended" @click="setSuspended(proc, false)">
        <Play />Fortsetzen
      </button>
      <button v-else class="btn btn-outline h-7 px-2.5" :disabled="ended" @click="setSuspended(proc, true)">
        <Pause />Anhalten
      </button>
      <div class="flex-1" />
      <Tip text="Dateipfad öffnen">
        <button class="icon-btn" aria-label="Dateipfad öffnen" :disabled="!proc.exe" @click="showInExplorer(proc.exe)">
          <FolderSearch />
        </button>
      </Tip>
      <Tip text="Eigenschaften">
        <button class="icon-btn" aria-label="Eigenschaften" :disabled="!proc.exe" @click="showProperties(proc.exe)">
          <FileText />
        </button>
      </Tip>
      <Tip text="Online suchen">
        <button class="icon-btn" aria-label="Online suchen" @click="searchOnline(proc.name)"><Globe /></button>
      </Tip>
    </div>

    <Segmented v-model="tab" :options="tabs" class="self-start" />

    <div v-if="tab === 'overview'" class="min-h-0 flex-1 space-y-3 overflow-y-auto">
      <div class="grid grid-cols-2 gap-2">
        <Sparkline
          label="CPU"
          :values="history?.cpu ?? []"
          :max="100"
          :format="(v) => `${formatCpu(v)} %`"
          color="var(--series-1)"
          :step="state.interval / 1000"
        />
        <Sparkline
          label="Arbeitsspeicher"
          :values="history?.memory ?? []"
          :max="memoryMax"
          :format="formatBytes"
          color="var(--series-2)"
          :step="state.interval / 1000"
        />
      </div>

      <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-[13px] [&>dt]:text-muted-foreground [&>dd]:min-w-0 [&>dd]:tabular-nums">
        <dt>Pfad</dt>
        <dd class="font-mono text-xs break-all select-text">{{ proc.exe || "–" }}</dd>
        <dt>Befehlszeile</dt>
        <dd class="max-h-24 overflow-y-auto font-mono text-xs break-all select-text">{{ proc.cmd || "–" }}</dd>
        <dt>Übergeordnet</dt>
        <dd>
          <button v-if="parent" class="inline-flex max-w-full items-center gap-1.5 rounded hover:underline" @click="reveal(parent.pid)">
            <ProcIcon :exe="parent.exe" />
            <span class="truncate">{{ parent.name }}</span>
            <span class="text-muted-foreground">({{ parent.pid }})</span>
          </button>
          <span v-else class="text-muted-foreground">{{ proc.parent ? `PID ${proc.parent} (beendet)` : "–" }}</span>
        </dd>
        <dt>Gestartet</dt>
        <dd>{{ formatDateTime(proc.startTime) }}</dd>
        <dt>Laufzeit</dt>
        <dd>{{ runtime }}</dd>
        <dt>CPU-Zeit</dt>
        <dd>{{ formatElapsed(proc.cpuTime) }}</dd>
        <dt>Priorität</dt>
        <dd>
          <select
            v-if="details?.priority"
            class="input h-7 w-auto py-0 pr-7"
            :value="details.priority"
            :disabled="ended"
            @change="changePriority"
          >
            <option v-for="p in PRIORITIES" :key="p.value" :value="p.value">{{ p.label }}</option>
          </select>
          <span v-else class="text-muted-foreground">nicht lesbar</span>
        </dd>
        <dt>Erhöhte Rechte</dt>
        <dd>{{ details?.elevated == null ? "unbekannt" : details.elevated ? "Ja" : "Nein" }}</dd>
        <dt>Threads · Handles</dt>
        <dd>{{ formatCount(proc.threads) }} · {{ formatCount(proc.handles) }}</dd>
        <dt>Privater Arbeitssatz</dt>
        <dd>{{ formatBytes(proc.memory) }}</dd>
        <dt>Arbeitssatz gesamt</dt>
        <dd>{{ formatBytes(proc.workingSet) }}</dd>
        <dt>Zugesicherter Speicher</dt>
        <dd>{{ formatBytes(proc.private) }}</dd>
        <dt>Datenträger</dt>
        <dd>{{ formatRate(proc.diskRead) }} lesen · {{ formatRate(proc.diskWrite) }} schreiben</dd>
      </dl>
    </div>
    <ModuleList v-else-if="tab === 'modules'" :pid="proc.pid" class="min-h-0 flex-1" />
    <ConnectionList v-else :pid="proc.pid" class="min-h-0 flex-1" />
  </div>
</template>
