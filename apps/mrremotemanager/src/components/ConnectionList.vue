<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { ContextMenuRoot, ContextMenuTrigger } from "reka-ui";
import { AppWindow, ArrowDownAZ, Clock, MonitorSmartphone, Plus, Save, SearchX, Star, Unplug } from "@lucide/vue";
import Segmented from "@mrtools/ui/components/Segmented";
import Tip from "@mrtools/ui/components/Tip";
import KindIcon from "./KindIcon.vue";
import RowMenu from "./RowMenu.vue";
import { askDelete, askEndSession, connect, create, edit, focusSession, saveActive } from "@/lib/actions";
import { address, displayName, formatLastUsed, kindLabels } from "@/lib/list";
import { connectedIds, groups, probes, state, unsaved, visible } from "@/lib/store";
import type { Connection } from "@/lib/types";

const sorts = [
  { value: "name" as const, label: "Name", icon: ArrowDownAZ },
  { value: "recent" as const, label: "Zuletzt", icon: Clock },
];

/** Green: open right now; grey: did not answer the last check. */
function status(c: Connection): { class: string; title: string } | null {
  if (connectedIds.value.has(c.id)) return { class: "bg-emerald-500", title: "Verbunden" };
  if (probes.get(c.id)?.reachable === false) return { class: "bg-zinc-400 dark:bg-zinc-500", title: "Nicht erreichbar" };
  return null;
}

const container = ref<HTMLElement>();
const menuConnection = ref<Connection>();

function onKeydown(event: KeyboardEvent) {
  const list = visible.value;
  const i = list.findIndex((c) => c.id === state.selected);
  const go = (index: number) => {
    event.preventDefault();
    const c = list[Math.max(0, Math.min(index, list.length - 1))];
    if (c) state.selected = c.id;
  };
  const current = list[i];
  switch (event.key) {
    case "ArrowDown":
      return go(i + 1);
    case "ArrowUp":
      return go(i <= 0 ? 0 : i - 1);
    case "Home":
      return go(0);
    case "End":
      return go(list.length - 1);
    case "Enter":
      if (current) connect(current);
      return;
    case "F2":
      if (current) edit(current);
      return;
    case "Delete":
      if (current) askDelete(current);
      return;
  }
}

watch(
  () => state.selected,
  async (id) => {
    await nextTick();
    container.value?.querySelector(`[data-id="${CSS.escape(id)}"]`)?.scrollIntoView({ block: "nearest" });
  },
);
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="flex h-10 shrink-0 items-center gap-2 border-b border-border px-3">
      <span class="text-xs text-muted-foreground">Sortieren</span>
      <Segmented v-model="state.sort" :options="sorts" />
    </div>

    <ContextMenuRoot>
      <ContextMenuTrigger as-child>
        <div
          id="connection-list"
          ref="container"
          class="min-h-0 flex-1 overflow-y-auto p-1.5 outline-none"
          tabindex="0"
          @keydown="onKeydown"
          @contextmenu.capture="menuConnection = undefined"
        >
          <section v-if="unsaved.length" class="mb-2 rounded-lg border border-emerald-500/25 bg-emerald-500/5 pb-1">
            <h3 class="flex items-center gap-2 px-2.5 pt-2 pb-1 text-xs font-semibold tracking-wide text-emerald-700 uppercase dark:text-emerald-400">
              <span class="size-2 rounded-full bg-emerald-500" />Aktiv – nicht gespeichert
              <span class="font-normal tabular-nums opacity-70">{{ unsaved.length }}</span>
            </h3>
            <div
              v-for="a in unsaved"
              :key="`${a.pid}-${a.address}-${a.port}`"
              class="mx-1 flex h-11 cursor-default items-center gap-3 rounded-md px-2.5 hover:bg-foreground/5"
              @dblclick="saveActive(a)"
            >
              <KindIcon :kind="a.kind" class="size-7" />
              <div class="min-w-0 flex-1">
                <p class="truncate text-[13px] font-medium">{{ a.user ? `${a.user}@${a.host}` : a.host }}</p>
                <p class="truncate text-[11px] text-muted-foreground">
                  {{ kindLabels[a.kind] }} · {{ a.process }} · <span class="font-mono">{{ a.address }}:{{ a.port }}</span>
                </p>
              </div>
              <Tip text="Fenster zeigen">
                <button class="icon-btn" aria-label="Fenster zeigen" @click="focusSession(a)"><AppWindow /></button>
              </Tip>
              <Tip text="Sitzung beenden">
                <button class="icon-btn hover:!text-red-500" aria-label="Sitzung beenden" @click="askEndSession(a)"><Unplug /></button>
              </Tip>
              <button class="btn btn-outline h-7 px-2.5 text-xs" @click="saveActive(a)"><Save />Speichern</button>
            </div>
          </section>

          <section v-for="group in groups" :key="group.favorites ? '*favorites' : group.name" class="mb-2">
            <h3 class="flex items-center gap-1.5 px-2.5 pt-2 pb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
              <Star v-if="group.favorites" class="size-3.5 fill-amber-400 text-amber-400" />
              {{ group.name || (groups.length > 1 ? "Ohne Gruppe" : "Verbindungen") }}
              <span class="ml-0.5 font-normal tabular-nums opacity-70">{{ group.items.length }}</span>
            </h3>
            <div
              v-for="c in group.items"
              :key="c.id"
              :data-id="c.id"
              class="flex h-11 cursor-default items-center gap-3 rounded-md border px-2.5"
              :class="c.id === state.selected ? 'border-accent/30 bg-accent/15' : 'border-transparent hover:bg-foreground/5'"
              @click="state.selected = c.id"
              @dblclick="connect(c)"
              @contextmenu="(state.selected = c.id), (menuConnection = c)"
            >
              <div class="relative" :title="status(c)?.title">
                <KindIcon :kind="c.kind" class="size-7" :class="probes.get(c.id)?.reachable === false && 'opacity-60'" />
                <span
                  v-if="status(c)"
                  class="absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full border-2 border-card"
                  :class="status(c)!.class"
                />
              </div>
              <div class="min-w-0 flex-1">
                <p class="truncate text-[13px] font-medium">{{ displayName(c) }}</p>
                <p class="truncate font-mono text-[11px] text-muted-foreground">{{ address(c) }}</p>
              </div>
              <span v-if="state.sort === 'recent' && c.lastUsed" class="shrink-0 text-[11px] text-muted-foreground">
                {{ formatLastUsed(c.lastUsed) }}
              </span>
            </div>
          </section>

          <div v-if="!visible.length && !unsaved.length" class="flex h-full flex-col items-center justify-center gap-3 text-muted-foreground">
            <template v-if="state.query">
              <SearchX class="size-8 opacity-60" />
              <p>Nichts gefunden für „{{ state.query.trim() }}“</p>
            </template>
            <template v-else>
              <MonitorSmartphone class="size-8 opacity-60" />
              <p>Noch keine Verbindungen gespeichert</p>
              <div class="flex gap-2">
                <button class="btn btn-primary" @click="create"><Plus />Erste Verbindung anlegen</button>
                <button class="btn btn-outline" @click="state.importing = true">Von diesem PC importieren …</button>
              </div>
            </template>
          </div>
        </div>
      </ContextMenuTrigger>
      <RowMenu :connection="menuConnection" />
    </ContextMenuRoot>
  </div>
</template>
