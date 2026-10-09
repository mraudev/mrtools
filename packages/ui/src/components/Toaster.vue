<script setup lang="ts">
import { CircleAlert, CircleCheck, Info, X } from "@lucide/vue";
import { dismissToast, toasts } from "../lib/toast";

const icons = { success: CircleCheck, error: CircleAlert, info: Info };
const colors = { success: "text-emerald-500", error: "text-red-500", info: "text-accent-text" };
</script>

<template>
  <div class="pointer-events-none fixed right-4 bottom-10 z-[70] flex w-96 max-w-[calc(100vw-2rem)] flex-col gap-2">
    <TransitionGroup
      enter-from-class="opacity-0 translate-y-2"
      leave-to-class="opacity-0"
      enter-active-class="transition duration-200"
      leave-active-class="transition duration-150"
    >
      <div
        v-for="t in toasts"
        :key="t.id"
        class="pointer-events-auto flex items-start gap-3 rounded-lg border border-border bg-popover p-3 shadow-xl"
        role="status"
      >
        <component :is="icons[t.kind]" class="mt-0.5 size-4 shrink-0" :class="colors[t.kind]" />
        <div class="min-w-0 flex-1">
          <p class="font-medium">{{ t.title }}</p>
          <p v-if="t.detail" class="mt-0.5 text-xs break-words text-muted-foreground select-text">
            {{ t.detail }}
          </p>
        </div>
        <button class="icon-btn -m-1 size-6" aria-label="Schließen" @click="dismissToast(t.id)">
          <X />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>
