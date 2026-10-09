<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { FolderSearch, RefreshCw, Search } from "@lucide/vue";
import Tip from "@mrtools/ui/components/Tip";
import { api } from "@/lib/api";
import { showInExplorer } from "@/lib/actions";
import { formatBytes, formatCount } from "@/lib/format";
import type { Module } from "@/lib/types";

const props = defineProps<{ pid: number }>();

const modules = ref<Module[]>([]);
const error = ref("");
const loading = ref(false);
const filter = ref("");

async function load() {
  const pid = props.pid;
  loading.value = true;
  try {
    const result = await api.modules(pid);
    if (pid === props.pid) [modules.value, error.value] = [result, ""];
  } catch (e) {
    if (pid === props.pid) [modules.value, error.value] = [[], String(e)];
  } finally {
    loading.value = false;
  }
}
watch(() => props.pid, load, { immediate: true });

const shown = computed(() => {
  const q = filter.value.trim().toLowerCase();
  return q ? modules.value.filter((m) => m.path.toLowerCase().includes(q)) : modules.value;
});
</script>

<template>
  <div class="flex flex-col gap-2">
    <div class="flex items-center gap-2">
      <div class="relative flex-1">
        <Search class="pointer-events-none absolute top-2 left-2 size-4 text-muted-foreground" />
        <input v-model="filter" class="input h-8 pl-8" placeholder="Module filtern" spellcheck="false" />
      </div>
      <span class="text-xs text-muted-foreground tabular-nums">{{ formatCount(shown.length) }}</span>
      <Tip text="Neu laden">
        <button class="icon-btn" aria-label="Neu laden" :disabled="loading" @click="load"><RefreshCw /></button>
      </Tip>
    </div>
    <p v-if="error" class="rounded-lg bg-foreground/4 p-3 text-xs text-muted-foreground">
      Module nicht lesbar: {{ error }}
    </p>
    <div v-else class="min-h-0 flex-1 overflow-y-auto">
      <div
        v-for="m in shown"
        :key="m.path + m.base"
        class="group flex items-center gap-2 rounded-md px-1.5 py-1 hover:bg-foreground/5"
        :title="`${m.path}\nBasisadresse ${m.base}`"
      >
        <div class="min-w-0 flex-1">
          <p class="truncate text-[13px]">{{ m.name }}</p>
          <p class="truncate text-xs text-muted-foreground">{{ m.path }}</p>
        </div>
        <span class="text-xs text-muted-foreground tabular-nums">{{ formatBytes(m.size) }}</span>
        <button
          class="icon-btn size-6 opacity-0 group-hover:opacity-100 [&_svg]:size-3.5"
          aria-label="Im Explorer zeigen"
          @click="showInExplorer(m.path)"
        >
          <FolderSearch />
        </button>
      </div>
    </div>
  </div>
</template>
