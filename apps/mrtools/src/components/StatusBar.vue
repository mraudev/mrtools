<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import Tip from "@mrtools/ui/components/Tip";
import { openFolder } from "@/lib/actions";
import { state, visibleApps } from "@/lib/store";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));

const installed = computed(() => state.apps.filter((a) => a.installed).length);
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <span class="tabular-nums">
      {{ state.apps.length }} Apps<template v-if="state.query"> · {{ visibleApps.length }} angezeigt</template>
    </span>
    <span class="tabular-nums">{{ installed }} installiert</span>
    <Tip text="Ordner im Explorer öffnen">
      <button class="min-w-0 truncate font-mono hover:text-foreground" @click="openFolder(state.root)">
        {{ state.root }}
      </button>
    </Tip>
    <div class="flex-1" />

    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
