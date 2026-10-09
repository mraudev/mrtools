<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, HardDrive, Usb } from "@lucide/vue";
import { formatBytes, formatPercent } from "@/lib/format";
import { startScan, state } from "@/lib/store";

function usage(used: number, total: number) {
  return total > 0 ? used / total : 0;
}

async function chooseFolder() {
  const path = await open({ directory: true, title: "Ordner analysieren" });
  if (typeof path === "string") startScan(path);
}
</script>

<template>
  <div class="mx-auto max-w-5xl px-6 py-8">
    <h1 class="text-lg font-semibold tracking-tight">Was belegt den Platz?</h1>
    <p class="mt-1 text-muted-foreground">Laufwerk oder Ordner wählen – mrdiskspace zeigt, welche Ordner und Dateien am meisten Platz brauchen.</p>

    <div class="mt-6 grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3">
      <button
        v-for="drive in state.drives"
        :key="drive.path"
        class="group rounded-xl border border-border bg-card p-4 text-left transition-colors hover:border-accent/60 hover:bg-accent/5"
        @click="startScan(drive.path)"
      >
        <div class="flex items-center gap-3">
          <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-4.5">
            <Usb v-if="drive.removable" />
            <HardDrive v-else />
          </div>
          <div class="min-w-0 flex-1">
            <p class="truncate font-semibold">
              {{ drive.label || (drive.removable ? "Wechseldatenträger" : "Lokaler Datenträger") }}
              ({{ drive.path.slice(0, 2) }})
            </p>
            <p class="text-xs text-muted-foreground">{{ drive.fileSystem }}</p>
          </div>
          <span class="text-xs font-medium text-muted-foreground tabular-nums">
            {{ formatPercent(drive.total - drive.free, drive.total) }}
          </span>
        </div>
        <div class="mt-3 h-2 overflow-hidden rounded-full bg-foreground/8">
          <div
            class="h-full rounded-full"
            :class="usage(drive.total - drive.free, drive.total) > 0.9 ? 'bg-red-500' : 'bg-accent'"
            :style="{ width: `${usage(drive.total - drive.free, drive.total) * 100}%` }"
          />
        </div>
        <p class="mt-2 text-xs text-muted-foreground tabular-nums">
          {{ formatBytes(drive.free) }} frei von {{ formatBytes(drive.total) }}
        </p>
      </button>

      <button
        class="flex min-h-[124px] flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-border p-4 text-muted-foreground transition-colors hover:border-accent/60 hover:bg-accent/5 hover:text-foreground"
        @click="chooseFolder"
      >
        <FolderOpen class="size-5" />
        <span class="font-medium">Ordner wählen …</span>
        <span class="kbd">Strg+O</span>
      </button>
    </div>
  </div>
</template>
