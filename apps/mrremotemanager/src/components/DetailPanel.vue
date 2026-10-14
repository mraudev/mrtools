<script setup lang="ts">
import { computed } from "vue";
import { Clipboard, CopyPlus, Info, Pencil, Play, Trash2 } from "@lucide/vue";
import KindIcon from "./KindIcon.vue";
import { askDelete, connect, copyAddress, duplicate, edit } from "@/lib/actions";
import { address, defaultPorts, displayName, formatLastUsed, kindLabels } from "@/lib/list";
import { connectedIds, selected, state } from "@/lib/store";

const c = computed(() => selected.value);
const counts = computed(() => ({
  rdp: state.connections.filter((x) => x.kind === "rdp").length,
  ssh: state.connections.filter((x) => x.kind === "ssh").length,
  web: state.connections.filter((x) => x.kind === "web").length,
}));
</script>

<template>
  <aside class="flex h-full min-w-0 flex-col overflow-y-auto">
    <template v-if="c">
      <div class="flex items-start gap-3 border-b border-border p-4">
        <KindIcon :kind="c.kind" class="size-9" />
        <div class="min-w-0 flex-1">
          <p class="flex items-center gap-2 font-semibold">
            <span class="truncate">{{ displayName(c) }}</span>
            <span
              v-if="connectedIds.has(c.id)"
              class="inline-flex shrink-0 items-center gap-1 rounded-full bg-emerald-500/15 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-400"
            >
              <span class="size-1.5 rounded-full bg-emerald-500" />Verbunden
            </span>
          </p>
          <p class="text-xs text-muted-foreground">{{ kindLabels[c.kind] }}<template v-if="c.group"> · {{ c.group }}</template></p>
        </div>
      </div>

      <div class="flex flex-wrap gap-1.5 border-b border-border p-3">
        <button class="btn btn-primary h-7 text-xs" @click="connect(c)"><Play />Verbinden</button>
        <button class="btn btn-outline h-7 text-xs" @click="edit(c)"><Pencil />Bearbeiten</button>
        <button class="btn btn-outline h-7 text-xs" @click="duplicate(c)"><CopyPlus />Duplizieren</button>
        <button class="btn btn-danger h-7 text-xs" @click="askDelete(c)"><Trash2 />Löschen</button>
      </div>

      <dl class="grid grid-cols-[6rem_1fr] gap-x-3 gap-y-2 p-4 text-xs [&>dd]:min-w-0 [&>dd]:break-words [&>dt]:text-muted-foreground">
        <dt>{{ c.kind === "web" ? "Adresse" : "Host" }}</dt>
        <dd class="flex items-start gap-1">
          <span class="min-w-0 flex-1 font-mono text-[11px] select-text">{{ c.host }}</span>
          <button class="icon-btn -my-1 size-6 [&_svg]:size-3.5" aria-label="Adresse kopieren" title="Adresse kopieren" @click="copyAddress(c)">
            <Clipboard />
          </button>
        </dd>
        <template v-if="c.kind !== 'web'">
          <dt>Port</dt>
          <dd class="tabular-nums">{{ c.port ?? `${defaultPorts[c.kind]} (Standard)` }}</dd>
          <dt>Benutzer</dt>
          <dd class="select-text">{{ c.user || "–" }}</dd>
        </template>
        <template v-if="c.kind === 'rdp'">
          <dt>Anzeige</dt>
          <dd>{{ c.fullscreen ? "Vollbild" : "Fenster" }}<template v-if="c.multimon">, alle Monitore</template></dd>
        </template>
        <template v-if="c.kind === 'ssh'">
          <dt>Schlüssel</dt>
          <dd class="font-mono text-[11px] select-text">{{ c.keyFile || "Standard (~/.ssh)" }}</dd>
        </template>
        <dt>Aufruf</dt>
        <dd class="font-mono text-[11px] text-muted-foreground select-text">{{ address(c) }}</dd>
        <dt>Zuletzt</dt>
        <dd>{{ formatLastUsed(c.lastUsed) }}</dd>
      </dl>

      <section v-if="c.notes" class="border-t border-border p-4">
        <h3 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Notizen</h3>
        <p class="text-[13px] whitespace-pre-wrap select-text">{{ c.notes }}</p>
      </section>
    </template>

    <div v-else class="flex flex-col gap-4 p-4">
      <div class="grid grid-cols-3 gap-2">
        <div v-for="kind in (['rdp', 'ssh', 'web'] as const)" :key="kind" class="rounded-lg border border-border bg-card p-3">
          <p class="text-2xl font-semibold tabular-nums">{{ counts[kind] }}</p>
          <p class="text-xs text-muted-foreground">{{ kindLabels[kind] }}</p>
        </div>
      </div>
      <p class="flex items-start gap-2 rounded-lg border border-border p-3 text-xs text-muted-foreground">
        <Info class="mt-0.5 size-4 shrink-0 text-accent-text" />
        <span>
          Doppelklick oder <span class="kbd">Enter</span> verbindet. Passwörter speichert mrremotemanager nicht –
          Remotedesktop merkt sie sich auf Wunsch selbst („Anmeldedaten speichern“), SSH nutzt am besten einen Schlüssel.
        </span>
      </p>
    </div>
  </aside>
</template>
