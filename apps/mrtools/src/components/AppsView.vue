<script setup lang="ts">
import { FolderSearch, SearchX, TriangleAlert } from "@lucide/vue";
import AppTile from "./AppTile.vue";
import logo from "@/assets/logo.svg";
import { chooseRoot } from "@/lib/actions";
import { state, visibleApps } from "@/lib/store";
</script>

<template>
  <div class="p-5">
    <div v-if="visibleApps.length" class="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] gap-4">
      <AppTile v-for="app in visibleApps" :key="app.path" :app="app" />
    </div>

    <div v-else-if="state.error" class="mx-auto mt-20 flex max-w-md flex-col items-center text-center">
      <TriangleAlert class="mb-3 size-10 text-red-500 opacity-80" />
      <h2 class="text-lg font-semibold">Ordner nicht lesbar</h2>
      <p class="mt-2 text-xs break-all text-muted-foreground select-text">{{ state.error }}</p>
      <button class="btn btn-primary mt-6" @click="chooseRoot"><FolderSearch />Anderen Ordner wählen</button>
    </div>

    <div v-else-if="state.query" class="mt-24 flex flex-col items-center text-center text-muted-foreground">
      <SearchX class="mb-3 size-10 opacity-60" />
      <p>Keine App passt zu „{{ state.query }}“.</p>
    </div>

    <div v-else class="mx-auto mt-20 flex max-w-md flex-col items-center text-center">
      <img :src="logo" alt="" class="mb-5 size-16 drop-shadow-lg" />
      <h2 class="text-xl font-semibold">Keine Apps gefunden</h2>
      <p class="mt-2 text-muted-foreground">
        Jeder Unterordner von <span class="font-mono text-xs">{{ state.root }}</span> erscheint hier als App.
      </p>
      <button class="btn btn-primary mt-6" @click="chooseRoot"><FolderSearch />Ordner wählen</button>
    </div>
  </div>
</template>
