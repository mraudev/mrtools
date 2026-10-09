<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Calculator, ExternalLink, FolderSearch, Info, LoaderCircle, Files } from "@lucide/vue";
import EntryIcon from "./EntryIcon.vue";
import Tip from "@mrtools/ui/components/Tip";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "@/lib/api";
import { openEntry, showInExplorer, showProperties } from "@/lib/actions";
import { previewOf, typeLabel } from "@/lib/fileTypes";
import { formatBytes, formatCount, formatDate } from "@/lib/format";
import { baseName } from "@/lib/paths";
import { selectedEntries, state } from "@/lib/store";
import type { Entry, FolderSize } from "@/lib/types";

/** The selected entry, or the current folder when nothing is selected. */
const entry = computed<Entry | undefined>(() => {
  if (selectedEntries.value.length === 1) return selectedEntries.value[0];
  if (selectedEntries.value.length || !state.path) return undefined;
  return {
    name: baseName(state.path),
    path: state.path,
    isDir: true,
    size: 0,
    modified: 0,
    created: 0,
    hidden: false,
    system: false,
    readonly: false,
    link: false,
  };
});
const isCurrentFolder = computed(() => entry.value?.path === state.path);
const preview = computed(() => (entry.value ? previewOf(entry.value) : null));
const src = computed(() => (entry.value && preview.value !== "text" ? convertFileSrc(entry.value.path) : ""));
const selectionSize = computed(() => selectedEntries.value.reduce((sum, e) => sum + e.size, 0));

const text = ref<string | null>(null);
const textLoading = ref(false);
const imageFailed = ref(false);
const size = ref<FolderSize | null>(null);
const sizing = ref(false);

const attributes = computed(() => {
  const e = entry.value;
  if (!e) return "";
  return [e.readonly && "schreibgeschützt", e.hidden && "versteckt", e.system && "System", e.link && "Verknüpfung"]
    .filter(Boolean)
    .join(", ");
});

// Text preview, read lazily – keyboard navigation through a folder should not read every file.
let timer: ReturnType<typeof setTimeout> | undefined;
watch(
  () => entry.value?.path,
  (path) => {
    clearTimeout(timer);
    text.value = null;
    textLoading.value = false;
    size.value = null;
    imageFailed.value = false;
    // Files larger than 50 MB are rarely text worth previewing.
    if (!path || preview.value !== "text" || entry.value!.size > 50 * 1024 ** 2) return;
    textLoading.value = true;
    timer = setTimeout(async () => {
      try {
        const result = await api.readText(path);
        if (entry.value?.path === path) text.value = result;
      } catch {
        // No preview – the properties are still shown.
      } finally {
        if (entry.value?.path === path) textLoading.value = false;
      }
    }, 120);
  },
  { immediate: true },
);

async function computeSize() {
  const path = entry.value?.path;
  if (!path) return;
  sizing.value = true;
  try {
    const result = await api.folderSize(path);
    if (entry.value?.path === path) size.value = result;
  } catch (e) {
    toastError("Größe konnte nicht berechnet werden", e);
  } finally {
    sizing.value = false;
  }
}
</script>

<template>
  <aside class="flex h-full min-w-0 flex-col overflow-y-auto bg-card">
    <div v-if="selectedEntries.length > 1" class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
      <Files class="size-12 text-accent-text" :stroke-width="1.25" />
      <p class="font-semibold">{{ formatCount(selectedEntries.length) }} Elemente ausgewählt</p>
      <p class="text-xs text-muted-foreground tabular-nums">
        {{ formatBytes(selectionSize) }}
        <template v-if="selectedEntries.some((e) => e.isDir)">ohne Ordnerinhalte</template>
      </p>
    </div>

    <template v-else-if="entry">
      <div class="grid min-h-44 shrink-0 place-items-center border-b border-border bg-background/60 p-3">
        <img
          v-if="preview === 'image' && !imageFailed"
          :src="src"
          alt=""
          class="max-h-72 max-w-full rounded object-contain shadow-sm"
          @error="imageFailed = true"
        />
        <video v-else-if="preview === 'video'" :src="src" controls class="max-h-72 max-w-full rounded" />
        <div v-else-if="preview === 'audio'" class="flex w-full flex-col items-center gap-3">
          <EntryIcon :entry="entry" class="size-14" :stroke-width="1.25" />
          <audio :src="src" controls class="w-full" />
        </div>
        <pre
          v-else-if="text !== null"
          class="max-h-80 w-full overflow-auto rounded-md border border-border bg-card p-2 font-mono text-[11px] leading-4 whitespace-pre select-text"
          >{{ text.slice(0, 20_000) }}</pre
        >
        <LoaderCircle v-else-if="textLoading" class="size-5 animate-spin text-muted-foreground" />
        <EntryIcon v-else :entry="entry" class="size-16" :stroke-width="1.25" />
      </div>

      <div class="flex flex-col gap-3 p-3">
        <div class="flex items-start gap-1">
          <p class="min-w-0 flex-1 font-semibold break-words select-text">{{ entry.name }}</p>
          <Tip v-if="!isCurrentFolder" text="Öffnen">
            <button class="icon-btn" aria-label="Öffnen" @click="openEntry(entry)"><ExternalLink /></button>
          </Tip>
          <Tip text="Im Explorer zeigen">
            <button class="icon-btn" aria-label="Im Explorer zeigen" @click="showInExplorer(entry.path)"><FolderSearch /></button>
          </Tip>
          <Tip text="Eigenschaften (Alt+Enter)">
            <button class="icon-btn" aria-label="Eigenschaften" @click="showProperties(entry.path)"><Info /></button>
          </Tip>
        </div>

        <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-xs [&>dd]:min-w-0 [&>dd]:break-words [&>dt]:text-muted-foreground">
          <dt>Typ</dt>
          <dd>{{ typeLabel(entry) }}</dd>
          <template v-if="!entry.isDir">
            <dt>Größe</dt>
            <dd class="tabular-nums">{{ formatBytes(entry.size) }} ({{ formatCount(entry.size) }} Bytes)</dd>
          </template>
          <template v-if="isCurrentFolder">
            <dt>Inhalt</dt>
            <dd class="tabular-nums">
              {{ formatCount(state.entries.filter((e) => e.isDir).length) }} Ordner,
              {{ formatCount(state.entries.filter((e) => !e.isDir).length) }} Dateien
            </dd>
          </template>
          <template v-if="entry.modified">
            <dt>Geändert</dt>
            <dd class="tabular-nums">{{ formatDate(entry.modified) }}</dd>
          </template>
          <template v-if="entry.created">
            <dt>Erstellt</dt>
            <dd class="tabular-nums">{{ formatDate(entry.created) }}</dd>
          </template>
          <template v-if="attributes">
            <dt>Attribute</dt>
            <dd>{{ attributes }}</dd>
          </template>
          <dt>Pfad</dt>
          <dd class="font-mono text-[11px] select-text">{{ entry.path }}</dd>
        </dl>

        <div v-if="entry.isDir" class="rounded-lg border border-border p-2.5 text-xs">
          <template v-if="size">
            <p class="font-semibold tabular-nums">{{ formatBytes(size.size) }}</p>
            <p class="text-muted-foreground tabular-nums">
              {{ formatCount(size.files) }} Dateien · {{ formatCount(size.dirs) }} Ordner
              <template v-if="size.errors"> · {{ formatCount(size.errors) }} nicht lesbar</template>
            </p>
          </template>
          <button v-else class="btn btn-outline h-7 w-full text-xs" :disabled="sizing" @click="computeSize">
            <LoaderCircle v-if="sizing" class="animate-spin" />
            <Calculator v-else />
            {{ sizing ? "Wird berechnet …" : "Ordnergröße berechnen" }}
          </button>
        </div>
      </div>
    </template>

    <div v-else class="flex flex-1 items-center justify-center p-6 text-center text-xs text-muted-foreground">
      Wähle einen Ordner oder ein Laufwerk.
    </div>
  </aside>
</template>
