<script setup lang="ts">
import { onMounted } from "vue";
import { TooltipProvider } from "reka-ui";
import ConfigDialog from "./components/ConfigDialog.vue";
import EditorPane from "./components/EditorPane.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import UnsavedDialog from "./components/UnsavedDialog.vue";
import Toaster from "@mrtools/ui/components/Toaster";
import UpdateBar from "@mrtools/ui/components/UpdateBar";
import { initDoc } from "./lib/doc";
import "@mrtools/ui/lib/theme";
import { startUpdateChecks } from "@mrtools/ui/lib/updater";

onMounted(() => {
  initDoc();
  startUpdateChecks();
});
</script>

<template>
  <TooltipProvider :delay-duration="400" :skip-delay-duration="200">
    <div class="flex h-full flex-col">
      <TitleBar />
      <UpdateBar />
      <main class="min-h-0 flex-1 bg-card">
        <EditorPane />
      </main>
      <StatusBar />
    </div>
    <ConfigDialog />
    <UnsavedDialog />
    <Toaster />
  </TooltipProvider>
</template>
