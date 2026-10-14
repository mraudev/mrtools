<script setup lang="ts">
import { ContextMenuContent, ContextMenuItem, ContextMenuPortal, ContextMenuSeparator } from "reka-ui";
import { Clipboard, CopyPlus, Pencil, Play, Trash2 } from "@lucide/vue";
import { askDelete, connect, copyAddress, duplicate, edit } from "@/lib/actions";
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
      <ContextMenuSeparator class="menu-separator" />
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
