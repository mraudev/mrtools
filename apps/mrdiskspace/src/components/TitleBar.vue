<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, HardDrive, RefreshCw } from "@lucide/vue";
import logo from "@/assets/logo.svg";
import Tip from "@mrtools/ui/components/Tip";
import WindowControls from "@mrtools/ui/components/WindowControls";
import { goHome, rescan, root, startScan, state } from "@/lib/store";

async function chooseFolder() {
  const path = await open({ directory: true, title: "Ordner analysieren" });
  if (typeof path === "string") startScan(path);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "F5" && state.phase === "result") {
    event.preventDefault();
    rescan();
  } else if (event.ctrlKey && event.key.toLowerCase() === "o") {
    event.preventDefault();
    chooseFolder();
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));

onUnmounted(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <header
    class="flex h-11 shrink-0 items-stretch border-b border-border bg-card select-none"
    data-tauri-drag-region
  >
    <div class="flex items-center gap-2 pr-4 pl-3.5" data-tauri-drag-region>
      <img :src="logo" alt="" class="pointer-events-none size-5" />
      <span class="pointer-events-none text-[13px] font-semibold tracking-tight">mrdiskspace</span>
    </div>

    <nav class="flex min-w-0 items-center gap-1">
      <button
        class="inline-flex h-7 shrink-0 items-center gap-2 rounded-md px-2.5 text-[13px] transition-colors [&_svg]:size-3.5"
        :class="
          state.phase === 'start'
            ? 'bg-accent/15 font-medium text-accent-text'
            : 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'
        "
        @click="goHome"
      >
        <HardDrive />
        Laufwerke
      </button>
      <template v-if="state.hasResult && root">
        <div class="mx-1 h-4 w-px shrink-0 bg-border" />
        <button
          class="inline-flex h-7 min-w-0 items-center gap-2 rounded-md px-2.5 text-[13px] transition-colors [&_svg]:size-3.5 [&_svg]:shrink-0"
          :class="
            state.phase === 'result'
              ? 'bg-accent/15 font-medium text-accent-text'
              : 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'
          "
          :title="root.path"
          @click="state.phase = 'result'"
        >
          <FolderOpen />
          <span class="truncate">{{ root.path }}</span>
        </button>
      </template>
    </nav>

    <div class="min-w-6 flex-1" data-tauri-drag-region />

    <div class="flex items-center gap-1 pr-2">
      <Tip text="Ordner analysieren (Strg+O)" side="bottom">
        <button class="icon-btn" aria-label="Ordner analysieren" @click="chooseFolder"><FolderOpen /></button>
      </Tip>
      <Tip text="Neu scannen (F5)" side="bottom">
        <button
          class="icon-btn"
          aria-label="Neu scannen"
          :disabled="state.phase !== 'result'"
          @click="rescan"
        >
          <RefreshCw />
        </button>
      </Tip>
    </div>

    <WindowControls />
  </header>
</template>
