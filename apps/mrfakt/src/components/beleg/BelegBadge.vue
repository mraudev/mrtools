<script setup lang="ts">
import { statusTitel } from "@/lib/format";

defineProps<{ status: string }>();

const farben: Record<string, string> = {
  entwurf: "bg-zinc-500/12 text-zinc-600 dark:text-zinc-300",
  gedruckt: "bg-sky-500/12 text-sky-700 dark:text-sky-300",
  festgeschrieben: "bg-emerald-500/12 text-emerald-700 dark:text-emerald-300",
  storniert: "bg-red-500/12 text-red-600 line-through dark:text-red-300",
};
</script>

<template>
  <span class="shrink-0 rounded px-1.5 py-px text-[10px] font-medium" :class="farben[status] ?? farben.entwurf">{{ statusTitel[status] ?? status }}</span>
</template>
