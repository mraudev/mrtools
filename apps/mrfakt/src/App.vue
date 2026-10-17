<script setup lang="ts">
import { onMounted } from "vue";
import { TooltipProvider } from "reka-ui";
import TitleBar from "./components/TitleBar.vue";
import SideNav from "./components/SideNav.vue";
import StatusBar from "./components/StatusBar.vue";
import Frage from "./components/Frage.vue";
import KundenSeite from "./components/kunden/KundenSeite.vue";
import BelegEditor from "./components/beleg/BelegEditor.vue";
import BriefEditor from "./components/briefe/BriefEditor.vue";
import Rechnungsbuch from "./components/journal/Rechnungsbuch.vue";
import Auswertung from "./components/journal/Auswertung.vue";
import ArtikelSeite from "./components/artikel/ArtikelSeite.vue";
import Einstellungen from "./components/einstellungen/Einstellungen.vue";
import Toaster from "@mrtools/ui/components/Toaster";
import UpdateBar from "@mrtools/ui/components/UpdateBar";
import { nav } from "./lib/nav";
import "@mrtools/ui/lib/theme";
import { startUpdateChecks } from "@mrtools/ui/lib/updater";

onMounted(() => {
  startUpdateChecks();
});
</script>

<template>
  <TooltipProvider :delay-duration="400" :skip-delay-duration="200">
    <div class="flex h-full flex-col">
      <TitleBar />
      <UpdateBar />
      <main class="flex min-h-0 flex-1">
        <SideNav />
        <BelegEditor v-if="nav.belegId" :key="'b' + nav.belegId" :id="nav.belegId" />
        <BriefEditor v-else-if="nav.briefId" :key="'r' + nav.briefId" :id="nav.briefId" />
        <KundenSeite v-else-if="nav.seite === 'kunden'" />
        <Rechnungsbuch v-else-if="nav.seite === 'rechnungsbuch'" />
        <Auswertung v-else-if="nav.seite === 'auswertung'" />
        <ArtikelSeite v-else-if="nav.seite === 'artikel'" />
        <Einstellungen v-else-if="nav.seite === 'einstellungen'" />
      </main>
      <StatusBar />
    </div>
    <Frage />
    <Toaster />
  </TooltipProvider>
</template>
