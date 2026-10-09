<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { formatBytes, formatPercent } from "@/lib/format";
import { childKey, reveal, state } from "@/lib/store";
import { squarify } from "@/lib/treemap";
import type { NodeView } from "@/lib/types";

const props = defineProps<{ node: NodeView; nodeKey: string }>();

/** More tiles would be too small to see or click. */
const MAX_TILES = 300;
const GAP = 2;

const box = ref<HTMLElement>();
const size = ref({ w: 0, h: 0 });
let observer: ResizeObserver | undefined;

onMounted(() => {
  observer = new ResizeObserver(([entry]) => {
    size.value = { w: entry.contentRect.width, h: entry.contentRect.height };
  });
  if (box.value) observer.observe(box.value);
});
onUnmounted(() => observer?.disconnect());

const tiles = computed(() => {
  const shown = props.node.children.filter((c) => c.size > 0).slice(0, MAX_TILES);
  const rest = props.node.size - shown.reduce((sum, c) => sum + c.size, 0);
  const items = shown.map((entry, i) => ({ key: childKey(props.nodeKey, i), entry, rest: false }));
  if (rest > 0) {
    items.push({
      key: `${props.nodeKey}/rest`,
      entry: { ...shown[0], name: "Weitere", size: rest, isDir: false, hasChildren: false },
      rest: true,
    });
  }
  const rects = squarify(
    items.map((t) => t.entry.size),
    { x: 0, y: 0, w: size.value.w, h: size.value.h },
  );
  return items.map((t, i) => ({ ...t, rect: rects[i] }));
});

function tileClass(isDir: boolean, rest: boolean) {
  if (rest) return "bg-foreground/6 text-muted-foreground";
  return isDir ? "bg-accent/30 hover:bg-accent/45" : "bg-foreground/10 hover:bg-foreground/18";
}
</script>

<template>
  <div ref="box" class="relative h-full min-h-0 overflow-hidden rounded-lg">
    <p v-if="!tiles.length" class="grid h-full place-items-center text-muted-foreground">Leer</p>
    <div
      v-for="tile in tiles"
      v-show="tile.rect.w > GAP && tile.rect.h > GAP"
      :key="tile.key"
      class="absolute cursor-default overflow-hidden rounded-[3px] px-1.5 py-1 text-xs leading-tight transition-colors"
      :class="[tileClass(tile.entry.isDir, tile.rest), state.selected === tile.key && 'ring-2 ring-foreground ring-inset']"
      :style="{
        left: `${tile.rect.x}px`,
        top: `${tile.rect.y}px`,
        width: `${tile.rect.w - GAP}px`,
        height: `${tile.rect.h - GAP}px`,
      }"
      :title="`${tile.entry.name}\n${formatBytes(tile.entry.size)} · ${formatPercent(tile.entry.size, node.size)}`"
      @click="!tile.rest && reveal(tile.key)"
    >
      <template v-if="tile.rect.w > 56 && tile.rect.h > 30">
        <p class="truncate font-medium">{{ tile.entry.name }}</p>
        <p class="truncate opacity-75 tabular-nums">{{ formatBytes(tile.entry.size) }}</p>
      </template>
    </div>
  </div>
</template>
