<script setup lang="ts">
import { onMounted, watch } from "vue";
import { TooltipProvider } from "reka-ui";
import HeatingView from "./components/HeatingView.vue";
import LightsView from "./components/LightsView.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import Toaster from "@mrtools/ui/components/Toaster";
import UpdateBar from "@mrtools/ui/components/UpdateBar";
import { initLights } from "./lib/lights";
import { initStore } from "./lib/store";
import { ui } from "./lib/ui";
import "@mrtools/ui/lib/theme";
import { startUpdateChecks } from "@mrtools/ui/lib/updater";

onMounted(() => {
  initLights();
  startUpdateChecks();
  // tado counts every request – it is only loaded once the heating is actually opened.
  let heatingStarted = false;
  watch(
    () => ui.tab,
    (tab) => {
      if (tab !== "heating" || heatingStarted) return;
      heatingStarted = true;
      initStore();
    },
    { immediate: true },
  );
});
</script>

<template>
  <TooltipProvider :delay-duration="400" :skip-delay-duration="200">
    <div class="flex h-full flex-col">
      <TitleBar />
      <UpdateBar />
      <main class="min-h-0 flex-1 overflow-y-auto">
        <LightsView v-if="ui.tab === 'lights'" />
        <HeatingView v-else />
      </main>
      <StatusBar />
    </div>
    <Toaster />
  </TooltipProvider>
</template>
