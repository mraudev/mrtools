<script setup lang="ts">
import { ref } from "vue";
import {
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuItemIndicator,
  ContextMenuPortal,
  ContextMenuRadioGroup,
  ContextMenuRadioItem,
  ContextMenuRoot,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger,
} from "reka-ui";
import {
  Check,
  ChevronRight,
  Clipboard,
  FileText,
  FolderSearch,
  Gauge,
  Globe,
  ListX,
  Pause,
  Play,
  X,
} from "@lucide/vue";
import { api } from "@/lib/api";
import {
  PRIORITIES,
  askKill,
  copy,
  searchOnline,
  setPriority,
  setSuspended,
  showInExplorer,
  showProperties,
} from "@/lib/actions";
import type { ProcInfo } from "@/lib/types";

/** Context menu for the process the user right-clicked last (set by the caller). */
const props = defineProps<{ proc: ProcInfo | undefined }>();

const priority = ref("");

async function onOpen(open: boolean) {
  priority.value = "";
  if (open && props.proc) priority.value = String((await api.details(props.proc.pid)).priority || "");
}

async function changePriority(value: unknown) {
  if (props.proc && (await setPriority(props.proc, Number(value)))) priority.value = String(value);
}
</script>

<template>
  <ContextMenuRoot @update:open="onOpen">
    <ContextMenuTrigger as-child>
      <slot />
    </ContextMenuTrigger>
    <ContextMenuPortal>
      <ContextMenuContent v-if="proc" class="menu anim-pop">
        <ContextMenuItem class="menu-item" @select="askKill(proc)">
          <X />Prozess beenden<span class="ml-auto pl-4 text-xs text-muted-foreground">Entf</span>
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="askKill(proc, true)">
          <ListX />Prozessstruktur beenden<span class="ml-auto pl-4 text-xs text-muted-foreground">⇧ Entf</span>
        </ContextMenuItem>
        <ContextMenuItem v-if="proc.suspended" class="menu-item" @select="setSuspended(proc, false)">
          <Play />Fortsetzen
        </ContextMenuItem>
        <ContextMenuItem v-else class="menu-item" @select="setSuspended(proc, true)"><Pause />Anhalten</ContextMenuItem>

        <ContextMenuSub>
          <ContextMenuSubTrigger class="menu-item">
            <Gauge />Priorität<ChevronRight class="ml-auto" />
          </ContextMenuSubTrigger>
          <ContextMenuPortal>
            <ContextMenuSubContent class="menu anim-pop min-w-48" :side-offset="4">
              <ContextMenuRadioGroup :model-value="priority" @update:model-value="changePriority">
                <ContextMenuRadioItem
                  v-for="p in PRIORITIES"
                  :key="p.value"
                  :value="String(p.value)"
                  class="menu-item pl-8 relative"
                >
                  <ContextMenuItemIndicator class="absolute left-2"><Check /></ContextMenuItemIndicator>
                  {{ p.label }}
                </ContextMenuRadioItem>
              </ContextMenuRadioGroup>
            </ContextMenuSubContent>
          </ContextMenuPortal>
        </ContextMenuSub>

        <ContextMenuSeparator class="menu-separator" />
        <ContextMenuItem class="menu-item" :disabled="!proc.exe" @select="showInExplorer(proc.exe)">
          <FolderSearch />Dateipfad öffnen
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" :disabled="!proc.exe" @select="showProperties(proc.exe)">
          <FileText />Eigenschaften
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="searchOnline(proc.name)"><Globe />Online suchen</ContextMenuItem>

        <ContextMenuSub>
          <ContextMenuSubTrigger class="menu-item">
            <Clipboard />Kopieren<ChevronRight class="ml-auto" />
          </ContextMenuSubTrigger>
          <ContextMenuPortal>
            <ContextMenuSubContent class="menu anim-pop min-w-44" :side-offset="4">
              <ContextMenuItem class="menu-item" @select="copy(proc.name, 'Name')">Name</ContextMenuItem>
              <ContextMenuItem class="menu-item" @select="copy(String(proc.pid), 'PID')">PID</ContextMenuItem>
              <ContextMenuItem class="menu-item" :disabled="!proc.exe" @select="copy(proc.exe, 'Pfad')">
                Pfad
              </ContextMenuItem>
              <ContextMenuItem class="menu-item" :disabled="!proc.cmd" @select="copy(proc.cmd, 'Befehlszeile')">
                Befehlszeile
              </ContextMenuItem>
            </ContextMenuSubContent>
          </ContextMenuPortal>
        </ContextMenuSub>
      </ContextMenuContent>
    </ContextMenuPortal>
  </ContextMenuRoot>
</template>
