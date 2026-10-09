<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import { state, visibleApps } from "@/lib/store";
import { appStatus } from "@/lib/version";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));

const installed = computed(() => state.apps.filter((a) => a.installed).length);
const updates = computed(
  () =>
    state.apps.filter(
      (a) => a.installed && appStatus(a.installed.version, state.releases[a.folder]?.version ?? null).kind === "outdated",
    ).length,
);
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <span class="tabular-nums">
      {{ state.apps.length }} Apps<template v-if="state.query"> · {{ visibleApps.length }} angezeigt</template>
    </span>
    <span class="tabular-nums">{{ installed }} installiert</span>
    <span v-if="updates" class="rounded bg-accent/15 px-1.5 font-medium text-accent-text tabular-nums">
      {{ updates }} {{ updates === 1 ? "Update" : "Updates" }} verfügbar
    </span>
    <div class="flex-1" />

    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
