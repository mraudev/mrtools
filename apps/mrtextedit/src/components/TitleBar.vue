<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { DropdownMenuContent, DropdownMenuItem, DropdownMenuPortal, DropdownMenuRoot, DropdownMenuSeparator, DropdownMenuTrigger } from "reka-ui";
import { ChevronDown, FileText, FilePlus, FolderOpen, Printer, Save, SaveAll, SlidersHorizontal, Trash2 } from "@lucide/vue";
import logo from "@/assets/logo.svg";
import Tip from "@mrtools/ui/components/Tip";
import WindowControls from "@mrtools/ui/components/WindowControls";
import { doc, forgetRecent, newDoc, openDoc, printDoc, saveDoc, saveDocAs, zoom } from "@/lib/doc";
import { fileName } from "@/lib/document";

const name = computed(() => (doc.path ? fileName(doc.path) : "Unbenannt"));

function onKeydown(event: KeyboardEvent) {
  // The editor handles some keys itself (e.g. Strg+P when it has a print button).
  if (!event.ctrlKey || event.altKey || event.defaultPrevented) return;
  const key = event.key.toLowerCase();
  const actions: Record<string, () => unknown> = {
    n: newDoc,
    o: () => openDoc(),
    s: event.shiftKey ? saveDocAs : saveDoc,
    p: printDoc,
    "+": () => zoom(1),
    "=": () => zoom(1),
    "-": () => zoom(-1),
    "0": () => zoom(0),
  };
  const action = actions[key];
  if (!action) return;
  event.preventDefault();
  action();
}

onMounted(() => window.addEventListener("keydown", onKeydown));

onUnmounted(() => window.removeEventListener("keydown", onKeydown));

const actions = [
  { label: "Neu (Strg+N)", icon: FilePlus, run: () => newDoc() },
  { label: "Speichern (Strg+S)", icon: Save, run: () => saveDoc() },
  { label: "Speichern unter (Strg+Umschalt+S)", icon: SaveAll, run: () => saveDocAs() },
  { label: "Drucken (Strg+P)", icon: Printer, run: () => printDoc() },
];
</script>

<template>
  <header class="flex h-11 shrink-0 items-stretch border-b border-border bg-card select-none" data-tauri-drag-region>
    <div class="flex items-center gap-2 pr-3 pl-3.5" data-tauri-drag-region>
      <img :src="logo" alt="" class="pointer-events-none size-5" />
      <span class="pointer-events-none text-[13px] font-semibold tracking-tight">mrtextedit</span>
    </div>

    <div class="flex items-center gap-1">
      <Tip :text="actions[0].label" side="bottom">
        <button class="icon-btn" :aria-label="actions[0].label" @click="actions[0].run"><FilePlus /></button>
      </Tip>
      <div class="flex">
        <Tip text="Öffnen (Strg+O)" side="bottom">
          <button class="icon-btn rounded-r-none" aria-label="Öffnen" @click="openDoc()"><FolderOpen /></button>
        </Tip>
        <DropdownMenuRoot>
          <Tip text="Zuletzt geöffnet" side="bottom">
            <DropdownMenuTrigger as-child>
              <button class="icon-btn w-4 rounded-l-none [&_svg]:size-3" aria-label="Zuletzt geöffnet"><ChevronDown /></button>
            </DropdownMenuTrigger>
          </Tip>
          <DropdownMenuPortal>
            <DropdownMenuContent align="start" :side-offset="4" class="menu anim-fade max-w-md">
              <DropdownMenuItem v-if="doc.recent.length === 0" class="menu-item" disabled>Noch keine Dateien</DropdownMenuItem>
              <DropdownMenuItem
                v-for="path in doc.recent"
                :key="path"
                class="menu-item"
                :title="path"
                @select="openDoc(path)"
              >
                <FileText />
                <span class="min-w-0 truncate">{{ fileName(path) }}</span>
                <span class="ml-auto min-w-0 truncate pl-3 text-xs text-muted-foreground">{{ path.slice(0, -fileName(path).length - 1) }}</span>
              </DropdownMenuItem>
              <template v-if="doc.recent.length">
                <DropdownMenuSeparator class="menu-separator" />
                <DropdownMenuItem class="menu-item" @select="forgetRecent"><Trash2 />Liste leeren</DropdownMenuItem>
              </template>
            </DropdownMenuContent>
          </DropdownMenuPortal>
        </DropdownMenuRoot>
      </div>
      <Tip v-for="action in actions.slice(1)" :key="action.label" :text="action.label" side="bottom">
        <button class="icon-btn" :aria-label="action.label" @click="action.run"><component :is="action.icon" /></button>
      </Tip>
    </div>

    <div class="flex min-w-0 flex-1 items-center justify-center px-4 text-[13px]" data-tauri-drag-region>
      <span class="pointer-events-none truncate" :title="doc.path ?? undefined">
        <span v-if="doc.dirty" class="mr-1 text-accent-text">•</span>{{ name }}
      </span>
    </div>

    <div class="flex items-center gap-1 pr-2">
      <Tip text="Toolleiste und Formate" side="bottom">
        <button class="icon-btn" aria-label="Toolleiste und Formate" @click="doc.configOpen = true">
          <SlidersHorizontal />
        </button>
      </Tip>
    </div>

    <WindowControls />
  </header>
</template>
