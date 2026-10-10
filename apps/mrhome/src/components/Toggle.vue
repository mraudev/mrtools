<script setup lang="ts">
import { SwitchRoot, SwitchThumb } from "reka-ui";

const model = defineModel<boolean>({ required: true });
defineProps<{ label: string; disabled?: boolean }>();
</script>

<template>
  <SwitchRoot
    v-model="model"
    :aria-label="label"
    :disabled="disabled"
    class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-transparent bg-foreground/15 transition-colors outline-none focus-visible:ring-2 focus-visible:ring-accent/40 disabled:cursor-not-allowed disabled:opacity-40 data-[state=checked]:bg-accent"
  >
    <SwitchThumb
      class="block size-4 translate-x-0.5 rounded-full bg-white shadow transition-transform data-[state=checked]:translate-x-[18px]"
    />
  </SwitchRoot>
</template>
