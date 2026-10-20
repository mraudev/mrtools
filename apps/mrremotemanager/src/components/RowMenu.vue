<script setup lang="ts">
import { ContextMenuContent, ContextMenuItem, ContextMenuPortal, ContextMenuSeparator } from "reka-ui";
import { Clipboard, CopyPlus, FolderSync, Pencil, Play, Star, StarOff, Trash2 } from "@lucide/vue";
import { askDelete, connect, copyAddress, duplicate, edit, openSftp, toggleFavorite } from "@/lib/actions";
import { tools } from "@/lib/store";
import type { Connection } from "@/lib/types";

/** Context menu for a connection – the caller wraps it in a ContextMenuRoot. */
defineProps<{ connection?: Connection }>();
</script>

<template>
  <ContextMenuPortal>
    <ContextMenuContent v-if="connection" class="menu anim-pop">
      <ContextMenuItem class="menu-item font-medium" @select="connect(connection)">
        <Play />Verbinden<span class="kbd ml-auto">Enter</span>
      </ContextMenuItem>
      <ContextMenuItem v-if="connection.kind === 'ssh' && tools.winscp" class="menu-item" @select="openSftp(connection)">
        <FolderSync />Dateien (SFTP in WinSCP)
      </ContextMenuItem>
      <ContextMenuSeparator class="menu-separator" />
      <ContextMenuItem class="menu-item" @select="toggleFavorite(connection)">
        <template v-if="connection.favorite"><StarOff />Kein Favorit mehr</template>
        <template v-else><Star />Zu Favoriten</template>
      </ContextMenuItem>
      <ContextMenuItem class="menu-item" @select="edit(connection)"><Pencil />Bearbeiten …<span class="kbd ml-auto">F2</span></ContextMenuItem>
      <ContextMenuItem class="menu-item" @select="duplicate(connection)"><CopyPlus />Duplizieren …</ContextMenuItem>
      <ContextMenuItem class="menu-item" @select="copyAddress(connection)"><Clipboard />Adresse kopieren</ContextMenuItem>
      <ContextMenuSeparator class="menu-separator" />
      <ContextMenuItem class="menu-item text-red-500 [&_svg]:!text-red-500" @select="askDelete(connection)">
        <Trash2 />Löschen …<span class="kbd ml-auto">Entf</span>
      </ContextMenuItem>
    </ContextMenuContent>
  </ContextMenuPortal>
</template>
