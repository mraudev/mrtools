<script setup lang="ts">
import { computed } from "vue";
import {
  CircleAlert,
  CircleArrowUp,
  CircleCheck,
  CircleDashed,
  Folder,
  FolderOpen,
  LoaderCircle,
  Play,
  Tag,
} from "@lucide/vue";
import GithubIcon from "./ui/GithubIcon.vue";
import Tip from "@mrtools/ui/components/Tip";
import { launchApp, openFolder, openGithub } from "@/lib/actions";
import { releaseKey, state } from "@/lib/store";
import type { App } from "@/lib/types";
import { appStatus, cleanVersion, newest } from "@/lib/version";

const props = defineProps<{ app: App }>();

const release = computed(() => (props.app.github ? state.releases[releaseKey(props.app)] : null));
const releaseLoading = computed(() => !!props.app.github && release.value === undefined);
const releaseError = computed(() => (release.value && "error" in release.value ? release.value.error : null));
const releaseInfo = computed(() => (release.value && "tag_name" in release.value ? release.value : null));
const releaseVersion = computed(() => {
  const tag = releaseInfo.value?.tag_name;
  if (!tag) return null;
  const prefix = props.app.tagPrefix;
  return prefix && tag.startsWith(prefix) ? tag.slice(prefix.length) : cleanVersion(tag);
});
const releaseTip = computed(() => {
  const date = releaseInfo.value?.published_at;
  return `Release auf GitHub öffnen${date ? `\nVeröffentlicht am ${new Date(date).toLocaleDateString("de-DE")}` : ""}`;
});

/** `undefined` = not installed, `null` = installed without a version. */
const installedVersion = computed(() => (props.app.installed ? props.app.installed.version : undefined));
const status = computed(() => appStatus(installedVersion.value, newest(releaseVersion.value, props.app.version)));
const exe = computed(() => props.app.installed?.exe ?? null);

const statusView = computed(() => {
  const s = status.value;
  if (s.kind === "missing") {
    return {
      label: "Nicht installiert",
      icon: CircleDashed,
      color: "var(--status-neutral)",
      tip: "Kein Eintrag unter den installierten Apps gefunden.",
    };
  }
  if (s.kind === "outdated") {
    return {
      label: `Update auf ${s.latest} verfügbar`,
      icon: CircleArrowUp,
      color: "var(--status-warning)",
      tip: `Installiert ist ${installedVersion.value}, aktuell ist ${s.latest}.`,
    };
  }
  return {
    label: "Aktuell",
    icon: CircleCheck,
    color: "var(--status-good)",
    tip: s.latest ? `Installiert ist die aktuelle Version ${s.latest}.` : "Installiert – keine neuere Version bekannt.",
  };
});

const githubUrl = computed(() => {
  const { github, repoPath } = props.app;
  if (!github) return null;
  return `https://github.com/${github}${repoPath ? `/tree/HEAD/${repoPath}` : ""}`;
});
</script>

<template>
  <article
    class="flex min-w-0 flex-col overflow-hidden rounded-xl border border-border bg-card shadow-sm transition-shadow hover:shadow-md"
  >
    <div class="h-[3px] bg-linear-to-r from-accent via-accent/60 to-accent/20" />

    <header class="flex items-start gap-3 px-4 pt-3.5 pb-3">
      <img v-if="app.icon" :src="app.icon" alt="" class="size-9 shrink-0 rounded-lg" />
      <div
        v-else
        class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-[18px]"
      >
        <Folder />
      </div>
      <div class="min-w-0 flex-1">
        <h3 class="truncate text-[15px] leading-5 font-semibold">{{ app.name }}</h3>
        <Tip :text="app.description ?? undefined">
          <p class="mt-0.5 truncate text-xs text-muted-foreground">{{ app.description || app.folder }}</p>
        </Tip>
      </div>
      <div class="-mt-0.5 -mr-1.5 flex">
        <Tip v-if="githubUrl" :text="`Auf GitHub öffnen\n${app.github}${app.repoPath ? `/${app.repoPath}` : ''}`">
          <button class="icon-btn" aria-label="Auf GitHub öffnen" @click="openGithub(githubUrl!)">
            <GithubIcon />
          </button>
        </Tip>
        <Tip :text="`Ordner öffnen\n${app.path}`">
          <button class="icon-btn" aria-label="Ordner öffnen" @click="openFolder(app.path)">
            <FolderOpen />
          </button>
        </Tip>
      </div>
    </header>

    <dl class="mx-4 grid grid-cols-3 divide-x divide-border rounded-lg border border-border bg-foreground/[0.02] text-center">
      <div class="px-2 py-1.5">
        <dt class="text-[11px] text-muted-foreground">Installiert</dt>
        <dd class="h-5 truncate text-[13px] font-medium tabular-nums">
          <template v-if="installedVersion">{{ installedVersion }}</template>
          <span v-else class="text-muted-foreground">–</span>
        </dd>
      </div>
      <div class="px-2 py-1.5">
        <dt class="text-[11px] text-muted-foreground">Quelle</dt>
        <dd class="h-5 truncate text-[13px] font-medium tabular-nums">
          <template v-if="app.version">{{ app.version }}</template>
          <span v-else class="text-muted-foreground">–</span>
        </dd>
      </div>
      <div class="px-2 py-1.5">
        <dt class="text-[11px] text-muted-foreground">Release</dt>
        <dd class="flex h-5 items-center justify-center text-[13px] font-medium tabular-nums">
          <LoaderCircle v-if="releaseLoading" class="size-3.5 animate-spin text-muted-foreground" />
          <Tip v-else-if="releaseError" :text="`Release nicht abrufbar:\n${releaseError}`">
            <CircleAlert class="size-3.5 text-amber-500" tabindex="0" />
          </Tip>
          <Tip v-else-if="releaseInfo" :text="releaseTip">
            <button
              class="inline-flex items-center gap-1 rounded px-1 hover:text-accent-text"
              @click="openGithub(releaseInfo.html_url)"
            >
              <Tag class="size-3" />{{ releaseVersion }}
            </button>
          </Tip>
          <span v-else class="text-muted-foreground">–</span>
        </dd>
      </div>
    </dl>

    <div class="min-h-3 flex-1" />

    <footer class="flex h-10 items-center gap-2 border-t border-border bg-foreground/[0.02] pr-1.5 pl-3.5">
      <Tip :text="statusView.tip">
        <span class="inline-flex min-w-0 flex-1 items-center gap-1.5 text-xs" tabindex="0">
          <component :is="statusView.icon" class="size-3.5 shrink-0" :style="{ color: statusView.color }" />
          <span class="truncate">{{ statusView.label }}</span>
        </span>
      </Tip>
      <Tip v-if="exe" :text="exe">
        <button class="btn btn-primary h-7 px-2.5 text-xs" @click="launchApp(app)"><Play />Starten</button>
      </Tip>
      <span v-else-if="app.installed" class="pr-2 text-xs text-muted-foreground">Programm nicht gefunden</span>
    </footer>
  </article>
</template>
