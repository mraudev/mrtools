<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "@lucide/vue";

/** Minimize, maximize/restore and close – for windows without decorations. */
const appWindow = getCurrentWindow();
const maximized = ref(false);
let unlistenResize: (() => void) | undefined;

async function syncMaximized() {
  maximized.value = await appWindow.isMaximized();
}

onMounted(async () => {
  await syncMaximized();
  unlistenResize = await appWindow.onResized(syncMaximized);
});

onUnmounted(() => unlistenResize?.());
</script>

<template>
  <div class="flex border-l border-border [&>button]:grid [&>button]:w-11 [&>button]:place-items-center [&>button]:text-muted-foreground [&>button]:transition-colors [&_svg]:size-4">
    <button class="hover:bg-foreground/8 hover:text-foreground" aria-label="Minimieren" @click="appWindow.minimize()">
      <Minus />
    </button>
    <button
      class="hover:bg-foreground/8 hover:text-foreground"
      :aria-label="maximized ? 'Wiederherstellen' : 'Maximieren'"
      @click="appWindow.toggleMaximize()"
    >
      <Copy v-if="maximized" class="-scale-x-100 !size-3.5" />
      <Square v-else class="!size-3.5" />
    </button>
    <button class="hover:bg-red-600 hover:!text-white" aria-label="Schließen" @click="appWindow.close()">
      <X />
    </button>
  </div>
</template>
