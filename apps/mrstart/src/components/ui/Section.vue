<script setup lang="ts">
import type { Component } from "vue";

defineProps<{ title: string; description?: string; icon: Component }>();
</script>

<template>
  <section class="rounded-xl border border-border bg-card">
    <header class="flex items-start gap-3 px-5 pt-4">
      <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-4">
        <component :is="icon" />
      </div>
      <div class="min-w-0 flex-1">
        <h2 class="font-semibold">{{ title }}</h2>
        <p v-if="description" class="text-xs text-muted-foreground">{{ description }}</p>
      </div>
      <slot name="actions" />
    </header>
    <div class="px-5 pt-3 pb-5">
      <slot />
    </div>
  </section>
</template>
