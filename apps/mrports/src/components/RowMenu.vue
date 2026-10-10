<script setup lang="ts">
import { ContextMenuContent, ContextMenuItem, ContextMenuPortal, ContextMenuSeparator } from "reka-ui";
import { Clipboard, Globe, Hash, SquareTerminal, FolderSearch, X } from "@lucide/vue";
import { askKill, copy, openInBrowser, showFolder } from "@/lib/actions";
import type { Row } from "@/lib/rows";

/** Context menu for a row – the caller wraps it in a ContextMenuRoot. */
defineProps<{ row?: Row }>();
</script>

<template>
  <ContextMenuPortal>
    <ContextMenuContent v-if="row" class="menu anim-pop">
      <ContextMenuItem
        v-if="row.protocol === 'TCP' && row.state === 'Lauscht'"
        class="menu-item"
        @select="openInBrowser(row.port)"
      >
        <Globe />Im Browser öffnen<span class="ml-auto text-xs text-muted-foreground">localhost:{{ row.port }}</span>
      </ContextMenuItem>
      <ContextMenuItem class="menu-item" @select="showFolder(row)"><FolderSearch />Ordner im Explorer zeigen</ContextMenuItem>
      <ContextMenuSeparator class="menu-separator" />
      <ContextMenuItem class="menu-item" @select="copy(String(row.port), 'Port')"><Hash />Port kopieren</ContextMenuItem>
      <ContextMenuItem class="menu-item" @select="copy(String(row.pid), 'PID')"><Clipboard />PID kopieren</ContextMenuItem>
      <ContextMenuItem class="menu-item" :disabled="!row.process.cmd" @select="copy(row.process.cmd, 'Befehlszeile')">
        <SquareTerminal />Befehlszeile kopieren
      </ContextMenuItem>
      <ContextMenuSeparator class="menu-separator" />
      <ContextMenuItem class="menu-item text-red-500 [&_svg]:!text-red-500" :disabled="row.pid <= 4" @select="askKill(row)">
        <X />{{ row.process.name }} beenden …<span class="kbd ml-auto">Entf</span>
      </ContextMenuItem>
    </ContextMenuContent>
  </ContextMenuPortal>
</template>
