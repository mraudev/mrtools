<script setup lang="ts">
import { computed } from "vue";
import { Copy, FolderSearch, Globe, Radio, X } from "@lucide/vue";
import ProcIcon from "./ProcIcon.vue";
import Tip from "@mrtools/ui/components/Tip";
import { askKill, copy, openInBrowser, showFolder } from "@/lib/actions";
import { isListening, serviceName } from "@/lib/rows";
import { rows, selectedRow, snapshot, state } from "@/lib/store";

const row = computed(() => selectedRow.value);

/** All sockets of the selected process: servers first, then connections. */
const sockets = computed(() => {
  const pid = row.value?.pid;
  const own = snapshot.value.sockets.filter((s) => s.pid === pid);
  const listening = [...new Map(own.filter(isListening).map((s) => [`${s.protocol}:${s.localPort}`, s])).values()]
    .sort((a, b) => a.localPort - b.localPort);
  return { listening, connections: own.filter((s) => !isListening(s)).length };
});

const totals = computed(() => {
  const all = snapshot.value.sockets;
  const listening = all.filter(isListening);
  return {
    tcp: new Set(listening.filter((s) => s.protocol === "TCP").map((s) => s.localPort)).size,
    udp: new Set(listening.filter((s) => s.protocol === "UDP").map((s) => s.localPort)).size,
    connections: all.filter((s) => s.protocol === "TCP" && s.state === "Verbunden").length,
    processes: new Set(all.map((s) => s.pid)).size,
    open: new Set(listening.filter((s) => s.protocol === "TCP" && (s.localAddress === "0.0.0.0" || s.localAddress === "::")).map((s) => s.localPort)).size,
  };
});

const select = (port: number, protocol: string) => {
  const match = rows.value.find((r) => r.port === port && r.protocol === protocol && r.pid === row.value?.pid);
  if (match) state.selected = match.key;
};
</script>

<template>
  <aside class="flex h-full min-w-0 flex-col overflow-y-auto">
    <template v-if="row">
      <div class="flex items-start gap-3 border-b border-border p-4">
        <ProcIcon :process="row.process" class="size-8" />
        <div class="min-w-0 flex-1">
          <p class="truncate font-semibold">{{ row.process.name }}</p>
          <p class="text-xs text-muted-foreground tabular-nums">PID {{ row.pid }}</p>
        </div>
      </div>

      <div class="flex flex-wrap gap-1.5 border-b border-border p-3">
        <button
          v-if="row.protocol === 'TCP' && row.state === 'Lauscht'"
          class="btn btn-outline h-7 text-xs"
          @click="openInBrowser(row.port)"
        >
          <Globe />localhost:{{ row.port }}
        </button>
        <button class="btn btn-outline h-7 text-xs" @click="showFolder(row)"><FolderSearch />Ordner</button>
        <button class="btn btn-danger h-7 text-xs" :disabled="row.pid <= 4" @click="askKill(row)"><X />Beenden</button>
      </div>

      <dl class="grid grid-cols-[5.5rem_1fr] gap-x-3 gap-y-2 p-4 text-xs [&>dd]:min-w-0 [&>dd]:break-words [&>dt]:text-muted-foreground">
        <dt>Programm</dt>
        <dd class="font-mono text-[11px] select-text">{{ row.process.exe || "– (kein Zugriff)" }}</dd>
        <dt>Ordner</dt>
        <dd class="font-mono text-[11px] select-text">{{ row.process.cwd || "–" }}</dd>
        <dt>Befehlszeile</dt>
        <dd class="flex items-start gap-1">
          <span class="max-h-40 min-w-0 flex-1 overflow-y-auto font-mono text-[11px] select-text">{{ row.process.cmd || "–" }}</span>
          <Tip v-if="row.process.cmd" text="Befehlszeile kopieren">
            <button class="icon-btn size-6 [&_svg]:size-3.5" aria-label="Befehlszeile kopieren" @click="copy(row.process.cmd, 'Befehlszeile')">
              <Copy />
            </button>
          </Tip>
        </dd>
        <dt>Erreichbar</dt>
        <dd>
          {{ row.reach }}
          <span class="text-muted-foreground">({{ row.addresses.join(", ") }})</span>
        </dd>
      </dl>

      <section class="border-t border-border p-4">
        <h3 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Ports dieses Prozesses</h3>
        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="s in sockets.listening"
            :key="`${s.protocol}:${s.localPort}`"
            class="rounded-md border px-2 py-0.5 font-mono text-xs tabular-nums transition-colors"
            :class="
              s.localPort === row.port && s.protocol === row.protocol
                ? 'border-accent/50 bg-accent/15 text-accent-text'
                : 'border-border hover:border-accent/50'
            "
            :title="[s.protocol, serviceName(s.localPort)].filter(Boolean).join(' · ')"
            @click="select(s.localPort, s.protocol)"
          >
            {{ s.localPort }}<span class="ml-1 text-[10px] text-muted-foreground">{{ s.protocol }}</span>
          </button>
        </div>
        <p v-if="sockets.connections" class="mt-2 text-xs text-muted-foreground">
          dazu {{ sockets.connections }} {{ sockets.connections === 1 ? "Verbindung" : "Verbindungen" }} – siehe „Alle Verbindungen“
        </p>
      </section>
    </template>

    <div v-else class="flex flex-col gap-4 p-4">
      <div class="grid grid-cols-2 gap-2">
        <div class="rounded-lg border border-border bg-card p-3">
          <p class="text-2xl font-semibold tabular-nums">{{ totals.tcp }}</p>
          <p class="text-xs text-muted-foreground">TCP-Ports lauschen</p>
        </div>
        <div class="rounded-lg border border-border bg-card p-3">
          <p class="text-2xl font-semibold tabular-nums">{{ totals.udp }}</p>
          <p class="text-xs text-muted-foreground">UDP-Ports offen</p>
        </div>
        <div class="rounded-lg border border-border bg-card p-3">
          <p class="text-2xl font-semibold tabular-nums">{{ totals.connections }}</p>
          <p class="text-xs text-muted-foreground">aktive Verbindungen</p>
        </div>
        <div class="rounded-lg border border-border bg-card p-3">
          <p class="text-2xl font-semibold tabular-nums">{{ totals.processes }}</p>
          <p class="text-xs text-muted-foreground">Prozesse mit Netzwerk</p>
        </div>
      </div>
      <p class="flex items-start gap-2 rounded-lg border border-border p-3 text-xs text-muted-foreground">
        <Radio class="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400" />
        <span>
          <b class="text-foreground">{{ totals.open }} TCP-Ports</b> sind aus dem ganzen Netzwerk erreichbar („Alle Netzwerke“),
          die übrigen nur von diesem PC. Eine Portnummer ins Suchfeld tippen zeigt, ob sie frei ist.
        </span>
      </p>
    </div>
  </aside>
</template>
