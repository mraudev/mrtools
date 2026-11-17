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
  Scissors,
  SearchX,
  Trash,
  X,
} from "@lucide/vue";
import EntryIcon from "./EntryIcon.vue";
import Tip from "@mrtools/ui/components/Tip";
import { copyPaths, copyToClipboard, deletePaths, openEntry, openProperties, showInExplorer } from "@/lib/actions";
import { formatBytes, formatCount, formatDate } from "@/lib/format";
import { baseName, parentPath } from "@/lib/paths";
import { LIMIT, rebuildIndex, refreshSearch, search } from "@/lib/search";
import { navigate, state } from "@/lib/store";
import type { SearchHit } from "@/lib/types";

const COLUMNS = "grid-cols-[minmax(200px,2fr)_minmax(160px,3fr)_140px_90px]";

const list = ref<HTMLElement>();
const selected = ref(new Set<string>());
/** Keyboard cursor (a path) and start of Shift ranges. */
const focused = ref("");
let anchor = "";

const cursor = computed(() => Math.max(0, search.hits.findIndex((h) => h.path === focused.value)));
const hit = computed(() => search.hits[cursor.value]);
const chosen = computed(() => search.hits.filter((h) => selected.value.has(h.path)));
const paths = computed(() => chosen.value.map((h) => h.path));
const single = computed(() => (chosen.value.length === 1 ? chosen.value[0] : undefined));

// New results keep what is still there – after deleting, the rest stays selected.
watch(
  () => search.hits,
  (hits) => {
    const present = new Set(hits.map((h) => h.path));
    selected.value = new Set([...selected.value].filter((p) => present.has(p)));
    if (!present.has(focused.value)) focused.value = anchor = hits[0]?.path ?? "";
  },
);

function selectRange(to: string) {
  const a = Math.max(0, search.hits.findIndex((h) => h.path === anchor));
  const b = search.hits.findIndex((h) => h.path === to);
  const [from, end] = a <= b ? [a, b] : [b, a];
  selected.value = new Set(search.hits.slice(from, end + 1).map((h) => h.path));
}

function onClick(event: MouseEvent, h: SearchHit) {
  if (event.shiftKey) selectRange(h.path);
  else if (event.ctrlKey) {
    const next = new Set(selected.value);
    if (!next.delete(h.path)) next.add(h.path);
    selected.value = next;
  } else selected.value = new Set([h.path]);
  focused.value = h.path;
  if (!event.shiftKey) anchor = h.path;
}

function onContextmenu(h: SearchHit) {
  if (!selected.value.has(h.path)) selected.value = new Set([h.path]);
  focused.value = anchor = h.path;
}

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

/** Moves the cursor; with Shift the selection grows from the anchor. */
async function move(index: number, extend: boolean) {
  const i = Math.max(0, Math.min(index, search.hits.length - 1));
  const path = search.hits[i]?.path;
  if (!path) return;
  focused.value = path;
  if (extend) selectRange(path);
  else {
    selected.value = new Set([path]);
    anchor = path;
  }
  await nextTick();
  list.value?.querySelector(`[data-index="${i}"]`)?.scrollIntoView({ block: "nearest" });
}

function onKeydown(event: KeyboardEvent) {
  const page = 15;
  const ctrl = event.ctrlKey && !event.altKey;
  const key = event.key.toLowerCase();
  const shift = event.shiftKey;
  const keys: Record<string, () => unknown> = {
    ArrowDown: () => move(cursor.value + 1, shift),
    ArrowUp: () =>
      cursor.value === 0 && !shift ? document.getElementById("search")?.focus() : move(cursor.value - 1, shift),
    PageDown: () => move(cursor.value + page, shift),
    PageUp: () => move(cursor.value - page, shift),
    Home: () => move(0, shift),
    End: () => move(search.hits.length - 1, shift),
    Enter: () => hit.value && (ctrl ? openFolder(hit.value) : openEntry(hit.value)),
    Delete: () => deletePaths(paths.value, shift),
  };
  let action = event.altKey ? undefined : keys[event.key];
  if (ctrl && key === "a") action = () => (selected.value = new Set(search.hits.map((h) => h.path)));
  if (ctrl && key === "c") action = () => (shift ? copyPaths(paths.value) : copyToClipboard(paths.value, false));
  if (ctrl && key === "x") action = () => copyToClipboard(paths.value, true);
  if (event.altKey && event.key === "Enter") action = () => paths.value.length && openProperties(paths.value);
  if (action) {
    // Not on to the window – its keys act on the (hidden) folder.
    event.preventDefault();
    event.stopPropagation();
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
        <LoaderCircle class="size-3.5 animate-spin" />{{ search.reading ? "Ordner wird eingelesen …" : "Suche …" }}
      </span>
      <span v-else-if="!search.everywhere || search.status.entries" class="min-w-0 truncate tabular-nums">
        <span class="font-medium text-foreground">{{ formatCount(search.total) }} Treffer</span>
        {{ search.everywhere ? "auf allen Laufwerken" : `in ${baseName(state.path)} und Unterordnern` }}
        <template v-if="search.total > LIMIT"> · die ersten {{ formatCount(LIMIT) }}</template>
        · {{ search.elapsedMs }} ms
        <template v-if="selected.size"> · {{ formatCount(selected.size) }} ausgewählt</template>
        <span v-if="search.refreshing" class="ml-1 inline-flex items-center gap-1" title="Die Treffer stammen vom letzten Einlesen und werden gleich aktualisiert">
          · <LoaderCircle class="size-3 animate-spin" />wird aktualisiert</span
        >
      </span>
      <div class="flex-1" />
      <Tip v-if="!search.everywhere" text="Neu suchen (F5) – Ordner außerhalb des Index (USB, Netz) werden dabei neu eingelesen" side="bottom">
        <button class="icon-btn size-6 [&_svg]:size-3.5" aria-label="Neu suchen" :disabled="search.loading" @click="refreshSearch">
          <RefreshCw />
        </button>
      </Tip>
      <span v-else-if="search.status.building" class="inline-flex items-center gap-1.5 tabular-nums">
        <LoaderCircle class="size-3.5 animate-spin text-accent-text" />
        Index wird erneuert … {{ formatCount(search.status.scanned) }} gelesen
      </span>
      <span v-else-if="search.status.entries" class="tabular-nums">
        Index: {{ formatCount(search.status.entries) }} Einträge,
        {{ search.status.watching ? "laufend aktuell" : since(search.status.builtAt) }}
      </span>
      <Tip v-if="search.everywhere" text="Index komplett neu einlesen" side="bottom">
        <button class="icon-btn size-6 [&_svg]:size-3.5" aria-label="Index erneuern" :disabled="search.status.building" @click="rebuildIndex">
          <RefreshCw />
        </button>
      </Tip>
    </div>

    <div v-if="search.everywhere && !search.status.entries" class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center text-muted-foreground">
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
                selected.has(h.path) ? 'border-accent/30 bg-accent/15' : 'border-transparent hover:bg-foreground/5',
                h.path === focused && 'group-focus/list:border-accent/60',
                state.cut.has(h.path) && 'opacity-50',
              ]"
              :title="h.path"
              @click="onClick($event, h)"
              @dblclick="openEntry(h)"
              @contextmenu="onContextmenu(h)"
            >
              <div class="flex min-w-0 items-center gap-2 px-2">
                <EntryIcon :entry="{ name: h.name, isDir: h.isDir, link: false, path: h.path }" class="size-4" />
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
                {{ h.isDir ? "" : formatBytes(h.size) }}
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
          <ContextMenuContent v-if="paths.length" class="menu anim-pop">
            <template v-if="single">
              <ContextMenuItem class="menu-item font-medium" @select="openEntry(single)">
                <FolderOpen v-if="single.isDir" /><ExternalLink v-else />Öffnen<span class="kbd ml-auto font-normal">Enter</span>
              </ContextMenuItem>
              <ContextMenuItem class="menu-item" @select="openFolder(single)">
                <FolderOpen />Ordner öffnen<span class="kbd ml-auto">Strg+Enter</span>
              </ContextMenuItem>
              <ContextMenuItem class="menu-item" @select="showInExplorer(single.path)"><FolderSearch />Im Explorer zeigen</ContextMenuItem>
              <ContextMenuSeparator class="menu-separator" />
            </template>
            <ContextMenuItem class="menu-item" @select="copyToClipboard(paths, true)">
              <Scissors />Ausschneiden<span class="kbd ml-auto">Strg+X</span>
            </ContextMenuItem>
            <ContextMenuItem class="menu-item" @select="copyToClipboard(paths, false)">
              <Copy />Kopieren<span class="kbd ml-auto">Strg+C</span>
            </ContextMenuItem>
            <ContextMenuItem class="menu-item" @select="copyPaths(paths)">
              <ClipboardCopy />{{ paths.length > 1 ? "Pfade kopieren" : "Pfad kopieren" }}
              <span class="kbd ml-auto">Strg+Umschalt+C</span>
            </ContextMenuItem>
            <ContextMenuSeparator class="menu-separator" />
            <ContextMenuItem class="menu-item" @select="deletePaths(paths)">
              <Trash />In den Papierkorb<span class="kbd ml-auto">Entf</span>
            </ContextMenuItem>
            <ContextMenuItem class="menu-item text-red-500 [&_svg]:!text-red-500" @select="deletePaths(paths, true)">
              <X />Endgültig löschen …<span class="kbd ml-auto">Umschalt+Entf</span>
            </ContextMenuItem>
            <ContextMenuSeparator class="menu-separator" />
            <ContextMenuItem class="menu-item" @select="openProperties(paths)">
              <Info />Eigenschaften<span class="kbd ml-auto">Alt+Enter</span>
            </ContextMenuItem>
          </ContextMenuContent>
        </ContextMenuPortal>
      </ContextMenuRoot>
    </template>
  </div>
</template>
