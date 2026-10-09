<script setup lang="ts">
import { ContextMenuContent, ContextMenuItem, ContextMenuPortal, ContextMenuRoot, ContextMenuTrigger } from "reka-ui";
import { Clipboard, FolderSearch, ScanSearch } from "@lucide/vue";
import { copyPath, showInExplorer } from "@/lib/actions";
import { startScan } from "@/lib/store";

/** Context menu for the entry the user right-clicked last (set by the caller before it opens). */
const props = defineProps<{ path: string; isDir: boolean }>();
</script>

<template>
  <ContextMenuRoot>
    <ContextMenuTrigger as-child>
      <slot />
    </ContextMenuTrigger>
    <ContextMenuPortal>
      <ContextMenuContent
        class="anim-pop z-50 min-w-52 rounded-lg border border-border bg-popover p-1 text-[13px] shadow-xl [&_[role=menuitem]]:flex [&_[role=menuitem]]:h-8 [&_[role=menuitem]]:cursor-default [&_[role=menuitem]]:items-center [&_[role=menuitem]]:gap-2 [&_[role=menuitem]]:rounded-md [&_[role=menuitem]]:px-2 [&_[role=menuitem]]:outline-none [&_[role=menuitem][data-highlighted]]:bg-foreground/8 [&_svg]:size-4 [&_svg]:text-muted-foreground"
      >
        <ContextMenuItem @select="showInExplorer(props.path)"><FolderSearch />Im Explorer zeigen</ContextMenuItem>
        <ContextMenuItem @select="copyPath(props.path)"><Clipboard />Pfad kopieren</ContextMenuItem>
        <ContextMenuItem v-if="props.isDir" @select="startScan(props.path)">
          <ScanSearch />Diesen Ordner einzeln scannen
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenuPortal>
  </ContextMenuRoot>
</template>
