<script setup lang="ts">
import { computed, ref, watch, type Component } from "vue";
import {
  AppWindow,
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
  Pencil,
  SquareTerminal,
  Terminal,
  TriangleAlert,
} from "@lucide/vue";
import Tip from "./ui/Tip.vue";
import { api } from "@/lib/api";
import { baseName, launch, openGitTool, openPath, runProjectCommand } from "@/lib/actions";
import { runGit } from "@/lib/gitConsole";
import { openInBrowser, pullErrorFor, pullFor } from "@/lib/pulls";
import { openProjectEditor, store } from "@/lib/store";
import type { BranchInfo, Project } from "@/lib/types";

const props = defineProps<{ project: Project; editable: boolean; showCategory?: boolean }>();

/** `null` = not a git repository. */
const info = ref<BranchInfo | null>(null);

watch(
  () => [props.project.path, store.refreshTick, store.config.settings.giteaHost],
  async () => {
    info.value = await api
      .branchInfo(props.project.path, store.config.settings.giteaHost)
      .catch(() => null);
  },
  { immediate: true },
);

/** `null` = not a git repository, `""` = detached HEAD. */
const branch = computed(() => info.value?.branch ?? null);
const createPullUrl = computed(() => info.value?.createPullUrl ?? null);
const pull = computed(() => pullFor(props.project.path));
const pullError = computed(() => pullErrorFor(props.project.path));

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
  <article class="group flex min-w-0 flex-col overflow-hidden rounded-xl border border-border bg-card shadow-sm transition-shadow hover:shadow-md">
    <div class="h-[3px] bg-linear-to-r from-accent via-accent/60 to-accent/20" />

    <header class="flex items-start gap-3 px-4 pt-3.5 pb-3">
      <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-[18px]">
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
      <Tip v-if="editable" text="Projekt bearbeiten">
        <button
          class="icon-btn -mt-0.5 -mr-1.5 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
          aria-label="Projekt bearbeiten"
          @click="openProjectEditor(project)"
        >
          <Pencil />
        </button>
      </Tip>
    </header>

    <div class="grid grid-cols-4 gap-1 px-3">
      <Tip v-for="q in quickActions" :key="q.label" :text="q.tooltip">
        <button
          class="flex flex-col items-center gap-1 rounded-lg py-2 text-[11px] text-muted-foreground transition-colors hover:bg-foreground/6 hover:text-foreground [&_svg]:size-[18px]"
          @click="q.run"
        >
          <component :is="q.icon" />
          {{ q.label }}
        </button>
      </Tip>
    </div>

    <div v-if="actions.length" class="grid grid-cols-2 gap-1.5 px-3 pt-2">
      <Tip v-for="a in actions" :key="a.key" :text="a.tooltip">
        <button
          class="flex h-8 min-w-0 items-center gap-2 rounded-md border border-border bg-foreground/[0.03] px-2.5 text-left text-[13px] transition-colors hover:border-accent/50 hover:bg-accent/10 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:text-accent-text"
          @click="a.run"
        >
          <component :is="a.icon" />
          <span class="truncate">{{ a.caption }}</span>
        </button>
      </Tip>
    </div>

    <div class="min-h-3 flex-1" />

    <footer
      v-if="branch !== null"
      class="flex items-center gap-1 border-t border-border bg-foreground/[0.02] py-1.5 pr-1.5 pl-3.5"
    >
      <GitBranch class="size-3.5 shrink-0 text-muted-foreground" />
      <Tip :text="branch || 'detached HEAD'">
        <span class="min-w-0 flex-1 truncate font-mono text-xs" :class="!branch && 'text-muted-foreground italic'">
          {{ branch || "detached HEAD" }}
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
        <button class="btn btn-ghost h-7 px-2 text-xs" @click="runGit(project, 'pull')"><CloudDownload />Pull</button>
      </Tip>
      <Tip text="Push: pull + push">
        <button class="btn btn-ghost h-7 px-2 text-xs" @click="runGit(project, 'push')"><CloudUpload />Push</button>
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
