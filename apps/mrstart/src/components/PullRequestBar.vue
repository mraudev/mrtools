<script setup lang="ts">
import { ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { GitPullRequest, TriangleAlert } from "@lucide/vue";
import Tip from "./ui/Tip.vue";
import { api } from "@/lib/api";
import { store } from "@/lib/store";
import { toastError } from "@/lib/toast";
import type { PullRequest } from "@/lib/types";

const props = defineProps<{ paths: string[] }>();

const pulls = ref<PullRequest[]>([]);
const errors = ref<string[]>([]);
let request = 0;

async function load() {
  const current = ++request;
  const { githubToken, giteaHost, giteaToken } = store.config.settings;
  const result = props.paths.length
    ? await api
        .pullRequests({ paths: props.paths, githubToken, giteaHost, giteaToken })
        .catch((e) => ({ pulls: [], errors: [String(e)] }))
    : { pulls: [], errors: [] };
  // Ignore responses of requests that were superseded by a tab switch.
  if (current !== request) return;
  pulls.value = result.pulls;
  errors.value = result.errors;
}

watch(() => [props.paths.join("|"), store.refreshTick], load, { immediate: true });

function open(url: string) {
  openUrl(url).catch((e) => toastError("Link konnte nicht geöffnet werden", e));
}
</script>

<template>
  <div v-if="pulls.length || errors.length" class="mb-4 flex flex-wrap items-center gap-2">
    <Tip v-for="pr in pulls" :key="pr.url" :text="`${pr.repo} · ${pr.branch}\n${pr.url}`">
      <button
        class="inline-flex h-7 max-w-full items-center gap-2 rounded-full border border-accent/40 bg-accent/10 pr-3 pl-2.5 text-[13px] transition-colors hover:bg-accent/20"
        @click="open(pr.url)"
      >
        <GitPullRequest class="size-3.5 shrink-0 text-accent-text" />
        <span class="shrink-0 font-medium text-accent-text">{{ pr.repo }} #{{ pr.number }}</span>
        <span class="truncate">{{ pr.title }}</span>
      </button>
    </Tip>
    <Tip v-if="errors.length" :text="errors.join('\n')">
      <span class="inline-flex h-7 items-center gap-1.5 rounded-full px-2 text-xs text-muted-foreground">
        <TriangleAlert class="size-3.5 text-amber-500" />
        Pull Requests teilweise nicht abrufbar
      </span>
    </Tip>
  </div>
</template>
