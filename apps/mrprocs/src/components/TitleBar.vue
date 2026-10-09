<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { List, ListTree, Pause, Play, RefreshCw, Search, ShieldAlert, X } from "@lucide/vue";
import logo from "@/assets/logo.svg";
import Tip from "@mrtools/ui/components/Tip";
import WindowControls from "@mrtools/ui/components/WindowControls";
import { restartAsAdmin } from "@/lib/actions";
import { refresh, state } from "@/lib/store";

const search = ref<HTMLInputElement>();

function onKeydown(event: KeyboardEvent) {
  if (event.key === "F5") {
    event.preventDefault();
    refresh();
  } else if (event.ctrlKey && event.key.toLowerCase() === "f") {
    event.preventDefault();
    search.value?.focus();
    search.value?.select();
  } else if (event.ctrlKey && event.key.toLowerCase() === "t") {
    event.preventDefault();
    state.tree = !state.tree;
  }
}

function onSearchKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    state.query = "";
    search.value?.blur();
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));

onUnmounted(() => window.removeEventListener("keydown", onKeydown));

const views = [
  { tree: false, label: "Liste", icon: List },
  { tree: true, label: "Baum", icon: ListTree },
];
</script>

<template>
  <header class="flex h-11 shrink-0 items-stretch border-b border-border bg-card select-none" data-tauri-drag-region>
    <div class="flex items-center gap-2 pr-4 pl-3.5" data-tauri-drag-region>
      <img :src="logo" alt="" class="pointer-events-none size-5" />
      <span class="pointer-events-none text-[13px] font-semibold tracking-tight">mrprocs</span>
    </div>

    <nav class="flex items-center gap-1">
      <Tip v-for="view in views" :key="view.label" text="Ansicht wechseln (Strg+T)" side="bottom">
        <button
          class="inline-flex h-7 shrink-0 items-center gap-2 rounded-md px-2.5 text-[13px] transition-colors [&_svg]:size-3.5"
          :class="
            state.tree === view.tree
              ? 'bg-accent/15 font-medium text-accent-text'
              : 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'
          "
          @click="state.tree = view.tree"
        >
          <component :is="view.icon" />
          {{ view.label }}
        </button>
      </Tip>
    </nav>

    <div class="min-w-6 flex-1" data-tauri-drag-region />

    <div class="relative my-auto w-72 min-w-40 shrink">
      <Search class="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
      <input
        ref="search"
        v-model="state.query"
        class="input h-8 pr-16 pl-8"
        placeholder="Name, PID, Pfad oder Benutzer"
        spellcheck="false"
        @keydown="onSearchKeydown"
      />
      <button
        v-if="state.query"
        class="icon-btn absolute top-0.5 right-0.5"
        aria-label="Suche leeren"
        @click="state.query = ''"
      >
        <X />
      </button>
      <span v-else class="kbd pointer-events-none absolute top-1.5 right-2">Strg+F</span>
    </div>

    <div class="min-w-6 flex-1" data-tauri-drag-region />

    <div class="flex items-center gap-1 pr-2">
      <Tip v-if="!state.elevated" text="Als Administrator neu starten – nötig, um Systemprozesse zu beenden oder zu untersuchen" side="bottom">
        <button class="icon-btn text-amber-600 dark:text-amber-400" aria-label="Als Administrator neu starten" @click="restartAsAdmin">
          <ShieldAlert />
        </button>
      </Tip>
      <Tip :text="state.paused ? 'Aktualisierung fortsetzen' : 'Aktualisierung anhalten'" side="bottom">
        <button
          class="icon-btn"
          :class="state.paused && 'bg-accent/15 !text-accent-text'"
          :aria-label="state.paused ? 'Aktualisierung fortsetzen' : 'Aktualisierung anhalten'"
          @click="state.paused = !state.paused"
        >
          <Play v-if="state.paused" />
          <Pause v-else />
        </button>
      </Tip>
      <Tip text="Jetzt aktualisieren (F5)" side="bottom">
        <button class="icon-btn" aria-label="Jetzt aktualisieren" @click="refresh"><RefreshCw /></button>
      </Tip>
    </div>

    <WindowControls />
  </header>
</template>
