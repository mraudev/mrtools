<script setup lang="ts">
import { computed, watch } from "vue";
import {
  AppWindow,
  ArrowUpRight,
  File,
  FileArchive,
  FileCode,
  FileImage,
  FileMusic,
  FileSpreadsheet,
  FileText,
  FileVideoCamera,
  Folder,
} from "@lucide/vue";
import { kindOf } from "@/lib/fileTypes";
import { iconKey, icons, requestIcon, type IconSize } from "@/lib/icons";
import type { Entry } from "@/lib/types";

/**
 * The Windows icon of a file or folder, as in Explorer. Until it has loaded (or if Windows has none)
 * a symbol for the file type is shown. The caller sets the size via `class`.
 */
const props = withDefaults(
  defineProps<{ entry: Pick<Entry, "name" | "isDir" | "link"> & { path?: string }; size?: IconSize }>(),
  { size: "small" },
);

const key = computed(() => iconKey(props.entry));
watch([key, () => props.size], ([k, size]) => requestIcon(k, size), { immediate: true });
const src = computed(() => icons.get(`${props.size}|${key.value}`) ?? null);

const kind = computed(() => kindOf(props.entry));
const fallback = computed(
  () =>
    ({
      folder: Folder,
      image: FileImage,
      video: FileVideoCamera,
      audio: FileMusic,
      archive: FileArchive,
      code: FileCode,
      text: FileText,
      table: FileSpreadsheet,
      pdf: FileText,
      program: AppWindow,
      file: File,
    })[kind.value],
);
const color = computed(
  () =>
    ({
      folder: "text-accent-text",
      image: "text-emerald-500",
      video: "text-violet-500",
      audio: "text-pink-500",
      archive: "text-amber-600 dark:text-amber-400",
      code: "text-sky-500",
      text: "text-muted-foreground",
      table: "text-green-600 dark:text-green-500",
      pdf: "text-red-500",
      program: "text-blue-500",
      file: "text-muted-foreground",
    })[kind.value],
);
</script>

<template>
  <span class="relative inline-grid shrink-0 place-items-center">
    <img v-if="src" :src="src" alt="" class="size-full object-contain" draggable="false" />
    <component
      :is="fallback"
      v-else
      class="size-full"
      :class="color"
      :fill="kind === 'folder' ? 'currentColor' : 'none'"
      :fill-opacity="0.18"
    />
    <!-- Symbolic links and junctions, like Explorer's shortcut arrow -->
    <span
      v-if="entry.link"
      class="absolute -bottom-0.5 -left-0.5 grid size-[45%] min-h-2.5 min-w-2.5 place-items-center rounded-sm bg-white text-blue-600 shadow-sm"
    >
      <ArrowUpRight class="size-[85%]" :stroke-width="3" />
    </span>
  </span>
</template>
