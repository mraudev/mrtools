<script setup lang="ts">
import { computed } from "vue";
import {
  AppWindow,
  File,
  FileArchive,
  FileCode,
  FileImage,
  FileMusic,
  FileSpreadsheet,
  FileSymlink,
  FileText,
  FileVideoCamera,
  Folder,
  FolderSymlink,
} from "@lucide/vue";
import { kindOf } from "@/lib/fileTypes";
import type { Entry } from "@/lib/types";

const props = defineProps<{ entry: Pick<Entry, "name" | "isDir" | "link"> }>();

const kind = computed(() => kindOf(props.entry));
const icon = computed(() => {
  if (props.entry.link) return props.entry.isDir ? FolderSymlink : FileSymlink;
  return {
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
  }[kind.value];
});
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
  <component :is="icon" class="shrink-0" :class="color" :fill="kind === 'folder' ? 'currentColor' : 'none'" :fill-opacity="0.18" />
</template>
