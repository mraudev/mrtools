<script setup lang="ts">
import { computed } from "vue";
import { AppWindow, Cpu } from "@lucide/vue";
import { icons } from "@/lib/store";
import type { ProcessInfo } from "@/lib/types";

const props = defineProps<{ process: ProcessInfo }>();
const src = computed(() => icons.get(props.process.exe) ?? null);
</script>

<template>
  <img v-if="src" :src="src" alt="" class="shrink-0" />
  <Cpu v-else-if="!process.exe" class="shrink-0 text-muted-foreground" />
  <AppWindow v-else class="shrink-0 text-muted-foreground" />
</template>
