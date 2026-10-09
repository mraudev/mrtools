<script setup lang="ts">
import Dialog from "@mrtools/ui/components/Dialog";
import { confirmUninstall, uninstallApp } from "@/lib/actions";
</script>

<template>
  <Dialog
    v-model:open="confirmUninstall.open"
    :title="confirmUninstall.app ? `${confirmUninstall.app.name} deinstallieren?` : ''"
    :description="confirmUninstall.app?.installed?.version ? `Version ${confirmUninstall.app.installed.version}` : undefined"
    size="sm"
  >
    <p class="text-[13px]">
      Das Programm wird entfernt; eine laufende Instanz wird dafür beendet. Die Einstellungen der App bleiben
      erhalten und werden bei einer erneuten Installation wieder verwendet.
    </p>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-outline" @click="confirmUninstall.open = false">Abbrechen</button>
      <button class="btn bg-red-600 text-white hover:bg-red-600/85" autofocus @click="uninstallApp">
        Deinstallieren
      </button>
    </template>
  </Dialog>
</template>
