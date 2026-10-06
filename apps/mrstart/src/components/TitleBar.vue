<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
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
import { launch } from "@/lib/actions";
import {
  activeView,
  moveTab,
  openProjectEditor,
  refresh,
  renameTab,
  store,
  tabs,
  visibleProjects,
} from "@/lib/store";
import { PULLS, SETTINGS } from "@/lib/types";

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

// Filter: arrow keys move the selection, Enter opens it in the editor.
watch(() => store.filter, () => (store.selected = 0));
function onFilterKeydown(event: KeyboardEvent) {
  const count = visibleProjects.value.length;
  if (!count) return;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    const step = event.key === "ArrowDown" ? 1 : -1;
    store.selected = (store.selected + step + count) % count;
  } else if (event.key === "Enter") {
    event.preventDefault();
    const project = visibleProjects.value[Math.min(store.selected, count - 1)];
    launch(event.shiftKey ? "terminal" : "editor", project);
  }
}

// Tabs: drag & drop to reorder, double-click to rename.
const dragged = ref("");
const dropTarget = ref("");
const editing = ref("");
const editValue = ref("");
const renameInput = ref<HTMLInputElement[]>([]);

function onDragStart(event: DragEvent, label: string) {
  dragged.value = label;
  event.dataTransfer?.setData("text/plain", label);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
}
function onDrop(label: string) {
  if (dragged.value) moveTab(dragged.value, label);
  dragged.value = dropTarget.value = "";
}
async function startRename(label: string) {
  editing.value = label;
  editValue.value = label;
  await nextTick();
  renameInput.value[0]?.select();
}
function finishRename(save: boolean) {
  if (save && editing.value) renameTab(editing.value, editValue.value);
  editing.value = "";
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
      <div v-if="tabs.length" class="mx-1 h-4 w-px shrink-0 bg-border" />

      <template v-for="tab in tabs" :key="tab.id">
        <input
          v-if="editing === tab.label"
          ref="renameInput"
          v-model="editValue"
          class="input h-7 w-36 shrink-0 text-[13px]"
          aria-label="Tab umbenennen"
          spellcheck="false"
          @keydown.enter.prevent="finishRename(true)"
          @keydown.escape.prevent="finishRename(false)"
          @blur="finishRename(true)"
        />
        <button
          v-else
          draggable="true"
          :title="'Ziehen zum Verschieben, Doppelklick zum Umbenennen'"
          class="inline-flex h-7 shrink-0 items-center gap-2 rounded-md border-l-2 px-2.5 text-[13px] transition-colors [&_svg]:size-3.5"
          :class="[
            activeView === tab.id && !store.filter
              ? 'bg-accent/15 font-medium text-accent-text'
              : 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground',
            dropTarget === tab.label && dragged !== tab.label ? 'border-accent' : 'border-transparent',
            dragged === tab.label && 'opacity-50',
          ]"
          @click="selectTab(tab.id)"
          @dblclick="startRename(tab.label)"
          @dragstart="onDragStart($event, tab.label)"
          @dragover.prevent="dropTarget = tab.label"
          @dragleave="dropTarget === tab.label && (dropTarget = '')"
          @drop.prevent="onDrop(tab.label)"
          @dragend="dragged = dropTarget = ''"
        >
          <Eye v-if="tab.watched" />
          <Briefcase v-else />
          <span class="capitalize">{{ tab.label }}</span>
          <span class="rounded bg-foreground/8 px-1 text-[11px] leading-4 tabular-nums">{{ tab.count }}</span>
        </button>
      </template>
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
          title="↑/↓ auswählen · Enter: im Editor öffnen · Umschalt+Enter: Terminal"
          spellcheck="false"
          @keydown="onFilterKeydown"
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
