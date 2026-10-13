<script setup lang="ts">
import { onMounted } from "vue";
import { TooltipProvider } from "reka-ui";
import DetailPanel from "./components/DetailPanel.vue";
import ScanningView from "./components/ScanningView.vue";
import StartView from "./components/StartView.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import TreeTable from "./components/TreeTable.vue";
import Toaster from "@mrtools/ui/components/Toaster";
import UpdateBar from "@mrtools/ui/components/UpdateBar";
import { initStore, state } from "./lib/store";
import "@mrtools/ui/lib/theme";
import { startUpdateChecks } from "@mrtools/ui/lib/updater";

onMounted(() => {
  initStore();
  startUpdateChecks();
});
</script>

<template>
  <TooltipProvider :delay-duration="400" :skip-delay-duration="200">
    <div class="flex h-full flex-col">
      <TitleBar />
      <UpdateBar />
      <main class="min-h-0 flex-1">
        <div v-if="state.phase === 'start'" class="h-full overflow-y-auto">
          <StartView />
        </div>
        <ScanningView v-else-if="state.phase === 'scanning'" />
        <div v-else class="flex h-full">
          <div class="min-w-0 flex-[3] bg-card">
            <TreeTable />
          </div>
          <div class="min-w-[320px] flex-[2] border-l border-border">
            <DetailPanel />
          </div>
        </div>
      </main>
      <StatusBar />
    </div>
    <Toaster />
  </TooltipProvider>
</template>
