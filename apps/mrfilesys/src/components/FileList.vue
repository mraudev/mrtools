<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { ContextMenuRoot, ContextMenuTrigger } from "reka-ui";
import { ArrowDown, ArrowUp, FolderOpen, LoaderCircle, SearchX } from "@lucide/vue";
import EntryIcon from "./EntryIcon.vue";
import EntryMenu from "./EntryMenu.vue";
import RenameInput from "./RenameInput.vue";
import { openEntry, openSelection } from "@/lib/actions";
import { beginDrag, drag } from "@/lib/dnd";
import { typeLabel } from "@/lib/fileTypes";
import { formatBytes, formatDate } from "@/lib/format";
import { selectOnly, selectRange, settings, sortBy, state, toggleSelected, visible } from "@/lib/store";
import type { Entry, SortKey } from "@/lib/types";

const ROW_HEIGHT = 28;
const TILE_WIDTH = 116;
const TILE_HEIGHT = 112;
const PADDING = 6;
const COLUMNS = "grid-cols-[minmax(200px,1fr)_150px_140px_96px]";

const scroller = ref<HTMLElement>();
const scrollTop = ref(0);
const viewWidth = ref(0);
const viewHeight = ref(0);
/** Start of Shift ranges. */
const anchor = ref("");
const menuOnEntry = ref(false);

const grid = computed(() => settings.view === "grid");
const perLine = computed(() => (grid.value ? Math.max(1, Math.floor((viewWidth.value - 2 * PADDING) / TILE_WIDTH)) : 1));
const lineHeight = computed(() => (grid.value ? TILE_HEIGHT : ROW_HEIGHT));
const lineCount = computed(() => Math.ceil(visible.value.length / perLine.value));

// Only the rows in view (plus a few) are rendered – folders can have tens of thousands of entries.
const firstLine = computed(() => Math.max(0, Math.floor((scrollTop.value - PADDING) / lineHeight.value) - 4));
const lastLine = computed(() =>
  Math.min(lineCount.value, Math.ceil((scrollTop.value + viewHeight.value) / lineHeight.value) + 4),
);
const slice = computed(() => {
  const start = firstLine.value * perLine.value;
  return visible.value.slice(start, lastLine.value * perLine.value).map((entry, i) => ({ entry, index: start + i }));
});

let resizeObserver: ResizeObserver | undefined;
onMounted(() => {
  resizeObserver = new ResizeObserver(([e]) => {
    viewWidth.value = e.contentRect.width;
    viewHeight.value = e.contentRect.height;
  });
  resizeObserver.observe(scroller.value!);
});
onUnmounted(() => resizeObserver?.disconnect());

function ensureVisible(index: number) {
  const el = scroller.value;
  if (!el || index < 0) return;
  const top = Math.floor(index / perLine.value) * lineHeight.value + PADDING;
  if (top < el.scrollTop) el.scrollTop = top - PADDING;
  else if (top + lineHeight.value > el.scrollTop + el.clientHeight) el.scrollTop = top + lineHeight.value - el.clientHeight + PADDING;
}

watch(
  () => [state.path, state.focused] as const,
  async ([path], [oldPath]) => {
    await nextTick();
    if (path !== oldPath) {
      if (scroller.value) scroller.value.scrollTop = 0;
      anchor.value = state.focused;
    }
    ensureVisible(visible.value.findIndex((e) => e.path === state.focused));
  },
);

// ---------------------------------------------------------------------------
// Mouse

function onItemClick(event: MouseEvent, entry: Entry) {
  if (event.ctrlKey) toggleSelected(entry.path);
  else if (event.shiftKey) {
    selectRange(entry.path, anchor.value);
    state.focused = entry.path;
    return;
  } else selectOnly(entry.path);
  anchor.value = entry.path;
}

function onBackgroundMousedown(event: MouseEvent) {
  if (event.button === 0 && !(event.target as HTMLElement).closest("[data-path]") && !event.ctrlKey) selectOnly("");
}

function onContextmenu(event: MouseEvent) {
  const path = (event.target as HTMLElement).closest<HTMLElement>("[data-path]")?.dataset.path;
  menuOnEntry.value = !!path;
  if (!path) selectOnly("");
  else if (!state.selected.has(path)) selectOnly(path);
}

// ---------------------------------------------------------------------------
// Keyboard

let typed = "";
let typedAt = 0;

function moveTo(index: number, event: KeyboardEvent) {
  event.preventDefault();
  const list = visible.value;
  const entry = list[Math.max(0, Math.min(index, list.length - 1))];
  if (!entry) return;
  if (event.shiftKey) {
    selectRange(entry.path, anchor.value);
    state.focused = entry.path;
    return;
  }
  if (event.ctrlKey) state.focused = entry.path;
  else selectOnly(entry.path);
  anchor.value = entry.path;
}

function onKeydown(event: KeyboardEvent) {
  const list = visible.value;
  const i = list.findIndex((e) => e.path === state.focused);
  const page = Math.max(1, Math.floor(viewHeight.value / lineHeight.value) - 1) * perLine.value;
  switch (event.key) {
    case "ArrowDown":
      return moveTo(i < 0 ? 0 : i + perLine.value, event);
    case "ArrowUp":
      return moveTo(i < 0 ? 0 : i - perLine.value, event);
    case "ArrowRight":
      if (grid.value && !event.altKey) moveTo(i + 1, event);
      return;
    case "ArrowLeft":
      if (grid.value && !event.altKey) moveTo(Math.max(0, i - 1), event);
      return;
    case "PageDown":
      return moveTo(i + page, event);
    case "PageUp":
      return moveTo(Math.max(0, i - page), event);
    case "Home":
      return moveTo(0, event);
    case "End":
      return moveTo(list.length - 1, event);
    case "Enter":
      if (event.altKey) return;
      event.preventDefault();
      return openSelection();
    case " ":
      if (event.ctrlKey && state.focused) {
        event.preventDefault();
        toggleSelected(state.focused);
      }
      return;
  }
  // Type-ahead: jump to the first entry starting with the typed letters.
  if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
    const now = Date.now();
    typed = now - typedAt > 800 ? event.key : typed + event.key;
    typedAt = now;
    const prefix = typed.toLowerCase();
    const hit = list.find((e) => e.name.toLowerCase().startsWith(prefix));
    if (hit) {
      selectOnly(hit.path);
      anchor.value = hit.path;
    }
  }
}

const sortColumns: { key: SortKey; label: string; class: string }[] = [
  { key: "name", label: "Name", class: "pl-8" },
  { key: "modified", label: "Geändert", class: "" },
  { key: "type", label: "Typ", class: "" },
  { key: "size", label: "Größe", class: "justify-end pr-3" },
];
</script>

<template>
  <div class="flex h-full min-w-0 flex-col">
    <div v-if="!grid" class="grid shrink-0 border-b border-border px-1.5 text-xs font-medium text-muted-foreground" :class="COLUMNS">
      <button
        v-for="column in sortColumns"
        :key="column.key"
        class="flex h-7 items-center gap-1 px-2 transition-colors hover:text-foreground"
        :class="[column.class, settings.sortKey === column.key && 'text-foreground']"
        @click="sortBy(column.key)"
      >
        {{ column.label }}
        <template v-if="settings.sortKey === column.key">
          <ArrowUp v-if="settings.ascending" class="size-3" />
          <ArrowDown v-else class="size-3" />
        </template>
      </button>
    </div>

    <ContextMenuRoot>
      <ContextMenuTrigger as-child>
        <div
          id="file-list"
          ref="scroller"
          class="group/list relative min-h-0 flex-1 overflow-y-auto outline-none"
          tabindex="0"
          @scroll="scrollTop = ($event.target as HTMLElement).scrollTop"
          @keydown="onKeydown"
          @mousedown="onBackgroundMousedown"
          :data-drop="state.path"
          :class="drag.op && drag.target === state.path && 'ring-2 ring-accent/60 ring-inset'"
          @contextmenu="onContextmenu"
        >
          <div :style="{ height: `${lineCount * lineHeight + 2 * PADDING}px` }">
            <div class="px-1.5" :style="{ transform: `translateY(${firstLine * lineHeight + PADDING}px)` }">
              <div
                v-if="grid"
                class="grid"
                :style="{ gridTemplateColumns: `repeat(${perLine}, ${TILE_WIDTH}px)` }"
              >
                <div
                  v-for="{ entry } in slice"
                  :key="entry.path"
                  :data-path="entry.path"
                  :data-drop="entry.isDir ? entry.path : undefined"
                  class="m-0.5 flex cursor-default flex-col items-center gap-1 rounded-md border px-1.5 pt-2.5 pb-1.5 text-center"
                  :style="{ height: `${TILE_HEIGHT - 4}px` }"
                  :class="[
                    state.selected.has(entry.path) ? 'border-accent/30 bg-accent/15' : 'border-transparent hover:bg-foreground/5',
                    entry.path === state.focused && 'group-focus/list:border-accent/60',
                    (state.cut.has(entry.path) || entry.hidden) && 'opacity-50',
                    drag.op && drag.target === entry.path && '!border-accent !bg-accent/25',
                    drag.active && !drag.external && drag.paths.includes(entry.path) && 'opacity-50',
                  ]"
                  @pointerdown="beginDrag($event, entry.path)"
                  :title="entry.name"
                  @click="onItemClick($event, entry)"
                  @dblclick="openEntry(entry)"
                >
                  <EntryIcon :entry="entry" class="size-10" :stroke-width="1.5" />
                  <RenameInput v-if="state.renaming === entry.path" :path="entry.path" :name="entry.name" class="w-full text-center" />
                  <span v-else class="line-clamp-2 w-full text-xs leading-4 break-words">{{ entry.name }}</span>
                </div>
              </div>

              <template v-else>
                <div
                  v-for="{ entry } in slice"
                  :key="entry.path"
                  :data-path="entry.path"
                  :data-drop="entry.isDir ? entry.path : undefined"
                  class="grid cursor-default items-center rounded-md border text-[13px]"
                  :style="{ height: `${ROW_HEIGHT}px` }"
                  :class="[
                    COLUMNS,
                    state.selected.has(entry.path) ? 'border-accent/30 bg-accent/15' : 'border-transparent hover:bg-foreground/5',
                    entry.path === state.focused && 'group-focus/list:border-accent/60',
                    (state.cut.has(entry.path) || entry.hidden) && 'opacity-50',
                    drag.op && drag.target === entry.path && '!border-accent !bg-accent/25',
                    drag.active && !drag.external && drag.paths.includes(entry.path) && 'opacity-50',
                  ]"
                  @pointerdown="beginDrag($event, entry.path)"
                  @click="onItemClick($event, entry)"
                  @dblclick="openEntry(entry)"
                >
                  <div class="flex min-w-0 items-center gap-2 px-2">
                    <EntryIcon :entry="entry" class="size-4" />
                    <RenameInput v-if="state.renaming === entry.path" :path="entry.path" :name="entry.name" class="flex-1" />
                    <span v-else class="truncate">{{ entry.name }}</span>
                  </div>
                  <div class="truncate px-2 text-xs text-muted-foreground tabular-nums">{{ formatDate(entry.modified) }}</div>
                  <div class="truncate px-2 text-xs text-muted-foreground">{{ typeLabel(entry) }}</div>
                  <div class="truncate pr-3 text-right text-xs text-muted-foreground tabular-nums">
                    {{ entry.isDir ? "" : formatBytes(entry.size) }}
                  </div>
                </div>
              </template>
            </div>
          </div>

          <div
            v-if="!visible.length"
            class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center gap-2 text-muted-foreground"
          >
            <template v-if="state.loading">
              <LoaderCircle class="size-6 animate-spin" />
            </template>
            <template v-else-if="state.filter">
              <SearchX class="size-8 opacity-60" />
              <p>Keine Treffer für „{{ state.filter }}“</p>
            </template>
            <template v-else>
              <FolderOpen class="size-8 opacity-60" />
              <p>Dieser Ordner ist leer</p>
              <p v-if="!settings.showHidden && state.entries.length" class="text-xs">
                {{ state.entries.length }} versteckte Elemente – Strg+H zeigt sie an
              </p>
            </template>
          </div>
        </div>
      </ContextMenuTrigger>
      <EntryMenu :on-entry="menuOnEntry" />
    </ContextMenuRoot>
  </div>
</template>
