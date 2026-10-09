<script setup lang="ts">
import { AppWindow } from "@lucide/vue";
import { icons } from "@/lib/store";

defineProps<{ exe: string }>();
</script>

<template>
  <img v-if="icons.get(exe)" :src="icons.get(exe)!" alt="" class="size-4 shrink-0" draggable="false" />
  <AppWindow v-else class="size-4 shrink-0 text-muted-foreground/70" />
</template>
