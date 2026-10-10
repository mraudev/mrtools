<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { ZoomIn, ZoomOut } from "@lucide/vue";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import Tip from "@mrtools/ui/components/Tip";
import { doc, zoom } from "@/lib/doc";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <span class="tabular-nums">{{ doc.words }} {{ doc.words === 1 ? "Wort" : "Wörter" }}</span>
    <span class="tabular-nums">· {{ doc.chars }} Zeichen</span>
    <span v-if="doc.path" class="min-w-0 truncate">· {{ doc.path }}</span>
    <div class="flex-1" />
    <div class="flex items-center">
      <Tip text="Verkleinern (Strg+−)">
        <button class="icon-btn size-6 [&_svg]:size-3.5" aria-label="Verkleinern" @click="zoom(-1)"><ZoomOut /></button>
      </Tip>
      <Tip text="Auf 100 % (Strg+0)">
        <button class="w-11 rounded text-center tabular-nums hover:bg-foreground/8 hover:text-foreground" @click="zoom(0)">
          {{ doc.zoom }} %
        </button>
      </Tip>
      <Tip text="Vergrößern (Strg++)">
        <button class="icon-btn size-6 [&_svg]:size-3.5" aria-label="Vergrößern" @click="zoom(1)"><ZoomIn /></button>
      </Tip>
    </div>
    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
