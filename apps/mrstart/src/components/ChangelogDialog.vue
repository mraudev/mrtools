<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { Sparkles } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { api } from "@/lib/api";
import { store } from "@/lib/store";

const open = ref(false);
const version = ref("");
const notes = ref("");

/** After an update, shows the release notes of the new version once. */
onMounted(async () => {
  version.value = await getVersion();
  const seen = store.config.settings.lastSeenVersion;
  store.config.settings.lastSeenVersion = version.value;
  // First start (nothing seen yet) or no change: nothing to show.
  if (!seen || seen === version.value) return;
  notes.value = await api.releaseNotes(version.value).catch(() => "");
  open.value = true;
});
</script>

<template>
  <Dialog v-model:open="open" :title="`Neu in mrstart ${version}`" size="md">
    <template #icon>
      <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-[18px]">
        <Sparkles />
      </div>
    </template>
    <div
      v-if="notes"
      class="max-h-[50vh] overflow-y-auto rounded-lg border border-border bg-background p-3 text-[13px] whitespace-pre-wrap select-text"
    >
      {{ notes }}
    </div>
    <p v-else class="text-muted-foreground">
      Die Änderungen dieser Version konnten nicht geladen werden – sie stehen im Release auf GitHub.
    </p>
    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-primary" @click="open = false">Los geht's</button>
    </template>
  </Dialog>
</template>
