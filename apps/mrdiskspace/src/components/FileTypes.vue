<script setup lang="ts">
import { ref, watch } from "vue";
import { api } from "@/lib/api";
import { formatBytes, formatCount, formatPercent } from "@/lib/format";
import { indexOf } from "@/lib/store";
import { toastError } from "@mrtools/ui/lib/toast";
import type { NodeView, TypeStat } from "@/lib/types";

const props = defineProps<{ node: NodeView; nodeKey: string }>();

const types = ref<TypeStat[]>([]);

watch(
  () => props.nodeKey,
  async (key) => {
    try {
      const result = await api.fileTypes(indexOf(key), 100);
      if (key === props.nodeKey) types.value = result;
    } catch (e) {
      toastError("Dateitypen konnten nicht ermittelt werden", e);
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="h-full overflow-y-auto">
    <table class="w-full text-[13px]">
      <thead class="sticky top-0 bg-card text-xs text-muted-foreground">
        <tr class="[&>th]:py-1.5 [&>th]:font-medium">
          <th class="pl-1 text-left">Typ</th>
          <th class="w-[40%] px-3 text-left">Anteil</th>
          <th class="pr-3 text-right">Größe</th>
          <th class="pr-1 text-right">Dateien</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="t in types" :key="t.ext" class="hover:bg-foreground/5 [&>td]:py-1">
          <td class="pl-1 font-mono text-xs">
            <span v-if="t.ext">.{{ t.ext }}</span>
            <span v-else class="font-sans text-muted-foreground italic">ohne Endung</span>
          </td>
          <td class="px-3">
            <div class="flex items-center gap-2">
              <div class="h-2.5 flex-1 overflow-hidden rounded-sm bg-foreground/8">
                <div class="h-full rounded-sm bg-accent" :style="{ width: `${node.size ? (t.size / node.size) * 100 : 0}%` }" />
              </div>
              <span class="w-12 text-right text-xs text-muted-foreground tabular-nums">{{ formatPercent(t.size, node.size) }}</span>
            </div>
          </td>
          <td class="pr-3 text-right font-medium tabular-nums">{{ formatBytes(t.size) }}</td>
          <td class="pr-1 text-right text-muted-foreground tabular-nums">{{ formatCount(t.count) }}</td>
        </tr>
      </tbody>
    </table>
    <p v-if="!types.length" class="py-8 text-center text-muted-foreground">Keine Dateien</p>
  </div>
</template>
