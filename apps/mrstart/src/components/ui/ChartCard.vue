<script setup lang="ts">
import { ref } from "vue";
import { ChartColumn, Table } from "@lucide/vue";

/** Chart container with title and a table view – the accessible twin of every chart. */
defineProps<{ title: string; subtitle?: string }>();
const showTable = ref(false);
</script>

<template>
  <figure class="flex min-w-0 flex-col rounded-xl border border-border bg-card p-4">
    <figcaption class="mb-3 flex items-start gap-2">
      <div class="min-w-0 flex-1">
        <p class="font-semibold">{{ title }}</p>
        <p v-if="subtitle" class="text-xs text-muted-foreground">{{ subtitle }}</p>
      </div>
      <button
        class="icon-btn size-6 [&_svg]:size-3.5"
        :aria-label="showTable ? 'Als Diagramm anzeigen' : 'Als Tabelle anzeigen'"
        :title="showTable ? 'Als Diagramm anzeigen' : 'Als Tabelle anzeigen'"
        @click="showTable = !showTable"
      >
        <ChartColumn v-if="showTable" />
        <Table v-else />
      </button>
    </figcaption>
    <div v-if="showTable" class="text-[13px]"><slot name="table" /></div>
    <div v-else class="flex flex-1 flex-col"><slot /></div>
  </figure>
</template>
