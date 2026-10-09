<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { TooltipProvider } from "reka-ui";
import { TriangleAlert } from "@lucide/vue";
import CommandBar from "./components/CommandBar.vue";
import ConflictDialog from "./components/ConflictDialog.vue";
import DetailPanel from "./components/DetailPanel.vue";
import DragOverlay from "./components/DragOverlay.vue";
import FileList from "./components/FileList.vue";
import Sidebar from "./components/Sidebar.vue";
import StatusBar from "./components/StatusBar.vue";
import ThisPC from "./components/ThisPC.vue";
import TitleBar from "./components/TitleBar.vue";
import Toolbar from "./components/Toolbar.vue";
import Dialog from "@mrtools/ui/components/Dialog";
import Toaster from "@mrtools/ui/components/Toaster";
import {
  copyPaths,
  copyToClipboard,
  createNew,
  deletePaths,
  deleteSelection,
  openTerminal,
  paste,
  showProperties,
  startRename,
} from "./lib/actions";
import { initDeletion } from "./lib/deletion";
import { initExternalDrop } from "./lib/dnd";
import { initTransfer } from "./lib/transfer";
import { baseName } from "./lib/paths";
import { goBack, goForward, goUp, initStore, refresh, selectAll, selectedEntries, settings, state } from "./lib/store";
import "@mrtools/ui/lib/theme";

const toolbar = ref<InstanceType<typeof Toolbar>>();

const confirmOpen = computed({
  get: () => state.confirmDelete.length > 0,
  set: (open) => !open && (state.confirmDelete = []),
});

function onKeydown(event: KeyboardEvent) {
  const typing = (event.target as HTMLElement).closest("input, textarea");
  const ctrl = event.ctrlKey && !event.altKey;
  const key = event.key.toLowerCase();
  const paths = selectedEntries.value.map((e) => e.path);
  const run = (action: () => unknown) => {
    event.preventDefault();
    action();
  };

  // Work everywhere, also while typing in the filter.
  if (event.key === "F5") return run(() => refresh());
  if ((ctrl && key === "l") || (event.altKey && key === "d") || event.key === "F4") return run(() => toolbar.value?.editAddress());
  if ((ctrl && key === "f") || event.key === "F3") return run(() => (document.getElementById("search") as HTMLInputElement)?.select());
  if (event.altKey && event.key === "ArrowLeft") return run(goBack);
  if (event.altKey && event.key === "ArrowRight") return run(goForward);
  if (event.altKey && event.key === "ArrowUp") return run(goUp);
  if (ctrl && key === "h") return run(() => (settings.showHidden = !settings.showHidden));
  if (event.altKey && key === "p") return run(() => (settings.showDetails = !settings.showDetails));
  if (typing || state.confirmDelete.length) return;

  if (event.key === "Backspace") return run(goUp);
  if (ctrl && event.shiftKey && key === "n") return run(() => createNew(true));
  if (ctrl && event.shiftKey && key === "c") return run(() => copyPaths(paths.length ? paths : [state.path].filter(Boolean)));
  if (ctrl && key === "c") return run(() => copyToClipboard(paths, false));
  if (ctrl && key === "x") return run(() => copyToClipboard(paths, true));
  if (ctrl && key === "v") return run(() => paste());
  if (ctrl && key === "a") return run(selectAll);
  if (ctrl && key === "t" && state.path) return run(() => openTerminal(state.path));
  if (event.key === "Delete") return run(() => deleteSelection(event.shiftKey));
  if (event.key === "F2" && paths.length === 1) return run(() => startRename(paths[0]));
  if (event.altKey && event.key === "Enter") {
    const target = paths.length === 1 ? paths[0] : state.path;
    if (target) run(() => showProperties(target));
  }
}

// The back and forward buttons on the mouse.
function onMouseup(event: MouseEvent) {
  if (event.button === 3) goBack();
  else if (event.button === 4) goForward();
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("mouseup", onMouseup);
  initStore();
  initExternalDrop();
  initDeletion();
  initTransfer();
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("mouseup", onMouseup);
});
</script>

<template>
  <TooltipProvider :delay-duration="400" :skip-delay-duration="200">
    <div class="flex h-full flex-col">
      <TitleBar />
      <Toolbar ref="toolbar" />
      <main class="flex min-h-0 flex-1">
        <div class="w-56 shrink-0 border-r border-border bg-card">
          <Sidebar />
        </div>
        <div class="flex min-w-0 flex-1 flex-col bg-background">
          <template v-if="state.path">
            <CommandBar />
            <div class="min-h-0 flex-1">
              <FileList />
            </div>
          </template>
          <ThisPC v-else />
        </div>
        <div v-if="settings.showDetails" class="w-80 shrink-0 border-l border-border">
          <DetailPanel />
        </div>
      </main>
      <StatusBar />
    </div>

    <Dialog v-model:open="confirmOpen" title="Endgültig löschen?" size="sm">
      <template #icon>
        <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-red-500/15 text-red-500">
          <TriangleAlert class="size-4" />
        </div>
      </template>
      <p>
        <template v-if="state.confirmDelete.length === 1">„{{ baseName(state.confirmDelete[0]) }}“ wird</template>
        <template v-else>{{ state.confirmDelete.length }} Elemente werden</template>
        dauerhaft gelöscht – ohne Umweg über den Papierkorb. Das lässt sich nicht rückgängig machen.
      </p>
      <template #footer>
        <div class="flex-1" />
        <button class="btn btn-outline" @click="state.confirmDelete = []">Abbrechen</button>
        <button class="btn bg-red-600 text-white hover:bg-red-600/85" @click="deletePaths([...state.confirmDelete], true, true)">
          Endgültig löschen
        </button>
      </template>
    </Dialog>
    <ConflictDialog />
    <DragOverlay />
    <Toaster />
  </TooltipProvider>
</template>
