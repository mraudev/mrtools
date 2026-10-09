<script setup lang="ts">
import { TooltipContent, TooltipPortal, TooltipRoot, TooltipTrigger } from "reka-ui";

defineProps<{
  text?: string;
  side?: "top" | "bottom" | "left" | "right";
}>();
</script>

<template>
  <TooltipRoot v-if="text">
    <TooltipTrigger as-child>
      <slot />
    </TooltipTrigger>
    <TooltipPortal>
      <TooltipContent
        :side="side ?? 'top'"
        :side-offset="6"
        :collision-padding="8"
        class="anim-fade z-[60] max-w-sm rounded-md border border-border bg-popover px-2 py-1 text-xs break-words whitespace-pre-line text-foreground shadow-lg"
      >
        {{ text }}
      </TooltipContent>
    </TooltipPortal>
  </TooltipRoot>
  <slot v-else />
</template>
