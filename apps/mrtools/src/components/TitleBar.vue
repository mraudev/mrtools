<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { RefreshCw, Search, X } from "@lucide/vue";
import logo from "@/assets/logo.svg";
import Tip from "@mrtools/ui/components/Tip";
import WindowControls from "@mrtools/ui/components/WindowControls";
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
</script>

<template>
  <header class="flex h-11 shrink-0 items-stretch border-b border-border bg-card select-none" data-tauri-drag-region>
    <div class="flex items-center gap-2 pr-4 pl-3.5" data-tauri-drag-region>
      <img :src="logo" alt="" class="pointer-events-none size-5" />
      <span class="pointer-events-none text-[13px] font-semibold tracking-tight">mrtools</span>
    </div>

    <div class="min-w-6 flex-1" data-tauri-drag-region />

    <div class="relative my-auto w-72 min-w-40 shrink">
      <Search class="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
      <input
        ref="search"
        v-model="state.query"
        class="input h-8 pr-16 pl-8"
        placeholder="Apps filtern"
        spellcheck="false"
        @keydown="onSearchKeydown"
      />
      <button
        v-if="state.query"
        class="icon-btn absolute top-0.5 right-0.5"
        aria-label="Filter leeren"
        @click="state.query = ''"
      >
        <X />
      </button>
      <span v-else class="kbd pointer-events-none absolute top-1.5 right-2">Strg+F</span>
    </div>

    <div class="min-w-6 flex-1" data-tauri-drag-region />

    <div class="flex items-center gap-1 pr-2">
      <Tip text="Aktualisieren (F5)" side="bottom">
        <button class="icon-btn" aria-label="Aktualisieren" :disabled="state.releasesLoading" @click="refresh">
          <RefreshCw :class="state.releasesLoading && 'animate-spin'" />
        </button>
      </Tip>
    </div>

    <WindowControls />
  </header>
</template>
