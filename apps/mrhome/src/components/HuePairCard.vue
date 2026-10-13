<script setup lang="ts">
import { ref } from "vue";
import { LoaderCircle, Search } from "@lucide/vue";
import { discoverHue, lights, pairHue } from "@/lib/lights";

const manualIp = ref("");
</script>

<template>
  <section class="rounded-xl border border-border bg-card p-5">
    <h2 class="font-semibold">Philips Hue verbinden</h2>

    <div v-if="lights.hue.pairing" class="mt-3 flex items-start gap-3 rounded-lg bg-accent/10 p-3">
      <LoaderCircle class="mt-0.5 size-4 shrink-0 animate-spin text-accent-text" />
      <p>
        <b>Drück jetzt den runden Knopf oben auf der Hue Bridge</b> ({{ lights.hue.pairing }}). mrhome wartet bis zu einer
        Minute darauf.
      </p>
    </div>

    <template v-else>
      <p class="mt-1 text-muted-foreground">mrhome spricht direkt mit der Bridge in deinem Netz – ohne Cloud und ohne Konto.</p>

      <div class="mt-4 flex flex-col gap-2">
        <button
          v-for="bridge in lights.hue.found"
          :key="bridge.ip"
          class="flex items-center gap-3 rounded-lg border border-border px-3 py-2 text-left transition-colors hover:border-accent/60 hover:bg-accent/5"
          @click="pairHue(bridge.ip)"
        >
          <span class="min-w-0 flex-1">
            <span class="block font-medium">Hue Bridge</span>
            <span class="block font-mono text-xs text-muted-foreground">{{ bridge.ip }} · {{ bridge.id }}</span>
          </span>
          <span class="btn btn-primary h-7 text-xs">Verbinden</span>
        </button>
        <p v-if="!lights.hue.found.length && !lights.hue.searching" class="text-sm text-muted-foreground">
          Keine Bridge im Netz gefunden.
        </p>
      </div>

      <div class="mt-4 flex items-center gap-2">
        <button class="btn btn-outline h-8" :disabled="lights.hue.searching" @click="discoverHue">
          <LoaderCircle v-if="lights.hue.searching" class="animate-spin" />
          <Search v-else />
          {{ lights.hue.searching ? "Suche …" : "Erneut suchen" }}
        </button>
        <span class="text-xs text-muted-foreground">oder IP-Adresse:</span>
        <input v-model="manualIp" class="input h-8 w-36 font-mono text-xs" placeholder="192.168.1.20" @keydown.enter="manualIp && pairHue(manualIp)" />
        <button class="btn btn-ghost h-8" :disabled="!manualIp.trim()" @click="pairHue(manualIp)">Verbinden</button>
      </div>
    </template>
  </section>
</template>
