<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { ArrowRight, Download, LoaderCircle } from "@lucide/vue";
import Dialog from "./ui/Dialog.vue";
import { installUpdate, updater } from "@/lib/updater";

const currentVersion = ref("");
onMounted(async () => (currentVersion.value = await getVersion()));

const busy = computed(() => updater.status === "downloading" || updater.status === "installing");
const percent = computed(() =>
  updater.total > 0 ? Math.min(100, Math.round((updater.downloaded / updater.total) * 100)) : 0,
);
const megabytes = (bytes: number) => (bytes / 1024 / 1024).toFixed(1);
</script>

<template>
  <Dialog v-model:open="updater.dialogOpen" title="Update verfügbar" size="md" :persistent="busy">
    <template #icon>
      <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-[18px]">
        <Download />
      </div>
    </template>

    <div class="flex items-center justify-center gap-4 rounded-lg border border-border bg-background py-4">
      <div class="text-center">
        <p class="text-xs text-muted-foreground">installiert</p>
        <p class="font-mono text-lg">{{ currentVersion }}</p>
      </div>
      <ArrowRight class="size-5 text-muted-foreground" />
      <div class="text-center">
        <p class="text-xs text-muted-foreground">neu</p>
        <p class="font-mono text-lg font-semibold text-accent-text">{{ updater.version }}</p>
      </div>
    </div>

    <div v-if="updater.notes" class="mt-4">
      <p class="label">Änderungen</p>
      <div class="max-h-48 overflow-y-auto rounded-lg border border-border bg-background p-3 text-[13px] whitespace-pre-wrap select-text">
        {{ updater.notes }}
      </div>
    </div>

    <div v-if="busy" class="mt-4">
      <div class="h-2 overflow-hidden rounded-full bg-foreground/8">
        <div class="h-full rounded-full bg-accent transition-[width]" :style="{ width: `${percent}%` }" />
      </div>
      <p class="mt-1.5 text-xs text-muted-foreground">
        <template v-if="updater.status === 'installing'">Installiere – mrstart startet danach neu …</template>
        <template v-else-if="updater.total">
          {{ megabytes(updater.downloaded) }} / {{ megabytes(updater.total) }} MB ({{ percent }} %)
        </template>
        <template v-else>Lade herunter …</template>
      </p>
    </div>

    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-ghost" :disabled="busy" @click="updater.dialogOpen = false">Später</button>
      <button class="btn btn-primary" :disabled="busy" @click="installUpdate">
        <LoaderCircle v-if="busy" class="animate-spin" />
        <Download v-else />
        Installieren &amp; neu starten
      </button>
    </template>
  </Dialog>
</template>
