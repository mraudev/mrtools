<script setup lang="ts">
import { ChevronRight } from "@lucide/vue";

defineProps<{ label: string; open: boolean }>();
defineEmits<{ toggle: [] }>();
</script>

<template>
  <h2 class="mb-1">
    <button
      class="group flex w-full items-center gap-1 rounded px-2 py-0.5 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase transition-colors hover:text-foreground"
      :aria-expanded="open"
      @click="$emit('toggle')"
    >
      <ChevronRight class="-ml-0.5 size-3 shrink-0 transition-transform" :class="open && 'rotate-90'" />
      {{ label }}
    </button>
  </h2>
</template>
