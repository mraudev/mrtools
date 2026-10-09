<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { CircleAlert, CircleCheck, Download, LoaderCircle, Moon, Sun } from "@lucide/vue";
import Tip from "./ui/Tip.vue";
import { store } from "@/lib/store";
import { checkForUpdate, updater } from "@/lib/updater";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));

const dark = computed(() => {
  const theme = store.config.settings.theme;
  return theme === "dark" || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
});

function toggleTheme() {
  store.config.settings.theme = dark.value ? "light" : "dark";
}
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <span>{{ store.config.projects.length }} Projekte</span>
    <span v-if="store.watched.length">· {{ store.watched.length }} überwacht</span>
    <div class="flex-1" />

    <button
      v-if="updater.status === 'available' || updater.status === 'downloading' || updater.status === 'installing'"
      class="inline-flex items-center gap-1.5 rounded px-1.5 py-0.5 font-medium text-accent-text hover:bg-accent/15"
      @click="updater.dialogOpen = true"
    >
      <Download class="size-3.5" />Update {{ updater.version }} verfügbar
    </button>
    <span v-else-if="updater.status === 'checking'" class="inline-flex items-center gap-1.5">
      <LoaderCircle class="size-3.5 animate-spin" />Suche Updates
    </span>
    <Tip v-else-if="updater.status === 'error'" :text="`${updater.error}\nKlicken, um es erneut zu versuchen.`">
      <button class="inline-flex items-center gap-1.5 rounded px-1.5 py-0.5 hover:bg-foreground/8" @click="checkForUpdate(true)">
        <CircleAlert class="size-3.5" />Update-Prüfung fehlgeschlagen
      </button>
    </Tip>
    <span v-else-if="updater.status === 'current'" class="inline-flex items-center gap-1.5">
      <CircleCheck class="size-3.5 text-emerald-500" />Aktuell
    </span>

    <Tip :text="dark ? 'Helles Design' : 'Dunkles Design'">
      <button class="icon-btn size-6 [&_svg]:size-3.5" aria-label="Design umschalten" @click="toggleTheme">
        <Sun v-if="dark" />
        <Moon v-else />
      </button>
    </Tip>
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
