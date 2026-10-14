<script setup lang="ts">
import { Globe, Monitor, SquareTerminal } from "@lucide/vue";
import type { Kind } from "@/lib/types";

defineProps<{ kind: Kind }>();

const icons = { rdp: Monitor, ssh: SquareTerminal, web: Globe };
const colors = {
  rdp: "bg-sky-500/15 text-sky-600 dark:text-sky-400",
  ssh: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
  web: "bg-violet-500/15 text-violet-600 dark:text-violet-400",
};
</script>

<template>
  <span class="grid shrink-0 place-items-center rounded-md" :class="colors[kind]">
    <component :is="icons[kind]" class="size-[60%]" />
  </span>
</template>
