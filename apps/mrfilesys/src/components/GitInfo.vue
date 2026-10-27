<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { GitBranch, GitCommitHorizontal, LoaderCircle } from "@lucide/vue";
import EntryIcon from "./EntryIcon.vue";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { formatCount, formatDate } from "@/lib/format";
import { changesBelow, git, lastCommit, relative, statusOf, type Commit, type GitStatus } from "@/lib/git";
import { parentPath } from "@/lib/paths";
import { navigate, settings } from "@/lib/store";
import type { Entry } from "@/lib/types";

/** Git section of the detail panel for one file or folder. */
const props = defineProps<{ entry: Entry }>();

const SHOWN = 8;

const LABELS: Record<GitStatus, string> = {
  clean: "unverändert",
  modified: "geändert",
  staged: "vorgemerkt (git add)",
  untracked: "nicht versioniert",
  conflict: "Konflikt",
  ignored: "ignoriert (.gitignore)",
};

const repo = computed(() => (settings.git.enabled ? git.repo : null));
const rel = computed(() => (repo.value ? relative(props.entry.path, repo.value.root) : null));
const status = computed(() => (repo.value && rel.value !== null ? statusOf(repo.value, props.entry, true) : null));
const changes = computed(() => (repo.value && rel.value !== null && props.entry.isDir ? changesBelow(repo.value, rel.value) : []));

const commit = ref<Commit | null>(null);
const loading = ref(false);

// Read lazily – moving through a folder with the keyboard should not start git for every entry.
let timer: ReturnType<typeof setTimeout> | undefined;
watch(
  () => [props.entry.path, repo.value?.root, status.value] as const,
  ([path, root, s]) => {
    clearTimeout(timer);
    commit.value = null;
    loading.value = false;
    if (!root || rel.value === null || s === "untracked" || s === "ignored") return;
    loading.value = true;
    timer = setTimeout(async () => {
      try {
        const result = await lastCommit(path);
        if (props.entry.path === path) commit.value = result;
      } catch {
        // No commit info – the rest of the section still shows.
      } finally {
        if (props.entry.path === path) loading.value = false;
      }
    }, 200);
  },
  { immediate: true },
);

/** Absolute Windows path of a change. */
const absolute = (path: string) => `${repo.value!.root.replace(/\\$/, "")}\\${path.replace(/\/$/, "").replace(/\//g, "\\")}`;

/** Shown relative to the selected folder. */
function shortPath(path: string) {
  const prefix = rel.value ? `${rel.value}/` : "";
  return path.slice(prefix.length);
}

async function copyHash(hash: string) {
  try {
    await navigator.clipboard.writeText(hash);
    toast("success", "Commit-Hash kopiert", hash);
  } catch (e) {
    toastError("Kopieren fehlgeschlagen", e);
  }
}

function reveal(path: string) {
  const target = absolute(path);
  navigate(parentPath(target), { select: target });
}
</script>

<template>
  <section v-if="repo && rel !== null" class="rounded-lg border border-border p-2.5 text-xs">
    <h3 class="mb-2 flex items-center gap-1.5 font-semibold">
      <GitBranch class="size-3.5 text-accent-text" />
      Git
      <span class="ml-auto truncate font-normal text-muted-foreground" :title="repo.upstream ?? undefined">
        {{ repo.branch ?? "losgelöster HEAD" }}
        <template v-if="repo.ahead"> ↑{{ repo.ahead }}</template>
        <template v-if="repo.behind"> ↓{{ repo.behind }}</template>
      </span>
    </h3>

    <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 [&>dd]:min-w-0 [&>dd]:break-words [&>dt]:text-muted-foreground">
      <template v-if="status">
        <dt>Zustand</dt>
        <dd class="flex items-center gap-1.5">
          <EntryIcon :entry="{ name: entry.name, isDir: entry.isDir, link: false }" :git="status" class="size-4" />
          {{ LABELS[status] }}
        </dd>
      </template>

      <dt>Letzter Commit</dt>
      <dd>
        <span v-if="loading" class="inline-flex items-center gap-1.5 text-muted-foreground">
          <LoaderCircle class="size-3 animate-spin" />wird gelesen …
        </span>
        <template v-else-if="commit">
          <p class="font-medium select-text">{{ commit.subject }}</p>
          <p class="mt-0.5 text-muted-foreground">
            <button
              class="inline-flex items-center gap-1 font-mono hover:text-foreground"
              title="Commit-Hash kopieren"
              @click="copyHash(commit.hash)"
            >
              <GitCommitHorizontal class="size-3" />{{ commit.short }}
            </button>
            · {{ commit.author }} · {{ formatDate(Date.parse(commit.date)) }}
          </p>
        </template>
        <span v-else class="text-muted-foreground">
          {{ status === "untracked" ? "noch nie committet" : status === "ignored" ? "–" : "keiner" }}
        </span>
      </dd>

      <dt>Repository</dt>
      <dd class="font-mono text-[11px] select-text">{{ repo.root }}</dd>
    </dl>

    <template v-if="entry.isDir">
      <p class="mt-2.5 mb-1 text-muted-foreground">
        <template v-if="changes.length">{{ formatCount(changes.length) }} {{ changes.length === 1 ? "Änderung" : "Änderungen" }} in diesem Ordner</template>
        <template v-else>Keine Änderungen in diesem Ordner</template>
      </p>
      <ul v-if="changes.length" class="flex flex-col">
        <li v-for="change in changes.slice(0, SHOWN)" :key="change.path">
          <button
            class="flex w-full min-w-0 items-center gap-1.5 rounded px-1 py-0.5 text-left hover:bg-foreground/5"
            :title="`${LABELS[change.status]} – im Ordner zeigen`"
            @click="reveal(change.path)"
          >
            <EntryIcon
              :entry="{ name: change.path, isDir: change.path.endsWith('/'), link: false }"
              :git="change.status"
              class="size-3.5"
            />
            <span class="truncate font-mono text-[11px]">{{ shortPath(change.path) }}</span>
          </button>
        </li>
        <li v-if="changes.length > SHOWN" class="px-1 text-muted-foreground">… und {{ formatCount(changes.length - SHOWN) }} weitere</li>
      </ul>
    </template>
  </section>
</template>
