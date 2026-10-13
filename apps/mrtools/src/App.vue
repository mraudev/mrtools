<script setup lang="ts">
import { onMounted } from "vue";
import { TooltipProvider } from "reka-ui";
import { LoaderCircle } from "@lucide/vue";
import AppsView from "./components/AppsView.vue";
import ConfirmUninstall from "./components/ConfirmUninstall.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import UpdateBar from "@mrtools/ui/components/UpdateBar";
import Toaster from "@mrtools/ui/components/Toaster";
import { refresh, state } from "./lib/store";
import { startUpdateChecks } from "@mrtools/ui/lib/updater";
import "@mrtools/ui/lib/theme";

onMounted(() => {
  refresh();
  startUpdateChecks();
});
</script>

<template>
  <TooltipProvider :delay-duration="400" :skip-delay-duration="200">
    <div class="flex h-full flex-col">
      <TitleBar />
      <UpdateBar />
      <main class="min-h-0 flex-1 overflow-y-auto">
        <div v-if="!state.loaded" class="grid h-full place-items-center text-muted-foreground">
          <LoaderCircle class="size-5 animate-spin" />
        </div>
        <AppsView v-else />
      </main>
      <StatusBar />
    </div>
    <ConfirmUninstall />
    <Toaster />
  </TooltipProvider>
</template>
