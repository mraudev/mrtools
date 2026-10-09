<script setup lang="ts">
import { Eye, EyeOff, LayoutGrid, List, PanelRight } from "@lucide/vue";
import logo from "@/assets/logo.svg";
import Segmented from "@mrtools/ui/components/Segmented";
import Tip from "@mrtools/ui/components/Tip";
import WindowControls from "@mrtools/ui/components/WindowControls";
import { settings } from "@/lib/store";

const views = [
  { value: "list" as const, label: "Liste", icon: List },
  { value: "grid" as const, label: "Kacheln", icon: LayoutGrid },
];
</script>

<template>
  <header
    class="flex h-11 shrink-0 items-stretch border-b border-border bg-card select-none"
    data-tauri-drag-region
  >
    <div class="flex items-center gap-2 pr-4 pl-3.5" data-tauri-drag-region>
      <img :src="logo" alt="" class="pointer-events-none size-5" />
      <span class="pointer-events-none text-[13px] font-semibold tracking-tight">mrfilesys</span>
    </div>

    <div class="min-w-6 flex-1" data-tauri-drag-region />

    <div class="flex items-center gap-1 pr-2">
      <Segmented v-model="settings.view" :options="views" class="mr-1" />
      <Tip :text="settings.showHidden ? 'Versteckte Dateien ausblenden (Strg+H)' : 'Versteckte Dateien anzeigen (Strg+H)'" side="bottom">
        <button
          class="icon-btn"
          :class="settings.showHidden && 'bg-accent/15 text-accent-text'"
          aria-label="Versteckte Dateien"
          :aria-pressed="settings.showHidden"
          @click="settings.showHidden = !settings.showHidden"
        >
          <Eye v-if="settings.showHidden" />
          <EyeOff v-else />
        </button>
      </Tip>
      <Tip text="Detailbereich (Alt+P)" side="bottom">
        <button
          class="icon-btn"
          :class="settings.showDetails && 'bg-accent/15 text-accent-text'"
          aria-label="Detailbereich"
          :aria-pressed="settings.showDetails"
          @click="settings.showDetails = !settings.showDetails"
        >
          <PanelRight />
        </button>
      </Tip>
    </div>

    <WindowControls />
  </header>
</template>
