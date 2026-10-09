<script setup lang="ts" generic="T extends string">
import type { Component } from "vue";

const model = defineModel<T>({ required: true });
defineProps<{ options: { value: T; label: string; icon?: Component }[] }>();
</script>

<template>
  <div class="inline-flex rounded-lg border border-border bg-background p-0.5" role="radiogroup">
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      role="radio"
      :aria-checked="model === option.value"
      class="inline-flex h-7 items-center gap-1.5 rounded-md px-3 text-sm transition-colors [&_svg]:size-3.5"
      :class="
        model === option.value
          ? 'bg-accent text-accent-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground'
      "
      @click="model = option.value"
    >
      <component :is="option.icon" v-if="option.icon" />
      {{ option.label }}
    </button>
  </div>
</template>
