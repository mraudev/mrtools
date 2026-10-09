<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { ChevronRight, File, Folder, FolderOpen, TriangleAlert } from "@lucide/vue";
import EntryMenu from "./EntryMenu.vue";
import { formatBytes, formatCount, formatDate, formatPercent } from "@/lib/format";
import { childKey, entryOf, parentKey, pathOf, rows, select, state, toggle, type Row } from "@/lib/store";

const COLUMNS = "grid-cols-[minmax(180px,1fr)_88px_150px_80px_70px_124px]";

const container = ref<HTMLElement>();
const menuKey = ref("");
const menuPath = computed(() => pathOf(menuKey.value));
const menuIsDir = computed(() => entryOf(menuKey.value)?.isDir ?? false);

const entryRows = computed(() => rows.value.filter((r): r is Row => "entry" in r));

function onKeydown(event: KeyboardEvent) {
  const list = entryRows.value;
  const i = list.findIndex((r) => r.key === state.selected);
  const row = list[i];
  const go = (key: string | undefined) => {
    event.preventDefault();
    if (key !== undefined) select(key);
  };
  switch (event.key) {
    case "ArrowDown":
      return go(list[Math.min(i + 1, list.length - 1)]?.key);
    case "ArrowUp":
      return go(list[Math.max(i - 1, 0)]?.key);
    case "Home":
      return go(list[0]?.key);
    case "End":
      return go(list[list.length - 1]?.key);
    case "ArrowRight":
      if (!row?.entry.hasChildren) return;
      event.preventDefault();
      return row.expanded ? select(childKey(row.key, 0)) : toggle(row.key);
    case "ArrowLeft":
      if (!row) return;
      event.preventDefault();
      return row.expanded ? toggle(row.key) : row.key !== "" && select(parentKey(row.key));
    case "Enter":
      if (!row?.entry.hasChildren) return;
      event.preventDefault();
      return toggle(row.key);
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
      class="grid shrink-0 border-b border-border bg-card px-2 text-xs font-medium text-muted-foreground [&>div]:py-1.5"
      :class="COLUMNS"
    >
      <div class="pl-6">Name</div>
      <div class="pr-3 text-right">Größe</div>
      <div class="px-2">Anteil</div>
      <div class="pr-3 text-right">Dateien</div>
      <div class="pr-3 text-right">Ordner</div>
      <div class="pr-1">Geändert</div>
    </div>

    <EntryMenu :path="menuPath" :is-dir="menuIsDir">
      <div
        ref="container"
        class="min-h-0 flex-1 overflow-auto py-1 outline-none"
        tabindex="0"
        @keydown="onKeydown"
      >
        <template v-for="row in rows" :key="row.key">
          <div
            v-if="'entry' in row"
            :data-key="row.key"
            class="grid h-[26px] cursor-default items-center px-2 text-[13px]"
            :class="[COLUMNS, row.key === state.selected ? 'bg-accent/15' : 'hover:bg-foreground/5']"
            @click="select(row.key)"
            @dblclick="row.entry.hasChildren && toggle(row.key)"
            @contextmenu="(menuKey = row.key), select(row.key)"
          >
            <div class="flex min-w-0 items-center gap-1.5" :style="{ paddingLeft: `${row.depth * 16}px` }">
              <button
                v-if="row.entry.hasChildren"
                class="grid size-4.5 shrink-0 place-items-center rounded text-muted-foreground hover:bg-foreground/10 hover:text-foreground"
                :aria-label="row.expanded ? 'Zuklappen' : 'Aufklappen'"
                tabindex="-1"
                @click.stop="toggle(row.key)"
                @dblclick.stop
              >
                <ChevronRight class="size-3.5 transition-transform" :class="row.expanded && 'rotate-90'" />
              </button>
              <span v-else class="w-4.5 shrink-0" />
              <FolderOpen v-if="row.entry.isDir && row.expanded" class="size-4 shrink-0 text-accent-text" />
              <Folder v-else-if="row.entry.isDir" class="size-4 shrink-0 text-accent-text" />
              <File v-else class="size-4 shrink-0 text-muted-foreground" />
              <span class="truncate" :class="row.depth === 0 && 'font-semibold'">{{ row.entry.name }}</span>
              <span v-if="row.entry.errors" class="shrink-0" :title="`${formatCount(row.entry.errors)} Einträge nicht lesbar`">
                <TriangleAlert class="size-3.5 text-amber-500" />
              </span>
            </div>
            <div class="pr-3 text-right font-medium tabular-nums">{{ formatBytes(row.entry.size) }}</div>
            <div class="flex items-center gap-2 px-2">
              <div class="h-2.5 flex-1 overflow-hidden rounded-sm bg-foreground/8">
                <div
                  class="h-full rounded-sm bg-accent"
                  :style="{ width: `${row.parentSize > 0 ? (row.entry.size / row.parentSize) * 100 : 0}%` }"
                />
              </div>
              <span class="w-12 text-right text-xs text-muted-foreground tabular-nums">
                {{ formatPercent(row.entry.size, row.parentSize) }}
              </span>
            </div>
            <div class="pr-3 text-right text-muted-foreground tabular-nums">
              {{ row.entry.isDir ? formatCount(row.entry.files) : "" }}
            </div>
            <div class="pr-3 text-right text-muted-foreground tabular-nums">
              {{ row.entry.isDir ? formatCount(row.entry.dirs) : "" }}
            </div>
            <div class="truncate pr-1 text-xs text-muted-foreground tabular-nums">{{ formatDate(row.entry.modified) }}</div>
          </div>
          <div
            v-else
            class="flex h-[26px] items-center px-2 text-xs text-muted-foreground italic"
            :style="{ paddingLeft: `${row.depth * 16 + 8 + 24}px` }"
          >
            … und {{ formatCount(row.more) }} weitere kleinere Einträge
          </div>
        </template>
      </div>
    </EntryMenu>
  </div>
</template>
