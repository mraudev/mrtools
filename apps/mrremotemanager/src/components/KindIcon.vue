<script setup lang="ts">
import { Box, Cable, Globe, Monitor, Network, ScreenShare, SquareTerminal } from "@lucide/vue";
import type { Kind } from "@/lib/types";

defineProps<{ kind: Kind }>();

const icons = { rdp: Monitor, ssh: SquareTerminal, web: Globe, vnc: ScreenShare, telnet: Network, serial: Cable, hyperv: Box };
const colors = {
  rdp: "bg-sky-500/15 text-sky-600 dark:text-sky-400",
  ssh: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
  web: "bg-violet-500/15 text-violet-600 dark:text-violet-400",
  vnc: "bg-amber-500/15 text-amber-600 dark:text-amber-400",
  telnet: "bg-slate-500/15 text-slate-600 dark:text-slate-300",
  serial: "bg-rose-500/15 text-rose-600 dark:text-rose-400",
  hyperv: "bg-blue-500/15 text-blue-600 dark:text-blue-400",
};
</script>

<template>
  <span class="grid shrink-0 place-items-center rounded-md" :class="colors[kind]">
    <component :is="icons[kind]" class="size-[60%]" />
  </span>
</template>
