<script setup lang="ts">
import { computed, ref, watch, type Component } from "vue";
import {
  AppWindow,
  ArrowDown,
  ArrowUp,
  CircleCheck,
  CloudOff,
  Boxes,
  ClipboardCheck,
  CloudDownload,
  CloudUpload,
  Code,
  Command,
  Eye,
  FileCode,
  Folder,
  FolderGit2,
  FolderOpen,
  GitBranch,
  GitFork,
  GitPullRequest,
  GitPullRequestCreate,
  History,
  FilePen,
  Pencil,
  SquareTerminal,
  Star,
  Terminal,
  TriangleAlert,
} from "@lucide/vue";
import Tip from "./ui/Tip.vue";
import { api } from "@/lib/api";
import { baseName, launch, openGitTool, openPath, runProjectCommand } from "@/lib/actions";
import { runGit } from "@/lib/gitConsole";
import { openInBrowser, pullErrorFor, pullFor } from "@/lib/pulls";
import { ago, dateTime } from "@/lib/time";
import { isPinned, openProjectEditor, store, togglePin } from "@/lib/store";
import { toast, toastError } from "@/lib/toast";
import type { BranchInfo, Project } from "@/lib/types";

const props = defineProps<{
  project: Project;
  editable: boolean;
  showCategory?: boolean;
  /** Keyboard selection (arrow keys / Enter in the filter). */
  selected?: boolean;
}>();

const compact = computed(() => store.config.settings.compactTiles);
const pinned = computed(() => isPinned(props.project));

/** Copies the fix (e.g. the safe.directory command) or the whole message. */
async function copyGitError() {
  const command = gitError.value.split("\n").find((line) => line.includes("git config"));
  const text = command ? command.replace(/^.*?(git config)/, "$1") : gitError.value;
  try {
    await navigator.clipboard.writeText(text);
    toast("success", "In die Zwischenablage kopiert", text);
  } catch (e) {
    toastError("Kopieren fehlgeschlagen", e);
  }
}

/** `null` = not a git repository. */
const info = ref<BranchInfo | null>(null);
/** Git cannot be used here (not installed, dubious ownership …). */
const gitError = ref("");

watch(
  () => [props.project.path, store.refreshTick, store.config.settings.giteaHost],
  async () => {
    try {
      info.value = await api.branchInfo(props.project.path, store.config.settings.giteaHost);
      gitError.value = "";
    } catch (e) {
      info.value = null;
      gitError.value = String(e);
    }
  },
  { immediate: true },
);

/** `null` = not a git repository, `""` = detached HEAD. */
const branch = computed(() => info.value?.branch ?? null);
const createPullUrl = computed(() => info.value?.createPullUrl ?? null);
const pull = computed(() => pullFor(props.project.path));
const pullError = computed(() => pullErrorFor(props.project.path));

/** Local HEAD vs. upstream branch, as of the last fetch. */
const sync = computed(() => {
  const i = info.value;
  if (!i?.branch) return null;
  const fetched = i.fetchedAt ? `Zuletzt geholt ${ago(i.fetchedAt)} (${dateTime(i.fetchedAt)}).` : "Noch nie geholt.";
  if (!i.upstream) {
    return { kind: "none" as const, tip: "Kein Remote-Branch verknüpft – noch nicht gepusht?" };
  }
  const { name, ahead, behind } = i.upstream;
  const parts = [
    ahead ? `${ahead} lokale${ahead === 1 ? "r Commit" : " Commits"} noch nicht gepusht` : "",
    behind ? `${behind} Commit${behind === 1 ? "" : "s"} auf ${name} noch nicht geholt (Pull)` : "",
  ].filter(Boolean);
  return {
    kind: ahead || behind ? ("diverged" as const) : ("same" as const),
    ahead,
    behind,
    tip: `${parts.length ? parts.join("\n") : `Stimmt mit ${name} überein.`}\nVergleich mit dem zuletzt geholten Stand. ${fetched}`,
  };
});

const fork = computed(() => store.config.settings.gitTool === "fork");

function appIcon(path: string): Component {
  const ext = path.slice(path.lastIndexOf(".")).toLowerCase();
  if (ext === ".exe") return AppWindow;
  if ([".bat", ".cmd", ".ps1", ".sh"].includes(ext)) return SquareTerminal;
  if ([".sln", ".slnx", ".code-workspace"].includes(ext)) return Code;
  if (ext === ".groupproj") return Boxes;
  return FileCode;
}

interface Action {
  key: string;
  caption: string;
  tooltip: string;
  icon: Component;
  run: () => void;
}

const actions = computed<Action[]>(() => [
  ...props.project.apps.map((app) => ({
    key: `app:${app}`,
    caption: baseName(app),
    tooltip: app,
    icon: appIcon(app),
    run: () => openPath(app),
  })),
  ...props.project.commands.map((c, i) => ({
    key: `cmd:${i}`,
    caption: c.caption || c.command,
    tooltip: c.command,
    icon: Command,
    run: () => runProjectCommand(c.command, props.project),
  })),
]);

const quickActions = [
  { label: "Editor", tooltip: "Im Editor öffnen", icon: Code, run: () => launch("editor", props.project) },
  { label: "Explorer", tooltip: "Im Explorer öffnen", icon: FolderOpen, run: () => openPath(props.project.path) },
  { label: "Bash", tooltip: "Git Bash hier öffnen", icon: SquareTerminal, run: () => launch("bash", props.project) },
  { label: "Terminal", tooltip: "Terminal hier öffnen", icon: Terminal, run: () => launch("terminal", props.project) },
];
</script>

<template>
  <article
    class="group flex min-w-0 flex-col overflow-hidden rounded-xl border border-border bg-card shadow-sm transition-shadow hover:shadow-md"
    :class="selected && 'ring-2 ring-accent'"
  >
    <div class="h-[3px] bg-linear-to-r from-accent via-accent/60 to-accent/20" />

    <header class="flex items-start gap-3 px-4" :class="compact ? 'pt-2.5 pb-2' : 'pt-3.5 pb-3'">
      <div
        v-if="!compact"
        class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-[18px]"
      >
        <FolderGit2 v-if="branch !== null" />
        <Folder v-else />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-2">
          <h3 class="truncate text-[15px] leading-5 font-semibold">{{ project.name }}</h3>
          <Tip v-if="!editable" text="Aus einem überwachten Ordner">
            <Eye class="size-3.5 shrink-0 text-muted-foreground" />
          </Tip>
          <span
            v-if="project.version"
            class="shrink-0 rounded bg-foreground/8 px-1.5 text-xs leading-5 text-muted-foreground tabular-nums"
          >
            {{ project.version }}
          </span>
          <span
            v-if="showCategory && project.category"
            class="shrink-0 rounded border border-border px-1.5 text-xs leading-[18px] text-muted-foreground capitalize"
          >
            {{ project.category }}
          </span>
        </div>
        <Tip :text="project.path">
          <p class="mt-0.5 truncate font-mono text-xs text-muted-foreground">{{ project.path }}</p>
        </Tip>
      </div>
      <div class="-mt-0.5 -mr-1.5 flex">
        <Tip :text="pinned ? 'Favorit entfernen' : 'Als Favorit oben anheften'">
          <button
            class="icon-btn"
            :class="!pinned && 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'"
            :aria-label="pinned ? 'Favorit entfernen' : 'Als Favorit anheften'"
            :aria-pressed="pinned"
            @click="togglePin(project)"
          >
            <Star :class="pinned && 'fill-current text-accent-text'" />
          </button>
        </Tip>
        <Tip v-if="editable" text="Projekt bearbeiten">
          <button
            class="icon-btn opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
            aria-label="Projekt bearbeiten"
            @click="openProjectEditor(project)"
          >
            <Pencil />
          </button>
        </Tip>
      </div>
    </header>

    <div class="grid grid-cols-4 gap-1 px-3">
      <Tip v-for="q in quickActions" :key="q.label" :text="q.tooltip">
        <button
          class="flex flex-col items-center gap-1 rounded-lg text-[11px] text-muted-foreground transition-colors hover:bg-foreground/6 hover:text-foreground [&_svg]:size-[18px]"
          :class="compact ? 'py-1.5' : 'py-2'"
          :aria-label="q.tooltip"
          @click="q.run"
        >
          <component :is="q.icon" />
          <template v-if="!compact">{{ q.label }}</template>
        </button>
      </Tip>
    </div>

    <div
      v-if="actions.length"
      class="px-3 pt-2"
      :class="compact ? 'flex flex-wrap gap-1' : 'grid grid-cols-2 gap-1.5'"
    >
      <Tip v-for="a in actions" :key="a.key" :text="a.tooltip">
        <button
          class="flex min-w-0 items-center gap-2 rounded-md border border-border bg-foreground/[0.03] text-left transition-colors hover:border-accent/50 hover:bg-accent/10 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:text-accent-text"
          :class="compact ? 'h-7 max-w-full px-2 text-xs' : 'h-8 px-2.5 text-[13px]'"
          @click="a.run"
        >
          <component :is="a.icon" />
          <span class="truncate">{{ a.caption }}</span>
        </button>
      </Tip>
    </div>

    <div class="flex-1" :class="compact ? 'min-h-2' : 'min-h-3'" />

    <Tip v-if="gitError" :text="`${gitError}\n\nKlicken zum Kopieren.`">
      <button
        class="flex w-full items-center gap-2 border-t border-border bg-red-500/5 py-2 pr-2 pl-3.5 text-left text-xs text-red-600 hover:bg-red-500/10 dark:text-red-400"
        @click="copyGitError"
      >
        <TriangleAlert class="size-3.5 shrink-0" />
        <span class="min-w-0 flex-1 truncate">{{ gitError.split("\n")[0] }}</span>
      </button>
    </Tip>

    <footer
      v-else-if="branch !== null"
      class="flex items-center gap-1 border-t border-border bg-foreground/[0.02] py-1.5 pr-1.5 pl-3.5"
    >
      <GitBranch class="size-3.5 shrink-0 text-muted-foreground" />
      <Tip :text="branch || 'detached HEAD'">
        <span class="min-w-0 flex-1 truncate font-mono text-xs" :class="!branch && 'text-muted-foreground italic'">
          {{ branch || "detached HEAD" }}
        </span>
      </Tip>
      <Tip
        v-if="info && info.changes > 0"
        :text="`${info.changes} geänderte ${info.changes === 1 ? 'Datei' : 'Dateien'} – noch nicht committet`"
      >
        <span class="inline-flex h-7 shrink-0 items-center gap-0.5 px-1 text-xs tabular-nums" tabindex="0">
          <FilePen class="size-3.5" :style="{ color: 'var(--status-warning)' }" />{{ info.changes }}
        </span>
      </Tip>
      <Tip v-if="sync" :text="sync.tip">
        <span class="inline-flex h-7 shrink-0 items-center gap-1 px-1 text-xs tabular-nums" tabindex="0">
          <template v-if="sync.kind === 'same'">
            <CircleCheck class="size-3.5" :style="{ color: 'var(--status-good)' }" />
          </template>
          <template v-else-if="sync.kind === 'none'">
            <CloudOff class="size-3.5 text-muted-foreground" />
          </template>
          <template v-else>
            <span v-if="sync.ahead" class="inline-flex items-center">
              <ArrowUp class="size-3.5" :style="{ color: 'var(--status-warning)' }" />{{ sync.ahead }}
            </span>
            <span v-if="sync.behind" class="inline-flex items-center">
              <ArrowDown class="size-3.5" :style="{ color: 'var(--status-warning)' }" />{{ sync.behind }}
            </span>
          </template>
        </span>
      </Tip>
      <Tip v-if="pull" :text="`Pull Request #${pull.number} im Browser öffnen\n${pull.title}`">
        <button class="btn btn-ghost h-7 px-2 text-xs text-accent-text" @click="pull && openInBrowser(pull.url)">
          <GitPullRequest />#{{ pull.number }}
        </button>
      </Tip>
      <template v-else>
        <Tip v-if="pullError" :text="`Pull Requests nicht abrufbar:\n${pullError}`">
          <span class="grid size-7 shrink-0 place-items-center text-amber-500 [&_svg]:size-4">
            <TriangleAlert />
          </span>
        </Tip>
        <Tip v-if="createPullUrl" text="Pull Request im Browser erstellen">
          <button
            class="icon-btn"
            aria-label="Pull Request erstellen"
            @click="createPullUrl && openInBrowser(createPullUrl)"
          >
            <GitPullRequestCreate />
          </button>
        </Tip>
      </template>
      <Tip text="Pull: fetch + rebase (autostash)">
        <button class="icon-btn" aria-label="Pull" @click="runGit(project, 'pull')"><CloudDownload /></button>
      </Tip>
      <Tip text="Push: pull + push">
        <button class="icon-btn" aria-label="Push" @click="runGit(project, 'push')"><CloudUpload /></button>
      </Tip>
      <Tip v-if="fork" text="In Fork öffnen">
        <button class="icon-btn" aria-label="In Fork öffnen" @click="openGitTool(project, 'status')"><GitFork /></button>
      </Tip>
      <template v-else>
        <Tip text="Änderungen (TortoiseGit)">
          <button class="icon-btn" aria-label="Änderungen" @click="openGitTool(project, 'status')"><ClipboardCheck /></button>
        </Tip>
        <Tip text="Log (TortoiseGit)">
          <button class="icon-btn" aria-label="Log" @click="openGitTool(project, 'log')"><History /></button>
        </Tip>
      </template>
    </footer>
  </article>
</template>
