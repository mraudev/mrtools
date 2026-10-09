<script setup lang="ts">
import {
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";
import { X } from "@lucide/vue";

const open = defineModel<boolean>("open", { required: true });
const props = withDefaults(
  defineProps<{
    title: string;
    description?: string;
    size?: "sm" | "md" | "lg" | "xl";
    /** Prevents closing via Escape, outside click and the close button. */
    persistent?: boolean;
  }>(),
  { size: "md" },
);

const widths = { sm: "max-w-md", md: "max-w-xl", lg: "max-w-3xl", xl: "max-w-5xl" };

function guard(event: Event) {
  if (props.persistent) event.preventDefault();
}
</script>

<template>
  <DialogRoot v-model:open="open">
    <DialogPortal>
      <DialogOverlay class="anim-fade fixed inset-0 z-40 bg-black/55" />
      <DialogContent
        class="anim-pop fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100vh-4rem)] w-[calc(100vw-2rem)] -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl border border-border bg-card shadow-2xl outline-none"
        :class="widths[size]"
        @escape-key-down="guard"
        @interact-outside="guard"
      >
        <header class="flex items-start gap-3 px-5 pt-4 pb-3">
          <slot name="icon" />
          <div class="min-w-0 flex-1">
            <DialogTitle class="text-base font-semibold">{{ title }}</DialogTitle>
            <DialogDescription
              :class="description ? 'truncate text-xs text-muted-foreground' : 'sr-only'"
            >
              {{ description ?? title }}
            </DialogDescription>
          </div>
          <DialogClose v-if="!persistent" class="icon-btn -mr-1.5" aria-label="Schließen">
            <X />
          </DialogClose>
        </header>
        <div class="min-h-0 flex-1 overflow-y-auto px-5 pb-4">
          <slot />
        </div>
        <footer
          v-if="$slots.footer"
          class="flex items-center gap-2 rounded-b-xl border-t border-border bg-foreground/[0.02] px-5 py-3"
        >
          <slot name="footer" />
        </footer>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>
