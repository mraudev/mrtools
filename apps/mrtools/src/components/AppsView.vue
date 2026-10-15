<script setup lang="ts">
import { SearchX } from "@lucide/vue";
import AppTile from "./AppTile.vue";
import { categories, state, visibleApps } from "@/lib/store";
</script>

<template>
  <div class="p-5">
    <div v-if="visibleApps.length && state.grouped" class="flex flex-col gap-6">
      <section v-for="category in categories" :key="category.name">
        <h2 class="mb-2.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {{ category.name }}
          <span class="ml-1 font-normal tabular-nums opacity-70">{{ category.apps.length }}</span>
        </h2>
        <div class="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] gap-4">
          <AppTile v-for="app in category.apps" :key="app.folder" :app="app" />
        </div>
      </section>
    </div>
    <div v-else-if="visibleApps.length" class="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] gap-4">
      <AppTile v-for="app in visibleApps" :key="app.folder" :app="app" />
    </div>

    <div v-else-if="state.query" class="mt-24 flex flex-col items-center text-center text-muted-foreground">
      <SearchX class="mb-3 size-10 opacity-60" />
      <p>Keine App passt zu „{{ state.query }}“.</p>
    </div>
  </div>
</template>
