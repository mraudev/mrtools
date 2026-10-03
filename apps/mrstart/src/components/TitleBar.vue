<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Briefcase,
  Copy,
  Eye,
  GitPullRequest,
  Minus,
  Plus,
  RefreshCw,
  Search,
  Settings,
  Square,
  X,
} from "@lucide/vue";
import logo from "@/assets/logo.svg";
import Tip from "./ui/Tip.vue";
import { dashboard } from "@/lib/dashboard";
import { activeView, openProjectEditor, refresh, store, tabs } from "@/lib/store";
import { PULLS, SETTINGS, WATCHED } from "@/lib/types";

const appWindow = getCurrentWindow();
const maximized = ref(false);
const filterInput = ref<HTMLInputElement>();
let unlistenResize: (() => void) | undefined;

async function syncMaximized() {
  maximized.value = await appWindow.isMaximized();
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "F5") {
    event.preventDefault();
    refresh();
  } else if (event.ctrlKey && event.key.toLowerCase() === "f") {
    event.preventDefault();
    filterInput.value?.focus();
    filterInput.value?.select();
  } else if (event.key === "Escape" && document.activeElement === filterInput.value) {
    store.filter = "";
    filterInput.value?.blur();
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  await syncMaximized();
  unlistenResize = await appWindow.onResized(syncMaximized);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  unlistenResize?.();
});

function selectTab(id: string) {
  store.view = id;
  store.filter = "";
}
</script>

<template>
  <header
    class="flex h-11 shrink-0 items-stretch border-b border-border bg-card select-none"
    data-tauri-drag-region
  >
    <div class="flex items-center gap-2 pr-4 pl-3.5" data-tauri-drag-region>
      <img :src="logo" alt="" class="pointer-events-none size-5" />
      <span class="pointer-events-none text-[13px] font-semibold tracking-tight">mrstart</span>
    </div>

    <nav class="no-scrollbar flex min-w-0 items-center gap-1 overflow-x-auto">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        class="inline-flex h-7 shrink-0 items-center gap-2 rounded-md px-2.5 text-[13px] transition-colors [&_svg]:size-3.5"
        :class="
          activeView === tab.id && !store.filter
            ? 'bg-accent/15 font-medium text-accent-text'
            : 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'
        "
        @click="selectTab(tab.id)"
      >
        <Eye v-if="tab.id === WATCHED" />
        <Briefcase v-else />
        <span class="capitalize">{{ tab.label }}</span>
        <span class="rounded bg-foreground/8 px-1 text-[11px] leading-4 tabular-nums">{{ tab.count }}</span>
      </button>

      <div v-if="tabs.length" class="mx-1 h-4 w-px shrink-0 bg-border" />
      <Tip
        :text="dashboard.reviewRequests.length ? `${dashboard.reviewRequests.length} Review(s) angefordert` : undefined"
        side="bottom"
      >
        <button
          class="inline-flex h-7 shrink-0 items-center gap-2 rounded-md px-2.5 text-[13px] transition-colors [&_svg]:size-3.5"
          :class="
            activeView === PULLS && !store.filter
              ? 'bg-accent/15 font-medium text-accent-text'
              : 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'
          "
          @click="selectTab(PULLS)"
        >
          <GitPullRequest />
          Pull Requests
          <span
            v-if="dashboard.reviewRequests.length"
            class="rounded bg-accent px-1 text-[11px] leading-4 font-semibold text-accent-foreground tabular-nums"
          >
            {{ dashboard.reviewRequests.length }}
          </span>
        </button>
      </Tip>
    </nav>

    <div class="min-w-6 flex-1" data-tauri-drag-region />

    <div class="flex items-center gap-1 pr-2">
      <div class="relative">
        <Search class="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
        <input
          ref="filterInput"
          v-model="store.filter"
          class="input h-7 w-48 pl-7 text-[13px]"
          placeholder="Projekte filtern"
          spellcheck="false"
        />
      </div>
      <Tip text="Aktualisieren (F5)" side="bottom">
        <button class="icon-btn" aria-label="Aktualisieren" @click="refresh"><RefreshCw /></button>
      </Tip>
      <Tip text="Neues Projekt" side="bottom">
        <button class="icon-btn" aria-label="Neues Projekt" @click="openProjectEditor()"><Plus /></button>
      </Tip>
      <Tip text="Einstellungen" side="bottom">
        <button
          class="icon-btn"
          :class="activeView === SETTINGS && 'bg-accent/15 text-accent-text'"
          aria-label="Einstellungen"
          @click="selectTab(SETTINGS)"
        >
          <Settings />
        </button>
      </Tip>
    </div>

    <div class="flex border-l border-border [&>button]:grid [&>button]:w-11 [&>button]:place-items-center [&>button]:text-muted-foreground [&>button]:transition-colors [&_svg]:size-4">
      <button class="hover:bg-foreground/8 hover:text-foreground" aria-label="Minimieren" @click="appWindow.minimize()">
        <Minus />
      </button>
      <button
        class="hover:bg-foreground/8 hover:text-foreground"
        :aria-label="maximized ? 'Wiederherstellen' : 'Maximieren'"
        @click="appWindow.toggleMaximize()"
      >
        <Copy v-if="maximized" class="-scale-x-100 !size-3.5" />
        <Square v-else class="!size-3.5" />
      </button>
      <button class="hover:bg-red-600 hover:!text-white" aria-label="Schließen" @click="appWindow.close()">
        <X />
      </button>
    </div>
  </header>
</template>
