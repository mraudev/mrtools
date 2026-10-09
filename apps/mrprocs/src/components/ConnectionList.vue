<script setup lang="ts">
import { ref, watch } from "vue";
import { api } from "@/lib/api";
import { state } from "@/lib/store";
import type { Connection } from "@/lib/types";

const props = defineProps<{ pid: number }>();

const connections = ref<Connection[]>([]);

// Reloaded with every snapshot so the list stays live.
watch(
  () => [props.pid, state.tick],
  async () => {
    const pid = props.pid;
    const result = await api.connections(pid).catch(() => []);
    if (pid === props.pid) connections.value = result;
  },
  { immediate: true },
);
</script>

<template>
  <div class="overflow-y-auto">
    <table class="w-full table-fixed text-[13px]">
      <thead class="sticky top-0 bg-background text-xs text-muted-foreground">
        <tr class="[&>th]:py-1.5 [&>th]:text-left [&>th]:font-medium">
          <th class="w-14 pl-1">Proto.</th>
          <th>Lokal</th>
          <th>Remote</th>
          <th class="w-24">Status</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(c, i) in connections" :key="i" class="hover:bg-foreground/5 [&>td]:py-1">
          <td class="pl-1 text-xs text-muted-foreground">{{ c.protocol }}</td>
          <td class="truncate font-mono text-xs select-text" :title="c.local">{{ c.local }}</td>
          <td class="truncate font-mono text-xs select-text" :title="c.remote">{{ c.remote || "–" }}</td>
          <td class="truncate text-xs" :class="c.state === 'Verbunden' ? 'text-foreground' : 'text-muted-foreground'">
            {{ c.state || "–" }}
          </td>
        </tr>
      </tbody>
    </table>
    <p v-if="!connections.length" class="py-8 text-center text-muted-foreground">Keine offenen Verbindungen</p>
  </div>
</template>
