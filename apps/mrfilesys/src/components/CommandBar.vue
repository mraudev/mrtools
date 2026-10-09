<script setup lang="ts">
import { computed } from "vue";
import { DropdownMenuContent, DropdownMenuItem, DropdownMenuPortal, DropdownMenuRoot, DropdownMenuTrigger } from "reka-ui";
import {
  ArrowDownWideNarrow,
  ArrowUpNarrowWide,
  ChevronDown,
  ClipboardPaste,
  Copy,
  FilePlus,
  FolderPlus,
  Pencil,
  Plus,
  Scissors,
  SquareTerminal,
  Trash,
} from "@lucide/vue";
import Tip from "@mrtools/ui/components/Tip";
import { copyToClipboard, createNew, deleteSelection, openTerminal, paste, startRename } from "@/lib/actions";
import { selectedEntries, settings, sortBy, state } from "@/lib/store";
import type { SortKey } from "@/lib/types";

const paths = computed(() => selectedEntries.value.map((e) => e.path));
const inFolder = computed(() => state.path !== "");

const sortOptions: { key: SortKey; label: string }[] = [
  { key: "name", label: "Name" },
  { key: "modified", label: "Änderungsdatum" },
  { key: "type", label: "Typ" },
  { key: "size", label: "Größe" },
];
const sortLabel = computed(() => sortOptions.find((o) => o.key === settings.sortKey)?.label);
</script>

<template>
  <div class="flex h-10 shrink-0 items-center gap-0.5 border-b border-border px-2">
    <DropdownMenuRoot>
      <DropdownMenuTrigger as-child>
        <button class="btn btn-ghost h-7 px-2" :disabled="!inFolder">
          <Plus />
          Neu
          <ChevronDown class="!size-3.5 text-muted-foreground" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuPortal>
        <DropdownMenuContent align="start" :side-offset="4" class="menu anim-fade">
          <DropdownMenuItem class="menu-item" @select="createNew(true)">
            <FolderPlus />Ordner<span class="kbd ml-auto">Strg+Umschalt+N</span>
          </DropdownMenuItem>
          <DropdownMenuItem class="menu-item" @select="createNew(false)"><FilePlus />Textdokument</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenuPortal>
    </DropdownMenuRoot>

    <div class="mx-1 h-4 w-px bg-border" />

    <Tip text="Ausschneiden (Strg+X)">
      <button class="icon-btn" aria-label="Ausschneiden" :disabled="!paths.length" @click="copyToClipboard(paths, true)">
        <Scissors />
      </button>
    </Tip>
    <Tip text="Kopieren (Strg+C)">
      <button class="icon-btn" aria-label="Kopieren" :disabled="!paths.length" @click="copyToClipboard(paths, false)">
        <Copy />
      </button>
    </Tip>
    <Tip text="Einfügen (Strg+V)">
      <button class="icon-btn" aria-label="Einfügen" :disabled="!inFolder" @click="paste()"><ClipboardPaste /></button>
    </Tip>
    <Tip text="Umbenennen (F2)">
      <button class="icon-btn" aria-label="Umbenennen" :disabled="paths.length !== 1" @click="startRename(paths[0])">
        <Pencil />
      </button>
    </Tip>
    <Tip text="In den Papierkorb (Entf) – Umschalt+Entf löscht endgültig">
      <button class="icon-btn hover:!text-red-500" aria-label="Löschen" :disabled="!paths.length" @click="deleteSelection()">
        <Trash />
      </button>
    </Tip>

    <div class="mx-1 h-4 w-px bg-border" />

    <Tip text="Terminal hier öffnen (Strg+T)">
      <button class="icon-btn" aria-label="Terminal hier öffnen" :disabled="!inFolder" @click="openTerminal(state.path)">
        <SquareTerminal />
      </button>
    </Tip>

    <div class="flex-1" />

    <DropdownMenuRoot>
      <DropdownMenuTrigger as-child>
        <button class="btn btn-ghost h-7 px-2 font-normal text-muted-foreground hover:text-foreground">
          <ArrowUpNarrowWide v-if="settings.ascending" />
          <ArrowDownWideNarrow v-else />
          {{ sortLabel }}
          <ChevronDown class="!size-3.5" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuPortal>
        <DropdownMenuContent align="end" :side-offset="4" class="menu anim-fade">
          <DropdownMenuItem v-for="option in sortOptions" :key="option.key" class="menu-item" @select="sortBy(option.key)">
            <span class="flex-1" :class="settings.sortKey === option.key && 'font-medium text-accent-text'">
              {{ option.label }}
            </span>
            <template v-if="settings.sortKey === option.key">
              <ArrowUpNarrowWide v-if="settings.ascending" />
              <ArrowDownWideNarrow v-else />
            </template>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenuPortal>
    </DropdownMenuRoot>
  </div>
</template>
