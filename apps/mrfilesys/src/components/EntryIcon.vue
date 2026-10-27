<script setup lang="ts">
import { computed, watch } from "vue";
import {
  AppWindow,
  ArrowUpRight,
  Check,
  CircleHelp,
  Minus,
  Pencil,
  Plus,
  TriangleAlert,
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
import type { GitStatus } from "@/lib/git";
import { iconKey, icons, requestIcon, type IconSize } from "@/lib/icons";
import type { Entry } from "@/lib/types";

/**
 * The Windows icon of a file or folder, as in Explorer. Until it has loaded (or if Windows has none)
 * a symbol for the file type is shown. The caller sets the size via `class`.
 */
const props = withDefaults(
  defineProps<{
    entry: Pick<Entry, "name" | "isDir" | "link"> & { path?: string };
    size?: IconSize;
    /** Git state shown as a badge, like TortoiseGit. */
    git?: GitStatus | null;
  }>(),
  { size: "small", git: null },
);

const gitBadges: Record<GitStatus, { icon: unknown; class: string; label: string }> = {
  clean: { icon: Check, class: "bg-emerald-500", label: "unverändert" },
  modified: { icon: Pencil, class: "bg-amber-500", label: "geändert" },
  staged: { icon: Plus, class: "bg-sky-500", label: "vorgemerkt" },
  untracked: { icon: CircleHelp, class: "bg-zinc-400 dark:bg-zinc-500", label: "nicht versioniert" },
  conflict: { icon: TriangleAlert, class: "bg-red-600", label: "Konflikt" },
  ignored: { icon: Minus, class: "bg-zinc-300 dark:bg-zinc-600", label: "ignoriert" },
};

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
    <!-- Git state, like TortoiseGit's overlays -->
    <span
      v-if="git"
      class="absolute -right-1 -bottom-1 grid size-[55%] min-h-2.5 min-w-2.5 place-items-center rounded-full text-white ring-2 ring-card"
      :class="gitBadges[git].class"
      :title="`Git: ${gitBadges[git].label}`"
    >
      <component :is="gitBadges[git].icon" class="size-[70%]" :stroke-width="3.5" />
    </span>
  </span>
</template>
