<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { TriangleAlert } from "@lucide/vue";
import ThemeToggle from "@mrtools/ui/components/ThemeToggle";
import Tip from "@mrtools/ui/components/Tip";
import { budgetAllowsAuto, formatTime, RESERVE } from "@/lib/logic";
import { lights } from "@/lib/lights";
import { settings, state } from "@/lib/store";
import { ui } from "@/lib/ui";

const version = ref("");
onMounted(async () => (version.value = await getVersion()));

const quota = computed(() => state.quota);
const low = computed(() => !budgetAllowsAuto(quota.value));
</script>

<template>
  <footer class="flex h-7 shrink-0 items-center gap-3 border-t border-border bg-card px-3 text-xs text-muted-foreground">
    <template v-if="ui.tab === 'lights'">
      <span v-if="lights.hue.bridge" class="inline-flex items-center gap-1.5">
        <span class="size-1.5 rounded-full" :class="lights.hue.connected ? 'bg-emerald-500' : 'bg-amber-500'" />
        Hue Bridge {{ lights.hue.bridge.ip }}{{ lights.hue.connected ? "" : " – Verbindung unterbrochen" }}
      </span>
      <span>· {{ lights.govee.devices.length }} Govee-{{ lights.govee.devices.length === 1 ? "Gerät" : "Geräte" }}</span>
      <span>· alles lokal</span>
    </template>
    <template v-else-if="state.phase === 'ready'">
      <span v-if="settings.home">{{ settings.home.name }} ·</span>
      <Tip
        v-if="quota?.remaining != null"
        :text="`tado zählt jeden Zugriff. Unter ${RESERVE} übrigen Zugriffen aktualisiert mrhome nicht mehr von selbst – für Änderungen bleibt genug.`"
      >
        <span class="inline-flex items-center gap-1 tabular-nums" :class="low && 'font-medium text-amber-600 dark:text-amber-400'">
          <TriangleAlert v-if="low" class="size-3.5" />
          Heute noch {{ quota.remaining }}<template v-if="quota.limit"> von {{ quota.limit }}</template> Zugriffen
          <template v-if="quota.resetAt"> · neu um {{ formatTime(quota.resetAt) }}</template>
        </span>
      </Tip>
      <span v-if="state.lastUpdate" class="tabular-nums">· aktualisiert {{ formatTime(state.lastUpdate) }}</span>
    </template>
    <div class="flex-1" />
    <ThemeToggle />
    <span class="tabular-nums">v{{ version }}</span>
  </footer>
</template>
