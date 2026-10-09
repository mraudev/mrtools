<script setup lang="ts">
import { computed, ref } from "vue";
import { ArrowUp, FileType, FolderSearch, LayoutDashboard, ListOrdered } from "@lucide/vue";
import FileTypes from "./FileTypes.vue";
import LargestFiles from "./LargestFiles.vue";
import TreemapView from "./TreemapView.vue";
import Segmented from "@mrtools/ui/components/Segmented";
import Tip from "@mrtools/ui/components/Tip";
import { showInExplorer } from "@/lib/actions";
import { formatBytes, formatCount, formatPercent } from "@/lib/format";
import { detailKey, parentKey, root, select, state } from "@/lib/store";

type Tab = "map" | "types" | "files";
const tab = ref<Tab>("map");
const tabs = [
  { value: "map" as const, label: "Treemap", icon: LayoutDashboard },
  { value: "types" as const, label: "Dateitypen", icon: FileType },
  { value: "files" as const, label: "Größte Dateien", icon: ListOrdered },
];

const node = computed(() => state.nodes.get(detailKey.value));
</script>

<template>
  <div v-if="node" class="flex h-full min-w-0 flex-col gap-3 p-3">
    <div class="flex items-start gap-2">
      <div class="min-w-0 flex-1">
        <p class="truncate font-semibold" :title="node.path">{{ node.name }}</p>
        <p class="text-xs text-muted-foreground tabular-nums">
          {{ formatBytes(node.size) }}
          <template v-if="root && detailKey !== ''">· {{ formatPercent(node.size, root.size) }} vom Scan</template>
          · {{ formatCount(node.files) }} Dateien · {{ formatCount(node.dirs) }} Ordner
        </p>
      </div>
      <Tip text="Übergeordneter Ordner">
        <button
          class="icon-btn"
          aria-label="Übergeordneter Ordner"
          :disabled="detailKey === ''"
          @click="select(parentKey(detailKey))"
        >
          <ArrowUp />
        </button>
      </Tip>
      <Tip text="Im Explorer zeigen">
        <button class="icon-btn" aria-label="Im Explorer zeigen" @click="showInExplorer(node.path)">
          <FolderSearch />
        </button>
      </Tip>
    </div>

    <Segmented v-model="tab" :options="tabs" class="self-start" />

    <div class="min-h-0 flex-1">
      <TreemapView v-if="tab === 'map'" :node="node" :node-key="detailKey" />
      <FileTypes v-else-if="tab === 'types'" :node="node" :node-key="detailKey" />
      <LargestFiles v-else :node="node" :node-key="detailKey" />
    </div>
  </div>
</template>
