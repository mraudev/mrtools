<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { api } from "@/lib/api";
import { formatBytes, formatDate } from "@/lib/format";
import { indexOf, reveal, root, state } from "@/lib/store";
import { toastError } from "@mrtools/ui/lib/toast";
import type { FileHit, NodeView } from "@/lib/types";
import EntryMenu from "./EntryMenu.vue";

const props = defineProps<{ node: NodeView; nodeKey: string }>();

const files = ref<FileHit[]>([]);
const menuPath = ref("");

watch(
  () => props.nodeKey,
  async (key) => {
    try {
      const result = await api.largestFiles(indexOf(key), 200);
      if (key === props.nodeKey) files.value = result;
    } catch (e) {
      toastError("Größte Dateien konnten nicht ermittelt werden", e);
    }
  },
  { immediate: true },
);

const rootPath = computed(() => root.value?.path ?? "");

/** Folder of the file relative to the scan root. */
function folder(path: string, name: string) {
  const dir = path.slice(0, path.length - name.length);
  return dir.startsWith(rootPath.value) ? dir.slice(rootPath.value.length).replace(/^\\|\\$/g, "") || "." : dir;
}
</script>

<template>
  <EntryMenu :path="menuPath" :is-dir="false">
    <div class="h-full overflow-y-auto">
      <table class="w-full table-fixed text-[13px]">
        <thead class="sticky top-0 bg-card text-xs text-muted-foreground">
          <tr class="[&>th]:py-1.5 [&>th]:font-medium">
            <th class="pl-1 text-left">Datei</th>
            <th class="w-24 pr-3 text-right">Größe</th>
            <th class="w-32 pr-1 text-left">Geändert</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="f in files"
            :key="f.path"
            class="cursor-default [&>td]:py-1"
            :class="state.selected === f.index.join('/') ? 'bg-accent/15' : 'hover:bg-foreground/5'"
            :title="f.path"
            @click="reveal(f.index.join('/'))"
            @contextmenu="menuPath = f.path"
          >
            <td class="min-w-0 pl-1">
              <p class="truncate">{{ f.name }}</p>
              <p class="truncate text-xs text-muted-foreground">{{ folder(f.path, f.name) }}</p>
            </td>
            <td class="pr-3 text-right font-medium tabular-nums">{{ formatBytes(f.size) }}</td>
            <td class="pr-1 text-xs text-muted-foreground tabular-nums">{{ formatDate(f.modified) }}</td>
          </tr>
        </tbody>
      </table>
      <p v-if="!files.length" class="py-8 text-center text-muted-foreground">Keine Dateien</p>
    </div>
  </EntryMenu>
</template>
