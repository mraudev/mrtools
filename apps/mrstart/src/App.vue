<script setup lang="ts">
import { onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { TooltipProvider } from "reka-ui";
import { TriangleAlert } from "@lucide/vue";
import GitConsoleDialog from "./components/GitConsoleDialog.vue";
import ProjectDialog from "./components/ProjectDialog.vue";
import ProjectsView from "./components/ProjectsView.vue";
import SettingsView from "./components/SettingsView.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import UpdateDialog from "./components/UpdateDialog.vue";
import Toaster from "./components/ui/Toaster.vue";
import { activeView, initStore, store } from "./lib/store";
import { toast } from "./lib/toast";
import { SETTINGS } from "./lib/types";
import { startUpdateChecks } from "./lib/updater";

onMounted(async () => {
  await initStore();
  listen<{ command: string; code: number | null }>("launch-failed", ({ payload }) => {
    toast("error", `Befehl beendet mit Exit-Code ${payload.code ?? "?"}`, payload.command);
  });
  if (import.meta.env.PROD) startUpdateChecks();
});
</script>

<template>
  <TooltipProvider :delay-duration="400" :skip-delay-duration="200">
    <div class="flex h-full flex-col">
      <TitleBar />
      <div
        v-if="store.loadError"
        class="flex items-start gap-2 border-b border-red-500/30 bg-red-500/10 px-4 py-2 text-[13px]"
      >
        <TriangleAlert class="mt-0.5 size-4 shrink-0 text-red-500" />
        <p>
          Die Konfiguration konnte nicht gelesen werden – Änderungen werden nicht gespeichert, bis die
          Datei repariert ist:
          <span class="font-mono text-xs select-text">{{ store.loadError }}</span>
        </p>
      </div>
      <main class="min-h-0 flex-1 overflow-y-auto">
        <template v-if="store.loaded">
          <SettingsView v-if="activeView === SETTINGS" />
          <ProjectsView v-else />
        </template>
      </main>
      <StatusBar />
    </div>

    <ProjectDialog />
    <GitConsoleDialog />
    <UpdateDialog />
    <Toaster />
  </TooltipProvider>
</template>
