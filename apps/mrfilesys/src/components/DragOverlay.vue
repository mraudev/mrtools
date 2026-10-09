<script setup lang="ts">
import { computed } from "vue";
import { ArrowUpDown, Ban, Copy, MoveRight, Star } from "@lucide/vue";
import { drag } from "@/lib/dnd";
import { baseName } from "@/lib/paths";

const label = computed(() => {
  const what = drag.paths.length === 1 ? `„${baseName(drag.paths[0])}“` : `${drag.paths.length} Elemente`;
  if (drag.pin >= 0) return drag.reorder ? `${what} hierher verschieben` : `${what} an Favoriten anheften`;
  if (!drag.op) return what;
  return `${what} ${drag.op === "move" ? "verschieben" : "kopieren"} nach „${baseName(drag.target)}“`;
});
</script>

<template>
  <!-- Files from outside get Windows' own drag image – only the hint is added. -->
  <div
    v-if="drag.active && (drag.op || drag.pin >= 0 || !drag.external)"
    class="pointer-events-none fixed z-[80] flex max-w-80 items-center gap-2 rounded-md border border-border bg-popover px-2.5 py-1.5 text-xs shadow-xl"
    :style="{ left: `${drag.x + 16}px`, top: `${drag.y + 18}px` }"
  >
    <ArrowUpDown v-if="drag.pin >= 0 && drag.reorder" class="size-3.5 shrink-0 text-accent-text" />
    <Star v-else-if="drag.pin >= 0" class="size-3.5 shrink-0 text-accent-text" />
    <MoveRight v-else-if="drag.op === 'move'" class="size-3.5 shrink-0 text-accent-text" />
    <Copy v-else-if="drag.op === 'copy'" class="size-3.5 shrink-0 text-accent-text" />
    <Ban v-else class="size-3.5 shrink-0 text-muted-foreground" />
    <span class="truncate">{{ label }}</span>
  </div>
</template>
