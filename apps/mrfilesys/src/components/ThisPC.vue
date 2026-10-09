<script setup lang="ts">
import { computed } from "vue";
import { Disc, Download, FileText, HardDrive, House, Image, Monitor, Music, Network, Star, Usb, Video } from "@lucide/vue";
import EntryIcon from "./EntryIcon.vue";
import { openEntry } from "@/lib/actions";
import { formatBytes, formatPercent } from "@/lib/format";
import { baseName } from "@/lib/paths";
import { navigate, settings, state, type Place } from "@/lib/store";
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
const fallback = { fixed: "Lokaler Datenträger", removable: "USB-Laufwerk", network: "Netzlaufwerk", cdrom: "CD-Laufwerk" };

const query = computed(() => state.filter.trim().toLowerCase());
const match = (text: string) => !query.value || text.toLowerCase().includes(query.value);

function usage(drive: Drive) {
  return drive.total > 0 ? (drive.total - drive.free) / drive.total : 0;
}
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div class="mx-auto max-w-5xl px-6 py-6">
      <template v-if="settings.favorites.some((f) => match(f.path))">
        <h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Favoriten</h2>
        <div class="mb-6 grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-2">
          <button
            v-for="fav in settings.favorites.filter((f) => match(f.path))"
            :key="fav.path"
            class="flex items-center gap-3 rounded-lg border border-border bg-card px-3 py-2.5 text-left transition-colors hover:border-accent/60 hover:bg-accent/5"
            :title="fav.path"
            @click="openEntry(fav)"
          >
            <Star v-if="fav.isDir" class="size-4.5 shrink-0 text-accent-text" fill="currentColor" fill-opacity="0.25" />
            <EntryIcon v-else :entry="{ name: fav.path, isDir: false, link: false }" class="size-4.5" />
            <span class="min-w-0">
              <span class="block truncate font-medium">{{ baseName(fav.path) }}</span>
              <span class="block truncate text-xs text-muted-foreground">{{ fav.path }}</span>
            </span>
          </button>
        </div>
      </template>

      <h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Ordner</h2>
      <div class="mb-6 grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-2">
        <button
          v-for="place in state.places.filter((p) => match(p.label))"
          :key="place.path"
          class="flex items-center gap-3 rounded-lg border border-border bg-card px-3 py-2.5 text-left transition-colors hover:border-accent/60 hover:bg-accent/5"
          :title="place.path"
          @click="navigate(place.path)"
        >
          <span class="grid size-8 shrink-0 place-items-center rounded-md bg-accent/15 text-accent-text [&_svg]:size-4">
            <component :is="placeIcons[place.icon]" />
          </span>
          <span class="truncate font-medium">{{ place.label }}</span>
        </button>
      </div>

      <h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Laufwerke</h2>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3">
        <button
          v-for="drive in state.drives.filter((d) => match(d.label + d.path))"
          :key="drive.path"
          class="rounded-xl border border-border bg-card p-4 text-left transition-colors hover:border-accent/60 hover:bg-accent/5"
          @click="navigate(drive.path)"
        >
          <div class="flex items-center gap-3">
            <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-4.5">
              <component :is="driveIcons[drive.kind]" />
            </div>
            <div class="min-w-0 flex-1">
              <p class="truncate font-semibold">{{ drive.label || fallback[drive.kind] }} ({{ drive.path.slice(0, 2) }})</p>
              <p class="text-xs text-muted-foreground">{{ drive.fileSystem }}</p>
            </div>
            <span class="text-xs font-medium text-muted-foreground tabular-nums">
              {{ formatPercent(drive.total - drive.free, drive.total) }}
            </span>
          </div>
          <div class="mt-3 h-2 overflow-hidden rounded-full bg-foreground/8">
            <div
              class="h-full rounded-full"
              :class="usage(drive) > 0.9 ? 'bg-red-500' : 'bg-accent'"
              :style="{ width: `${usage(drive) * 100}%` }"
            />
          </div>
          <p class="mt-2 text-xs text-muted-foreground tabular-nums">
            {{ formatBytes(drive.free) }} frei von {{ formatBytes(drive.total) }}
          </p>
        </button>
      </div>
    </div>
  </div>
</template>
