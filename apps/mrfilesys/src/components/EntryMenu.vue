<script setup lang="ts">
import { computed } from "vue";
import {
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuPortal,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
} from "reka-ui";
import {
  AppWindow,
  ChevronRight,
  ClipboardCopy,
  ClipboardPaste,
  Copy,
  ExternalLink,
  FilePlus,
  FolderOpen,
  FolderPlus,
  FolderSearch,
  Info,
  Pencil,
  Plus,
  RefreshCw,
  Scissors,
  SquareTerminal,
  Star,
  StarOff,
  Trash,
  X,
} from "@lucide/vue";
import {
  copyPaths,
  copyToClipboard,
  createNew,
  deletePaths,
  openSelection,
  openTerminal,
  openWith,
  paste,
  showInExplorer,
  openProperties,
  startRename,
} from "@/lib/actions";
import { isFavorite, refresh, selectedEntries, state, toggleFavorite } from "@/lib/store";

/** `onEntry`: opened on a (selected) entry, otherwise on the empty folder background. */
const props = defineProps<{ onEntry: boolean }>();

const entries = computed(() => (props.onEntry ? selectedEntries.value : []));
const paths = computed(() => entries.value.map((e) => e.path));
const single = computed(() => (entries.value.length === 1 ? entries.value[0] : undefined));
</script>

<template>
  <ContextMenuPortal>
    <ContextMenuContent class="menu anim-pop">
      <template v-if="entries.length">
        <ContextMenuItem class="menu-item font-medium" @select="openSelection">
          <FolderOpen v-if="entries[0].isDir" /><ExternalLink v-else />Öffnen
          <span class="kbd ml-auto font-normal">Enter</span>
        </ContextMenuItem>
        <ContextMenuItem v-if="single && !single.isDir" class="menu-item" @select="openWith(single.path)">
          <AppWindow />Öffnen mit …
        </ContextMenuItem>
        <ContextMenuItem v-if="single?.isDir" class="menu-item" @select="openTerminal(single.path)">
          <SquareTerminal />Terminal hier öffnen
        </ContextMenuItem>
        <ContextMenuItem v-if="single" class="menu-item" @select="showInExplorer(single.path)">
          <FolderSearch />Im Explorer zeigen
        </ContextMenuItem>
        <ContextMenuItem v-if="single" class="menu-item" @select="toggleFavorite(single.path, single.isDir)">
          <template v-if="isFavorite(single.path)"><StarOff />Aus Favoriten entfernen</template>
          <template v-else><Star />Zu Favoriten hinzufügen</template>
        </ContextMenuItem>
        <ContextMenuSeparator class="menu-separator" />
        <ContextMenuItem class="menu-item" @select="copyToClipboard(paths, true)">
          <Scissors />Ausschneiden<span class="kbd ml-auto">Strg+X</span>
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="copyToClipboard(paths, false)">
          <Copy />Kopieren<span class="kbd ml-auto">Strg+C</span>
        </ContextMenuItem>
        <ContextMenuItem v-if="single?.isDir" class="menu-item" @select="paste(single.path)">
          <ClipboardPaste />In diesen Ordner einfügen
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="copyPaths(paths)">
          <ClipboardCopy />{{ paths.length > 1 ? "Pfade kopieren" : "Pfad kopieren" }}
          <span class="kbd ml-auto">Strg+Umschalt+C</span>
        </ContextMenuItem>
        <ContextMenuSeparator class="menu-separator" />
        <ContextMenuItem v-if="single" class="menu-item" @select="startRename(single.path)">
          <Pencil />Umbenennen<span class="kbd ml-auto">F2</span>
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="deletePaths(paths)">
          <Trash />In den Papierkorb<span class="kbd ml-auto">Entf</span>
        </ContextMenuItem>
        <ContextMenuItem class="menu-item text-red-500 [&_svg]:!text-red-500" @select="deletePaths(paths, true)">
          <X />Endgültig löschen …<span class="kbd ml-auto">Umschalt+Entf</span>
        </ContextMenuItem>
        <ContextMenuSeparator class="menu-separator" />
        <ContextMenuItem class="menu-item" @select="openProperties(paths)">
          <Info />Eigenschaften<span class="kbd ml-auto">Alt+Enter</span>
        </ContextMenuItem>
      </template>

      <template v-else-if="state.path">
        <ContextMenuSub>
          <ContextMenuSubTrigger class="menu-item"><Plus />Neu<ChevronRight class="ml-auto" /></ContextMenuSubTrigger>
          <ContextMenuPortal>
            <ContextMenuSubContent class="menu anim-pop" :side-offset="4">
              <ContextMenuItem class="menu-item" @select="createNew(true)"><FolderPlus />Ordner</ContextMenuItem>
              <ContextMenuItem class="menu-item" @select="createNew(false)"><FilePlus />Textdokument</ContextMenuItem>
            </ContextMenuSubContent>
          </ContextMenuPortal>
        </ContextMenuSub>
        <ContextMenuItem class="menu-item" @select="paste()">
          <ClipboardPaste />Einfügen<span class="kbd ml-auto">Strg+V</span>
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="refresh()">
          <RefreshCw />Aktualisieren<span class="kbd ml-auto">F5</span>
        </ContextMenuItem>
        <ContextMenuSeparator class="menu-separator" />
        <ContextMenuItem class="menu-item" @select="openTerminal(state.path)"><SquareTerminal />Terminal hier öffnen</ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="showInExplorer(state.path)"><FolderSearch />Im Explorer zeigen</ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="copyPaths([state.path])"><ClipboardCopy />Pfad kopieren</ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="toggleFavorite(state.path)">
          <template v-if="isFavorite(state.path)"><StarOff />Aus Favoriten entfernen</template>
          <template v-else><Star />Zu Favoriten hinzufügen</template>
        </ContextMenuItem>
        <ContextMenuSeparator class="menu-separator" />
        <ContextMenuItem class="menu-item" @select="openProperties([state.path])"><Info />Eigenschaften</ContextMenuItem>
      </template>
    </ContextMenuContent>
  </ContextMenuPortal>
</template>
