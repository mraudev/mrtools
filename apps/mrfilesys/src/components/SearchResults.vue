<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import {
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuPortal,
  ContextMenuRoot,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "reka-ui";
import {
  ClipboardCopy,
  Copy,
  DatabaseZap,
  ExternalLink,
  FolderOpen,
  FolderSearch,
  Info,
  LoaderCircle,
  RefreshCw,
  SearchX,
} from "@lucide/vue";
import EntryIcon from "./EntryIcon.vue";
import Tip from "@mrtools/ui/components/Tip";
import { copyPaths, copyToClipboard, openEntry, openProperties, showInExplorer } from "@/lib/actions";
import { formatBytes, formatCount, formatDate } from "@/lib/format";
import { parentPath } from "@/lib/paths";
import { LIMIT, rebuildIndex, search } from "@/lib/search";
import { navigate, state } from "@/lib/store";
import type { SearchHit } from "@/lib/types";

const COLUMNS = "grid-cols-[minmax(200px,2fr)_minmax(160px,3fr)_140px_90px]";

const list = ref<HTMLElement>();
const cursor = ref(0);
const menuHit = ref<SearchHit>();
const hit = computed(() => search.hits[cursor.value]);

watch(
  () => search.hits,
  () => (cursor.value = 0),
);

/** The name split around the first plain search term, for highlighting. */
function parts(name: string): [string, string, string] {
  const term = state.filter
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .find((t) => t && !/[*?\\]/.test(t));
  const i = term ? name.toLowerCase().indexOf(term) : -1;
  if (!term || i < 0) return [name, "", ""];
  return [name.slice(0, i), name.slice(i, i + term.length), name.slice(i + term.length)];
}

function openFolder(h: SearchHit) {
  navigate(parentPath(h.path), { select: h.path });
}

async function move(index: number) {
  cursor.value = Math.max(0, Math.min(index, search.hits.length - 1));
  await nextTick();
  list.value?.querySelector(`[data-index="${cursor.value}"]`)?.scrollIntoView({ block: "nearest" });
}

function onKeydown(event: KeyboardEvent) {
  const page = 15;
  const keys: Record<string, () => unknown> = {
    ArrowDown: () => move(cursor.value + 1),
    ArrowUp: () => (cursor.value === 0 ? document.getElementById("search")?.focus() : move(cursor.value - 1)),
    PageDown: () => move(cursor.value + page),
    PageUp: () => move(cursor.value - page),
    Home: () => move(0),
    End: () => move(search.hits.length - 1),
    Enter: () => hit.value && (event.ctrlKey ? openFolder(hit.value) : openEntry(hit.value)),
  };
  if (event.altKey) return;
  const action = keys[event.key];
  if (action) {
    event.preventDefault();
    action();
  }
}

function since(ms: number) {
  if (!ms) return "";
  const minutes = Math.round((Date.now() - ms) / 60_000);
  if (minutes < 1) return "gerade eben";
  if (minutes < 60) return `vor ${minutes} min`;
  return `von ${formatDate(ms)}`;
}
</script>

<template>
  <div class="flex h-full min-w-0 flex-col">
    <div class="flex h-10 shrink-0 items-center gap-3 border-b border-border px-3 text-xs text-muted-foreground">
      <span v-if="search.loading" class="inline-flex items-center gap-1.5">
        <LoaderCircle class="size-3.5 animate-spin" />Suche …
      </span>
      <span v-else-if="search.status.entries" class="tabular-nums">
        <span class="font-medium text-foreground">{{ formatCount(search.total) }} Treffer</span>
        <template v-if="search.total > LIMIT"> · die ersten {{ formatCount(LIMIT) }}</template>
        · {{ search.elapsedMs }} ms
      </span>
      <div class="flex-1" />
      <span v-if="search.status.building" class="inline-flex items-center gap-1.5 tabular-nums">
        <LoaderCircle class="size-3.5 animate-spin text-accent-text" />
        Index wird erneuert … {{ formatCount(search.status.scanned) }} gelesen
      </span>
      <span v-else-if="search.status.entries" class="tabular-nums">
        Index: {{ formatCount(search.status.entries) }} Einträge, {{ since(search.status.builtAt) }}
      </span>
      <Tip text="Index jetzt erneuern – neue Dateien werden erst danach gefunden" side="bottom">
        <button class="icon-btn size-6 [&_svg]:size-3.5" aria-label="Index erneuern" :disabled="search.status.building" @click="rebuildIndex">
          <RefreshCw />
        </button>
      </Tip>
    </div>

    <div v-if="!search.status.entries" class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center text-muted-foreground">
      <DatabaseZap class="size-8 opacity-60" />
      <p class="font-medium text-foreground">Der Suchindex wird aufgebaut</p>
      <p class="max-w-sm text-xs">
        mrfilesys liest einmal alle Dateinamen der lokalen Laufwerke ein – danach findet die Suche alles sofort.
        <template v-if="search.status.scanned"><br />{{ formatCount(search.status.scanned) }} Einträge gelesen …</template>
      </p>
    </div>

    <template v-else>
      <div class="grid shrink-0 border-b border-border px-1.5 text-xs font-medium text-muted-foreground [&>div]:px-2 [&>div]:py-1.5" :class="COLUMNS">
        <div class="pl-8">Name</div>
        <div>Ordner</div>
        <div>Geändert</div>
        <div class="text-right">Größe</div>
      </div>

      <ContextMenuRoot>
        <ContextMenuTrigger as-child>
          <div
            id="search-results"
            ref="list"
            class="group/list min-h-0 flex-1 overflow-y-auto p-1.5 outline-none"
            tabindex="0"
            @keydown="onKeydown"
          >
            <div
              v-for="(h, i) in search.hits"
              :key="h.path"
              :data-index="i"
              class="grid h-7 cursor-default items-center rounded-md border text-[13px]"
              :class="[
                COLUMNS,
                i === cursor ? 'border-accent/30 bg-accent/15' : 'border-transparent hover:bg-foreground/5',
                !h.modified && 'opacity-50',
              ]"
              :title="h.modified ? h.path : `${h.path}\n(nicht mehr vorhanden)`"
              @click="cursor = i"
              @dblclick="openEntry(h)"
              @contextmenu="(cursor = i), (menuHit = h)"
            >
              <div class="flex min-w-0 items-center gap-2 px-2">
                <EntryIcon :entry="{ name: h.name, isDir: h.isDir, link: false }" class="size-4" />
                <span class="truncate">
                  {{ parts(h.name)[0] }}<mark class="rounded-sm bg-accent/30 text-inherit">{{ parts(h.name)[1] }}</mark>{{ parts(h.name)[2] }}
                </span>
              </div>
              <button
                class="truncate px-2 text-left text-xs text-muted-foreground hover:text-accent-text hover:underline"
                @click.stop="openFolder(h)"
              >
                {{ parentPath(h.path) }}
              </button>
              <div class="truncate px-2 text-xs text-muted-foreground tabular-nums">{{ formatDate(h.modified) }}</div>
              <div class="truncate px-2 text-right text-xs text-muted-foreground tabular-nums">
                {{ h.isDir || !h.modified ? "" : formatBytes(h.size) }}
              </div>
            </div>

            <div
              v-if="!search.hits.length && !search.loading"
              class="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground"
            >
              <SearchX class="size-8 opacity-60" />
              <p>Nichts gefunden für „{{ state.filter.trim() }}“</p>
              <p class="text-xs">Tipp: <span class="kbd">*.pdf</span> für Endungen, <span class="kbd">projekte\ bericht</span> für Teile des Pfads</p>
            </div>
          </div>
        </ContextMenuTrigger>
        <ContextMenuPortal>
          <ContextMenuContent v-if="menuHit" class="menu anim-pop">
            <ContextMenuItem class="menu-item font-medium" @select="openEntry(menuHit)">
              <FolderOpen v-if="menuHit.isDir" /><ExternalLink v-else />Öffnen<span class="kbd ml-auto font-normal">Enter</span>
            </ContextMenuItem>
            <ContextMenuItem class="menu-item" @select="openFolder(menuHit)">
              <FolderOpen />Ordner öffnen<span class="kbd ml-auto">Strg+Enter</span>
            </ContextMenuItem>
            <ContextMenuItem class="menu-item" @select="showInExplorer(menuHit.path)"><FolderSearch />Im Explorer zeigen</ContextMenuItem>
            <ContextMenuSeparator class="menu-separator" />
            <ContextMenuItem class="menu-item" @select="copyToClipboard([menuHit.path], false)"><Copy />Kopieren</ContextMenuItem>
            <ContextMenuItem class="menu-item" @select="copyPaths([menuHit.path])"><ClipboardCopy />Pfad kopieren</ContextMenuItem>
            <ContextMenuSeparator class="menu-separator" />
            <ContextMenuItem class="menu-item" @select="openProperties([menuHit.path])"><Info />Eigenschaften</ContextMenuItem>
          </ContextMenuContent>
        </ContextMenuPortal>
      </ContextMenuRoot>
    </template>
  </div>
</template>
