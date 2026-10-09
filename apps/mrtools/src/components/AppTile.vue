<script setup lang="ts">
import { computed } from "vue";
import {
  CircleAlert,
  CircleArrowUp,
  CircleCheck,
  CircleDashed,
  Download,
  LoaderCircle,
  Play,
  RefreshCw,
  Tag,
} from "@lucide/vue";
import Tip from "@mrtools/ui/components/Tip";
import GithubIcon from "./ui/GithubIcon.vue";
import { installApp, launchApp, openGithub } from "@/lib/actions";
import { state } from "@/lib/store";
import type { App } from "@/lib/types";
import { appStatus } from "@/lib/version";

const props = defineProps<{ app: App }>();

const icon = computed(() =>
  props.app.icon ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(props.app.icon)}` : null,
);
const githubUrl = computed(() => `https://github.com/mraudev/mrtools/tree/HEAD/apps/${props.app.folder}`);

const release = computed(() => state.releases[props.app.folder] ?? null);
const releaseTip = computed(() => {
  const date = release.value?.publishedAt;
  return `Release auf GitHub öffnen${date ? `\nVeröffentlicht am ${new Date(date).toLocaleDateString("de-DE")}` : ""}`;
});

/** `undefined` = not installed, `null` = installed without a version. */
const installedVersion = computed(() => (props.app.installed ? props.app.installed.version : undefined));
const status = computed(() => appStatus(installedVersion.value, release.value?.version ?? null));
const exe = computed(() => props.app.installed?.exe ?? null);
const installing = computed(() => state.installing.has(props.app.folder));
/** An installer is available for the latest release. */
const installable = computed(() => release.value?.installerSize != null);

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
      label: `Update auf ${s.latest}`,
      icon: CircleArrowUp,
      color: "var(--status-warning)",
      tip: `Installiert ist ${installedVersion.value}, aktuell ist ${s.latest}.`,
    };
  }
  return {
    label: "Aktuell",
    icon: CircleCheck,
    color: "var(--status-good)",
    tip: s.latest ? `Installiert ist die aktuelle Version ${s.latest}.` : "Installiert – kein Release bekannt.",
  };
});

function formatSize(bytes: number) {
  return `${(bytes / 1024 / 1024).toLocaleString("de-DE", { maximumFractionDigits: 1 })} MB`;
}
const installTip = computed(() => {
  const r = release.value;
  if (!r) return undefined;
  const size = r.installerSize != null ? ` (${formatSize(r.installerSize)})` : "";
  return status.value.kind === "missing"
    ? `Version ${r.version} herunterladen und installieren${size}`
    : `Auf Version ${r.version} aktualisieren${size}\nEine laufende Instanz wird dafür beendet.`;
});
</script>

<template>
  <article
    class="flex min-w-0 flex-col overflow-hidden rounded-xl border border-border bg-card shadow-sm transition-shadow hover:shadow-md"
  >
    <div class="h-[3px] bg-linear-to-r from-accent via-accent/60 to-accent/20" />

    <header class="flex items-start gap-3 px-4 pt-3.5 pb-3">
      <img v-if="icon" :src="icon" alt="" class="size-9 shrink-0 rounded-lg" />
      <div v-else class="size-9 shrink-0 rounded-lg bg-accent/15" />
      <div class="min-w-0 flex-1">
        <h3 class="truncate text-[15px] leading-5 font-semibold">{{ app.name }}</h3>
        <Tip :text="app.description ?? undefined">
          <p class="mt-0.5 truncate text-xs text-muted-foreground">{{ app.description }}</p>
        </Tip>
      </div>
      <Tip :text="`Quellcode auf GitHub öffnen\nmraudev/mrtools/apps/${app.folder}`">
        <button class="icon-btn -mt-0.5 -mr-1.5" aria-label="Auf GitHub öffnen" @click="openGithub(githubUrl)">
          <GithubIcon />
        </button>
      </Tip>
    </header>

    <dl class="mx-4 grid grid-cols-2 divide-x divide-border rounded-lg border border-border bg-foreground/[0.02] text-center">
      <div class="px-2 py-1.5">
        <dt class="text-[11px] text-muted-foreground">Installiert</dt>
        <dd class="h-5 truncate text-[13px] font-medium tabular-nums">
          <template v-if="installedVersion">{{ installedVersion }}</template>
          <span v-else class="text-muted-foreground">–</span>
        </dd>
      </div>
      <div class="px-2 py-1.5">
        <dt class="text-[11px] text-muted-foreground">Verfügbar</dt>
        <dd class="flex h-5 items-center justify-center text-[13px] font-medium tabular-nums">
          <LoaderCircle v-if="state.releasesLoading && !release" class="size-3.5 animate-spin text-muted-foreground" />
          <Tip v-else-if="state.releasesError && !release" :text="`Releases nicht abrufbar:\n${state.releasesError}`">
            <CircleAlert class="size-3.5 text-amber-500" tabindex="0" />
          </Tip>
          <Tip v-else-if="release" :text="releaseTip">
            <button class="inline-flex items-center gap-1 rounded px-1 hover:text-accent-text" @click="openGithub(release.url)">
              <Tag class="size-3" />{{ release.version }}
            </button>
          </Tip>
          <span v-else class="text-xs font-normal text-muted-foreground">noch kein Release</span>
        </dd>
      </div>
    </dl>

    <div class="min-h-3 flex-1" />

    <footer class="flex h-10 items-center gap-1.5 border-t border-border bg-foreground/[0.02] pr-1.5 pl-3.5">
      <Tip :text="statusView.tip">
        <span class="inline-flex min-w-0 flex-1 items-center gap-1.5 text-xs" tabindex="0">
          <component :is="statusView.icon" class="size-3.5 shrink-0" :style="{ color: statusView.color }" />
          <span class="truncate">{{ statusView.label }}</span>
        </span>
      </Tip>

      <button v-if="installing" class="btn btn-outline h-7 px-2.5 text-xs" disabled>
        <LoaderCircle class="animate-spin" />{{ status.kind === "missing" ? "Installiert …" : "Aktualisiert …" }}
      </button>
      <template v-else>
        <Tip v-if="status.kind === 'missing' && installable" :text="installTip">
          <button class="btn btn-primary h-7 px-2.5 text-xs" @click="installApp(app, false)">
            <Download />Installieren
          </button>
        </Tip>
        <Tip v-if="status.kind === 'outdated' && installable" :text="installTip">
          <button class="btn btn-outline h-7 px-2.5 text-xs" @click="installApp(app, true)">
            <RefreshCw />Aktualisieren
          </button>
        </Tip>
        <Tip v-if="exe" :text="exe">
          <button class="btn btn-primary h-7 px-2.5 text-xs" @click="launchApp(app)"><Play />Starten</button>
        </Tip>
        <span v-else-if="app.installed" class="pr-2 text-xs text-muted-foreground">Programm nicht gefunden</span>
      </template>
    </footer>
  </article>
</template>
