<script setup lang="ts">
import { ContextMenuContent, ContextMenuItem, ContextMenuPortal, ContextMenuRoot, ContextMenuTrigger } from "reka-ui";
import {
  Disc,
  Download,
  ExternalLink,
  FileText,
  Folder,
  FolderSearch,
  HardDrive,
  House,
  Image,
  Monitor,
  Music,
  Network,
  SquareTerminal,
  Star,
  StarOff,
  Usb,
  Video,
} from "@lucide/vue";
import { ref } from "vue";
import EntryIcon from "./EntryIcon.vue";
import SectionHeader from "./SectionHeader.vue";
import { openEntry, openTerminal, showInExplorer } from "@/lib/actions";
import { beginFavoriteDrag, drag } from "@/lib/dnd";
import { formatBytes } from "@/lib/format";
import { baseName, isInside } from "@/lib/paths";
import { isCollapsed, isFavorite, navigate, settings, state, toggleFavorite, toggleSection, type Place } from "@/lib/store";
import type { Drive } from "@/lib/types";

const placeIcons: Record<Place["icon"], unknown> = {
  home: House,
  desktop: Monitor,
  downloads: Download,
  documents: FileText,
  pictures: Image,
  music: Music,
  videos: Video,
};
const driveIcons: Record<Drive["kind"], unknown> = { fixed: HardDrive, removable: Usb, network: Network, cdrom: Disc };

const menuPath = ref("");
/** The right-clicked entry is a folder (favorites can be files). */
const menuIsDir = ref(true);

function active(path: string) {
  return state.path.toLowerCase() === path.toLowerCase();
}

function driveLabel(drive: Drive) {
  const fallback = { fixed: "Lokaler Datenträger", removable: "USB-Laufwerk", network: "Netzlaufwerk", cdrom: "CD-Laufwerk" };
  return `${drive.label || fallback[drive.kind]} (${drive.path.slice(0, 2)})`;
}

function usage(drive: Drive) {
  return drive.total > 0 ? (drive.total - drive.free) / drive.total : 0;
}

const item =
  "flex h-7 w-full min-w-0 items-center gap-2 rounded-md px-2 text-left text-[13px] transition-colors [&_svg]:size-4 [&_svg]:shrink-0";
const idle = "text-foreground/85 hover:bg-foreground/5";
const current = "bg-accent/15 font-medium text-accent-text";
const dropping = "ring-2 ring-accent ring-inset bg-accent/20";
</script>

<template>
  <ContextMenuRoot>
    <ContextMenuTrigger as-child>
      <nav
        class="flex h-full flex-col gap-3 overflow-y-auto px-2 py-3"
        aria-label="Orte"
        @contextmenu.capture="(menuPath = ''), (menuIsDir = true)"
      >
        <section>
          <button :class="[item, state.path === '' ? current : idle]" @click="navigate('')">
            <Monitor class="text-muted-foreground" />Dieser PC
          </button>
        </section>

        <section v-if="settings.favorites.length || drag.active" data-pin-zone>
          <SectionHeader
            label="Favoriten"
            :open="!isCollapsed('favorites') || drag.active"
            :class="drag.pin === 0 && !settings.favorites.length && '[&_button]:text-accent-text'"
            @toggle="toggleSection('favorites')"
          />
          <template v-if="!isCollapsed('favorites') || drag.active">
            <button
              v-for="(fav, i) in settings.favorites"
              :key="fav.path"
              class="relative"
              :class="[
                item,
                active(fav.path) ? current : idle,
                drag.op && drag.target === fav.path && dropping,
                drag.reorder && drag.paths[0] === fav.path && 'opacity-50',
              ]"
              :data-pin-index="i"
              :data-drop="fav.isDir ? fav.path : undefined"
              :title="fav.path"
              @click="openEntry(fav)"
              @pointerdown="beginFavoriteDrag($event, fav.path)"
              @contextmenu="(menuPath = fav.path), (menuIsDir = fav.isDir)"
            >
              <span v-if="drag.pin === i" class="absolute inset-x-1 -top-px h-0.5 rounded-full bg-accent" />
              <span
                v-if="drag.pin === i + 1 && i === settings.favorites.length - 1"
                class="absolute inset-x-1 -bottom-px h-0.5 rounded-full bg-accent"
              />
              <Star v-if="fav.isDir" class="text-accent-text" fill="currentColor" fill-opacity="0.25" />
              <EntryIcon v-else :entry="{ name: fav.path, isDir: false, link: false }" />
              <span class="truncate">{{ baseName(fav.path) }}</span>
            </button>
            <div
              v-if="!settings.favorites.length"
              class="mx-1 rounded-md border border-dashed px-2 py-2 text-center text-xs"
              :class="drag.pin >= 0 ? 'border-accent bg-accent/10 text-accent-text' : 'border-border text-muted-foreground'"
            >
              Hierher ziehen zum Anheften
            </div>
          </template>
        </section>

        <section>
          <SectionHeader label="Schnellzugriff" :open="!isCollapsed('places')" @toggle="toggleSection('places')" />
          <button
            v-for="place in isCollapsed('places') ? [] : state.places"
            :key="place.path"
            :class="[item, active(place.path) ? current : idle, drag.op && drag.target === place.path && dropping]"
            :data-drop="place.path"
            :title="place.path"
            @click="navigate(place.path)"
            @contextmenu="menuPath = place.path"
          >
            <component :is="placeIcons[place.icon]" class="text-accent-text" />
            <span class="truncate">{{ place.label }}</span>
          </button>
        </section>

        <section>
          <SectionHeader label="Laufwerke" :open="!isCollapsed('drives')" @toggle="toggleSection('drives')" />
          <button
            v-for="drive in isCollapsed('drives') ? [] : state.drives"
            :key="drive.path"
            class="h-auto flex-col items-stretch gap-1 py-1.5"
            :class="[item, isInside(state.path, drive.path) ? current : idle, drag.op && drag.target === drive.path && dropping]"
            :data-drop="drive.path"
            :title="`${formatBytes(drive.free)} frei von ${formatBytes(drive.total)}`"
            @click="navigate(drive.path)"
            @contextmenu="menuPath = drive.path"
          >
            <span class="flex min-w-0 items-center gap-2">
              <component :is="driveIcons[drive.kind]" class="text-muted-foreground" />
              <span class="truncate">{{ driveLabel(drive) }}</span>
            </span>
            <span class="ml-6 h-1 overflow-hidden rounded-full bg-foreground/10">
              <span
                class="block h-full rounded-full"
                :class="usage(drive) > 0.9 ? 'bg-red-500' : 'bg-accent'"
                :style="{ width: `${usage(drive) * 100}%` }"
              />
            </span>
          </button>
        </section>
      </nav>
    </ContextMenuTrigger>
    <ContextMenuPortal>
      <ContextMenuContent v-if="menuPath" class="menu anim-pop">
        <ContextMenuItem class="menu-item" @select="openEntry({ path: menuPath, isDir: menuIsDir })">
          <Folder v-if="menuIsDir" /><ExternalLink v-else />Öffnen
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="showInExplorer(menuPath)"><FolderSearch />Im Explorer zeigen</ContextMenuItem>
        <ContextMenuItem v-if="menuIsDir" class="menu-item" @select="openTerminal(menuPath)">
          <SquareTerminal />Terminal hier öffnen
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" @select="toggleFavorite(menuPath, menuIsDir)">
          <template v-if="isFavorite(menuPath)">
            <StarOff />Aus Favoriten entfernen
          </template>
          <template v-else><Star />Zu Favoriten hinzufügen</template>
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenuPortal>
  </ContextMenuRoot>
</template>
