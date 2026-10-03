<script setup lang="ts">
import { computed, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderPlus, Plus, SearchX } from "@lucide/vue";
import logo from "@/assets/logo.svg";
import ProjectTile from "./ProjectTile.vue";
import { loadPulls } from "@/lib/pulls";
import { activeView, openProjectEditor, store, visibleProjects } from "@/lib/store";
import { WATCHED } from "@/lib/types";

const filtering = computed(() => store.filter.trim() !== "");

// Pull requests of the current tab; not reloaded per keystroke while filtering.
watch(
  () => [visibleProjects.value.map((p) => p.path).join("|"), store.refreshTick, store.config.settings.giteaHost],
  () => {
    if (!filtering.value) loadPulls(visibleProjects.value.map((p) => p.path));
  },
  { immediate: true },
);

async function addWatchedDirectory() {
  const directory = await openDialog({ directory: true, title: "Ordner überwachen" });
  if (typeof directory === "string") {
    store.config.settings.watchedDirectories.push(directory);
    store.view = WATCHED;
  }
}
</script>

<template>
  <div class="p-5">
    <div
      v-if="visibleProjects.length"
      class="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] gap-4"
    >
      <ProjectTile
        v-for="project in visibleProjects"
        :key="project.id"
        :project="project"
        :editable="project.category !== WATCHED"
        :show-category="filtering"
      />
    </div>

    <div v-else-if="filtering" class="mt-24 flex flex-col items-center text-center text-muted-foreground">
      <SearchX class="mb-3 size-10 opacity-60" />
      <p>Keine Projekte für „{{ store.filter }}“ gefunden.</p>
    </div>

    <div v-else-if="activeView === WATCHED" class="mt-24 text-center text-muted-foreground">
      Die überwachten Ordner enthalten keine Unterordner.
    </div>

    <div v-else class="mx-auto mt-20 flex max-w-md flex-col items-center text-center">
      <img :src="logo" alt="" class="mb-5 size-16 drop-shadow-lg" />
      <h2 class="text-xl font-semibold">Willkommen bei mrstart</h2>
      <p class="mt-2 text-muted-foreground">
        Lege Projekte an oder lass einen Ordner überwachen – jeder Unterordner erscheint dann
        automatisch als Kachel mit Editor, Terminal, Apps und Git-Aktionen.
      </p>
      <div class="mt-6 flex gap-2">
        <button class="btn btn-primary" @click="openProjectEditor()"><Plus />Projekt anlegen</button>
        <button class="btn btn-outline" @click="addWatchedDirectory"><FolderPlus />Ordner überwachen</button>
      </div>
    </div>
  </div>
</template>
