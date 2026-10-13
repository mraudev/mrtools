<script setup lang="ts">
import { ExternalLink, LoaderCircle, Thermometer } from "@lucide/vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { startLogin, state } from "@/lib/store";
</script>

<template>
  <div class="flex h-full items-center justify-center p-6">
    <div class="w-full max-w-md rounded-xl border border-border bg-card p-6 text-center shadow-sm">
      <div class="mx-auto grid size-12 place-items-center rounded-xl bg-accent/15 text-accent-text">
        <Thermometer class="size-6" />
      </div>
      <h1 class="mt-4 text-lg font-semibold">Mit tado° verbinden</h1>

      <template v-if="state.login">
        <p class="mt-2 text-muted-foreground">
          Im Browser hat sich die Anmeldeseite von tado geöffnet. Melde dich dort an und bestätige diesen Code:
        </p>
        <p class="mt-4 font-mono text-3xl font-semibold tracking-[0.2em] select-text">{{ state.login.userCode }}</p>
        <button class="btn btn-ghost mt-3 text-xs" @click="openUrl(state.login.verificationUri)">
          <ExternalLink />Seite erneut öffnen
        </button>
        <p class="mt-4 inline-flex items-center gap-2 text-xs text-muted-foreground">
          <LoaderCircle class="size-3.5 animate-spin" />Warte auf die Bestätigung …
        </p>
      </template>

      <template v-else>
        <p class="mt-2 text-muted-foreground">
          mrhome steuert deine tado°-X-Thermostate über die tado-Cloud. Die Anmeldung läuft im Browser bei tado –
          mrhome sieht dein Passwort nie.
        </p>
        <button class="btn btn-primary mt-5 w-full" :disabled="state.loggingIn" @click="startLogin">
          <LoaderCircle v-if="state.loggingIn" class="animate-spin" />
          Bei tado anmelden
        </button>
        <p class="mt-4 text-xs text-muted-foreground">
          Ohne Auto-Assist erlaubt tado 100 Zugriffe pro Tag. mrhome geht sparsam damit um und zeigt unten an,
          wie viele noch übrig sind.
        </p>
      </template>
    </div>
  </div>
</template>
