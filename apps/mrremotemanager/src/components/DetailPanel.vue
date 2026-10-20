<script setup lang="ts">
import { computed } from "vue";
import { AppWindow, Clipboard, CopyPlus, FolderSync, Info, Pencil, Play, Star, Trash2, Unplug } from "@lucide/vue";
import KindIcon from "./KindIcon.vue";
import Tip from "@mrtools/ui/components/Tip";
import { askDelete, askEndSession, connect, copyAddress, duplicate, edit, focusSession, openSftp, toggleFavorite } from "@/lib/actions";
import { address, DEFAULT_BAUD, defaultPorts, displayName, formatLastUsed, kindLabels, kinds, probeTarget } from "@/lib/list";
import { passwords, probes, selected, sessionsOf, state, tools } from "@/lib/store";

const c = computed(() => selected.value);
const sessions = computed(() => (c.value ? sessionsOf(c.value) : []));
const probe = computed(() => (c.value && probeTarget(c.value) ? probes.get(c.value.id) : undefined));

const counts = computed(() => kinds.map((kind) => ({ kind, count: state.connections.filter((x) => x.kind === kind).length })).filter((k) => k.count));
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
              v-if="sessions.length"
              class="inline-flex shrink-0 items-center gap-1 rounded-full bg-emerald-500/15 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-400"
            >
              <span class="size-1.5 rounded-full bg-emerald-500" />Verbunden
            </span>
            <span
              v-else-if="probe"
              class="inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-medium"
              :class="probe.reachable ? 'bg-foreground/5 text-muted-foreground' : 'bg-zinc-500/15 text-zinc-600 dark:text-zinc-400'"
            >
              {{ probe.reachable ? "erreichbar" : "nicht erreichbar" }}
            </span>
          </p>
          <p class="text-xs text-muted-foreground">{{ kindLabels[c.kind] }}<template v-if="c.group"> · {{ c.group }}</template></p>
        </div>
        <Tip :text="c.favorite ? 'Kein Favorit mehr' : 'Zu Favoriten'">
          <button class="icon-btn" :aria-label="c.favorite ? 'Kein Favorit mehr' : 'Zu Favoriten'" @click="toggleFavorite(c)">
            <Star :class="c.favorite && 'fill-amber-400 text-amber-400'" />
          </button>
        </Tip>
      </div>

      <div class="flex flex-wrap gap-1.5 border-b border-border p-3">
        <button class="btn btn-primary h-7 text-xs" @click="connect(c)"><Play />Verbinden</button>
        <button v-if="c.kind === 'ssh' && tools.winscp" class="btn btn-outline h-7 text-xs" @click="openSftp(c)"><FolderSync />SFTP</button>
        <button class="btn btn-outline h-7 text-xs" @click="edit(c)"><Pencil />Bearbeiten</button>
        <button class="btn btn-outline h-7 text-xs" @click="duplicate(c)"><CopyPlus />Duplizieren</button>
        <button class="btn btn-danger h-7 text-xs" @click="askDelete(c)"><Trash2 />Löschen</button>
      </div>

      <section v-if="sessions.length" class="border-b border-border p-3">
        <h3 class="mb-1.5 px-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Offene Sitzungen</h3>
        <div v-for="a in sessions" :key="`${a.pid}-${a.address}`" class="flex items-center gap-2 rounded-md px-1 py-1 text-xs">
          <span class="min-w-0 flex-1 truncate">{{ a.process }} <span class="text-muted-foreground tabular-nums">· PID {{ a.pid }} · {{ a.address }}</span></span>
          <button class="btn btn-outline h-6 px-2 text-[11px]" @click="focusSession(a)"><AppWindow />Zeigen</button>
          <button class="btn btn-danger h-6 px-2 text-[11px]" @click="askEndSession(a)"><Unplug />Trennen</button>
        </div>
      </section>

      <dl class="grid grid-cols-[6rem_1fr] gap-x-3 gap-y-2 p-4 text-xs [&>dd]:min-w-0 [&>dd]:break-words [&>dt]:text-muted-foreground">
        <template v-if="c.kind === 'hyperv'">
          <dt>VM</dt>
          <dd class="select-text">{{ c.vmName }}</dd>
          <dt>Server</dt>
          <dd class="font-mono text-[11px] select-text">{{ c.host || "dieser PC" }}</dd>
        </template>
        <template v-else>
          <dt>{{ c.kind === "web" ? "Adresse" : c.kind === "serial" ? "Schnittstelle" : "Host" }}</dt>
          <dd class="flex items-start gap-1">
            <span class="min-w-0 flex-1 font-mono text-[11px] select-text">{{ c.host }}</span>
            <button class="icon-btn -my-1 size-6 [&_svg]:size-3.5" aria-label="Adresse kopieren" title="Adresse kopieren" @click="copyAddress(c)">
              <Clipboard />
            </button>
          </dd>
        </template>
        <template v-if="c.kind === 'serial'">
          <dt>Baudrate</dt>
          <dd class="tabular-nums">{{ c.baud ?? DEFAULT_BAUD }}</dd>
        </template>
        <template v-if="defaultPorts[c.kind]">
          <dt>Port</dt>
          <dd class="tabular-nums">{{ c.port ?? `${defaultPorts[c.kind]} (Standard)` }}</dd>
        </template>
        <template v-if="['rdp', 'ssh', 'telnet'].includes(c.kind)">
          <dt>Benutzer</dt>
          <dd class="select-text">{{ c.user || "–" }}</dd>
        </template>
        <template v-if="c.kind === 'rdp'">
          <dt>Passwort</dt>
          <dd>{{ passwords.get(c.host.trim()) ? "in Windows gespeichert" : "wird abgefragt" }}</dd>
          <dt>Anzeige</dt>
          <dd>
            {{ c.fullscreen ? "Vollbild" : c.width && c.height ? `Fenster ${c.width} × ${c.height}` : "Fenster" }}<template v-if="c.multimon">, alle Monitore</template>
          </dd>
          <dt>Weitergabe</dt>
          <dd>{{ [c.clipboard && "Zwischenablage", c.drives && "Laufwerke"].filter(Boolean).join(", ") || "nichts" }}</dd>
          <template v-if="c.gateway">
            <dt>Gateway</dt>
            <dd class="font-mono text-[11px] select-text">{{ c.gateway }}</dd>
          </template>
          <template v-if="c.admin">
            <dt>Sitzung</dt>
            <dd>Admin (/admin)</dd>
          </template>
        </template>
        <template v-if="c.kind === 'ssh'">
          <dt>Schlüssel</dt>
          <dd class="font-mono text-[11px] select-text">{{ c.keyFile || "Standard (~/.ssh)" }}</dd>
          <template v-if="c.jumpHost">
            <dt>Jump-Host</dt>
            <dd class="font-mono text-[11px] select-text">{{ c.jumpHost }}</dd>
          </template>
          <template v-if="c.forwards.trim()">
            <dt>Weiterleitung</dt>
            <dd class="font-mono text-[11px] whitespace-pre-line select-text">{{ c.forwards.trim() }}</dd>
          </template>
        </template>
        <template v-if="probe?.ips.length">
          <dt>IP-Adressen</dt>
          <dd class="font-mono text-[11px] select-text">{{ [...new Set(probe.ips)].join(", ") }}</dd>
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
      <div v-if="counts.length" class="grid grid-cols-3 gap-2">
        <div v-for="k in counts" :key="k.kind" class="rounded-lg border border-border bg-card p-3">
          <p class="text-2xl font-semibold tabular-nums">{{ k.count }}</p>
          <p class="text-xs text-muted-foreground">{{ kindLabels[k.kind] }}</p>
        </div>
      </div>
      <p class="flex items-start gap-2 rounded-lg border border-border p-3 text-xs text-muted-foreground">
        <Info class="mt-0.5 size-4 shrink-0 text-accent-text" />
        <span>
          Doppelklick oder <span class="kbd">Enter</span> verbindet. Ein grüner Punkt heißt „gerade verbunden“, ein grauer
          „nicht erreichbar“ (Prüfung alle 30 s). Passwörter für Remotedesktop liegen nur in Windows, nie in
          mrremotemanager.
        </span>
      </p>
    </div>
  </aside>
</template>
