<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { Database } from "@lucide/vue";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import { api } from "@/lib/api";
import { nav } from "@/lib/nav";
import type { DbInfo } from "@/lib/types";

const version = ref("");
const info = ref<DbInfo>();

async function laden() {
  try {
    info.value = await api.dbInfo();
  } catch {
    info.value = undefined;
  }
}

onMounted(async () => {
  version.value = await getVersion();
  laden();
});
watch(() => nav.version, laden);
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <span v-if="info" class="flex items-center gap-1.5 truncate" :title="info.pfad">
      <Database class="size-3.5" />{{ info.kunden.toLocaleString("de-DE") }} Kunden · {{ info.belege.toLocaleString("de-DE") }} Belege
    </span>
    <div class="flex-1" />
    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
