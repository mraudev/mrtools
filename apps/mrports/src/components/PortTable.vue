<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { ContextMenuRoot, ContextMenuTrigger } from "reka-ui";
import { ArrowDown, ArrowUp, CircleCheck, SearchX } from "@lucide/vue";
import ProcIcon from "./ProcIcon.vue";
import RowMenu from "./RowMenu.vue";
import { askKill, openInBrowser } from "@/lib/actions";
import { portIsFree, type Row, type SortKey } from "@/lib/rows";
import { rows, snapshot, sortBy, state } from "@/lib/store";

const container = ref<HTMLElement>();
const menuRow = ref<Row>();

const listening = computed(() => state.mode === "listening");
const columns = computed(() =>
  listening.value
    ? "grid-cols-[84px_64px_132px_140px_minmax(160px,1fr)_72px_minmax(160px,1.4fr)]"
    : "grid-cols-[84px_64px_minmax(170px,1fr)_110px_minmax(160px,1fr)_72px_minmax(160px,1.2fr)]",
);
const headers = computed<{ label: string; key?: SortKey; class?: string }[]>(() => [
  { label: "Port", key: "port", class: "justify-end pr-4" },
  { label: "Protokoll", key: "protocol" },
  listening.value ? { label: "Erreichbar" } : { label: "Gegenstelle" },
  listening.value ? { label: "Dienst" } : { label: "Status" },
  { label: "Prozess", key: "process", class: "pl-8" },
  { label: "PID", key: "pid", class: "justify-end pr-3" },
  { label: "Ordner" },
]);

const free = computed(() => portIsFree(snapshot.value.sockets, state.query));

function onKeydown(event: KeyboardEvent) {
  const list = rows.value;
  const i = list.findIndex((r) => r.key === state.selected);
  const go = (index: number) => {
    event.preventDefault();
    const row = list[Math.max(0, Math.min(index, list.length - 1))];
    if (row) state.selected = row.key;
  };
  const row = list[i];
  switch (event.key) {
    case "ArrowDown":
      return go(i + 1);
    case "ArrowUp":
      return go(i <= 0 ? 0 : i - 1);
    case "Home":
      return go(0);
    case "End":
      return go(list.length - 1);
    case "Delete":
      if (row) askKill(row);
      return;
    case "Enter":
      if (row?.protocol === "TCP" && row.state === "Lauscht") openInBrowser(row.port);
      return;
  }
}

watch(
  () => state.selected,
  async (key) => {
    await nextTick();
    container.value?.querySelector(`[data-key="${CSS.escape(key)}"]`)?.scrollIntoView({ block: "nearest" });
  },
);
</script>

<template>
  <div class="flex h-full min-w-0 flex-col">
    <div
      v-if="free"
      class="flex shrink-0 items-center gap-2 border-b border-border bg-emerald-500/10 px-4 py-2 text-[13px] text-emerald-700 dark:text-emerald-400"
    >
      <CircleCheck class="size-4" />
      <span>Port <b class="tabular-nums">{{ state.query.trim() }}</b> ist frei – kein Programm benutzt ihn.</span>
    </div>

    <div class="grid shrink-0 border-b border-border px-1.5 text-xs font-medium text-muted-foreground" :class="columns">
      <component
        :is="header.key ? 'button' : 'div'"
        v-for="header in headers"
        :key="header.label"
        class="flex h-7 items-center gap-1 px-2"
        :class="[header.class, header.key && 'transition-colors hover:text-foreground', state.sort.key === header.key && 'text-foreground']"
        @click="header.key && sortBy(header.key)"
      >
        {{ header.label }}
        <template v-if="header.key && state.sort.key === header.key">
          <ArrowDown v-if="state.sort.desc" class="size-3" />
          <ArrowUp v-else class="size-3" />
        </template>
      </component>
    </div>

    <ContextMenuRoot>
      <ContextMenuTrigger as-child>
        <div
          id="port-table"
          ref="container"
          class="group/list min-h-0 flex-1 overflow-y-auto p-1.5 outline-none"
          tabindex="0"
          @keydown="onKeydown"
          @contextmenu.capture="menuRow = undefined"
        >
          <div
            v-for="row in rows"
            :key="row.key"
            :data-key="row.key"
            class="grid h-[30px] cursor-default items-center rounded-md border text-[13px]"
            :class="[columns, row.key === state.selected ? 'border-accent/30 bg-accent/15' : 'border-transparent hover:bg-foreground/5']"
            @click="state.selected = row.key"
            @dblclick="row.protocol === 'TCP' && row.state === 'Lauscht' && openInBrowser(row.port)"
            @contextmenu="(state.selected = row.key), (menuRow = row)"
          >
            <div class="pr-4 text-right font-mono font-semibold tabular-nums">{{ row.port }}</div>
            <div class="px-2 text-xs text-muted-foreground">
              {{ row.protocol }}<span v-if="row.addresses.some((a) => a.includes(':'))" class="ml-1 opacity-70">v6</span>
            </div>
            <template v-if="listening">
              <div class="truncate px-2 text-xs" :class="row.reach === 'Alle Netzwerke' ? 'text-amber-600 dark:text-amber-400' : 'text-muted-foreground'" :title="row.addresses.join(', ')">
                {{ row.reach }}
              </div>
              <div class="truncate px-2 text-xs text-muted-foreground">{{ row.service }}</div>
            </template>
            <template v-else>
              <div class="truncate px-2 font-mono text-xs text-muted-foreground" :title="row.remote">{{ row.remote || "–" }}</div>
              <div class="truncate px-2 text-xs text-muted-foreground">{{ row.state }}</div>
            </template>
            <div class="flex min-w-0 items-center gap-2 px-2" :title="row.process.exe">
              <ProcIcon :process="row.process" class="size-4" />
              <span class="truncate">{{ row.process.name }}</span>
            </div>
            <div class="pr-3 text-right text-xs text-muted-foreground tabular-nums">{{ row.pid }}</div>
            <div class="truncate px-2 font-mono text-[11px] text-muted-foreground" :title="row.process.cwd">{{ row.process.cwd }}</div>
          </div>

          <div v-if="!rows.length && !free" class="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
            <SearchX class="size-8 opacity-60" />
            <p>{{ state.query ? `Nichts gefunden für „${state.query.trim()}“` : "Keine Ports" }}</p>
          </div>
        </div>
      </ContextMenuTrigger>
      <RowMenu :row="menuRow" />
    </ContextMenuRoot>
  </div>
</template>
