<script setup lang="ts">
import { onMounted } from "vue";
import { TooltipProvider } from "reka-ui";
import { LoaderCircle } from "@lucide/vue";
import ConfirmKill from "./components/ConfirmKill.vue";
import DetailPanel from "./components/DetailPanel.vue";
import PortTable from "./components/PortTable.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
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
      <main class="flex min-h-0 flex-1">
        <div v-if="!state.loaded" class="grid flex-1 place-items-center text-muted-foreground">
          <LoaderCircle class="size-5 animate-spin" />
        </div>
        <template v-else>
          <div class="min-w-0 flex-1 bg-card">
            <PortTable />
          </div>
          <div class="w-[380px] shrink-0 border-l border-border">
            <DetailPanel />
          </div>
        </template>
      </main>
      <StatusBar />
    </div>
    <ConfirmKill />
    <Toaster />
  </TooltipProvider>
</template>
