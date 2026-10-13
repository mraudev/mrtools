<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { TextEditor } from "@mrtools/textedit";
import { attachEditor, config, doc, zoom } from "@/lib/doc";

const host = ref<HTMLElement>();
let editor: TextEditor | undefined;

function onWheel(event: WheelEvent) {
  if (!event.ctrlKey) return;
  event.preventDefault();
  zoom(event.deltaY < 0 ? 1 : -1);
}

onMounted(() => {
  editor = new TextEditor(host.value!, { ...config.value, statusbar: false });
  attachEditor(editor);
  editor.focus();
});

onUnmounted(() => editor?.destroy());
</script>

<template>
  <div ref="host" class="editor-host h-full min-h-0" :style="{ '--zoom': doc.zoom / 100 }" @wheel="onWheel" />
</template>

<style>
/* The editor takes its colours from the app theme (light/dark). */
#app .editor-host .mrte {
  --mrte-accent: var(--accent);
  --mrte-bg: var(--card);
  --mrte-fg: var(--foreground);
  --mrte-toolbar-bg: var(--card);
  --mrte-border: var(--border);
  --mrte-muted: var(--muted-foreground);
  --mrte-hover: color-mix(in oklab, var(--foreground) 8%, transparent);
  --mrte-active-fg: var(--accent-text);
  color-scheme: inherit;
}

/* Page width and margins in em, so zoom (font size) scales them too. */
#app .editor-host .mrte-content {
  max-width: 56em;
  margin: 0 auto;
  padding: 1.9em 2.7em 3.2em;
  font-size: calc(15px * var(--zoom, 1));
}
</style>
