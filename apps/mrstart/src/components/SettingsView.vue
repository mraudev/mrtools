<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  AppWindow,
  Briefcase,
  Eye,
  FileJson,
  FolderOpen,
  FolderPlus,
  GitBranch,
  GitPullRequest,
  Info,
  LoaderCircle,
  Monitor,
  Moon,
  Palette,
  Plus,
  RefreshCw,
  Sparkles,
  Sun,
  SquareTerminal,
  X,
} from "@lucide/vue";
import Section from "./ui/Section.vue";
import Segmented from "./ui/Segmented.vue";
import SecretField from "./ui/SecretField.vue";
import Tip from "./ui/Tip.vue";
import { api } from "@/lib/api";
import { openPath } from "@/lib/actions";
import { addWatchedFolder, categories, openProjectEditor, store } from "@/lib/store";
import { ACCENTS } from "@/lib/theme";
import { DEFAULT_COMMANDS, folderName, type GitTool, type ReviewTarget, type Theme } from "@/lib/types";
import { checkForUpdate, updater } from "@/lib/updater";

const settings = computed(() => store.config.settings);
const version = ref("");
const configPath = ref("");

onMounted(async () => {
  version.value = await getVersion();
  configPath.value = await api.configPath();
});

const projects = computed(() =>
  [...store.config.projects].sort(
    (a, b) => a.category.localeCompare(b.category) || a.name.localeCompare(b.name),
  ),
);

async function addWatched() {
  const directory = await openDialog({ directory: true, title: "Ordner überwachen" });
  if (typeof directory === "string") addWatchedFolder(directory);
}

const newPattern = ref("");
function addPattern() {
  const pattern = newPattern.value.trim();
  if (pattern && !settings.value.defaultApps.includes(pattern)) {
    settings.value.defaultApps.push(pattern);
  }
  newPattern.value = "";
}

const launchFields = [
  { key: "editorCommand", label: "Editor", placeholder: DEFAULT_COMMANDS.editor },
  { key: "terminalCommand", label: "Terminal", placeholder: DEFAULT_COMMANDS.terminal },
  { key: "bashCommand", label: "Git Bash", placeholder: DEFAULT_COMMANDS.bash },
] as const;

// Same rule as the backend (pulls.rs): letters, digits, dots and dashes.
const giteaHostInvalid = computed(
  () => settings.value.giteaHost !== "" && !/^[a-z0-9.-]+$/i.test(settings.value.giteaHost),
);

const gitTools: { value: GitTool; label: string }[] = [
  { value: "fork", label: "Fork" },
  { value: "tortoise", label: "TortoiseGit" },
];
const gitToolPathKey = computed(() => (settings.value.gitTool === "tortoise" ? "tortoisePath" : "forkPath"));

async function browseGitTool() {
  const file = await openDialog({
    title: "Programm wählen",
    filters: [{ name: "Programme", extensions: ["exe"] }],
  });
  if (typeof file === "string") settings.value[gitToolPathKey.value] = file;
}

const reviewTargets: { value: ReviewTarget; label: string; icon: typeof Sun }[] = [
  { value: "desktop", label: "Claude Desktop", icon: AppWindow },
  { value: "terminal", label: "Claude Code im Terminal", icon: SquareTerminal },
];

const themes: { value: Theme; label: string; icon: typeof Sun }[] = [
  { value: "dark", label: "Dunkel", icon: Moon },
  { value: "light", label: "Hell", icon: Sun },
  { value: "system", label: "System", icon: Monitor },
];

const updateLabel = computed(() => {
  switch (updater.status) {
    case "checking":
      return "Suche nach Updates …";
    case "available":
      return `Version ${updater.version} ist verfügbar.`;
    case "current":
      return "mrstart ist auf dem neuesten Stand.";
    case "error":
      return "Die letzte Update-Prüfung ist fehlgeschlagen.";
    default:
      return "Updates werden beim Start und alle 6 Stunden geprüft.";
  }
});

function configDirectory() {
  return configPath.value.replace(/[\\/][^\\/]+$/, "");
}
</script>

<template>
  <div class="mx-auto max-w-4xl space-y-4 p-5">
    <Section :icon="Briefcase" title="Projekte" description="Fest eingetragene Projekte; die Kategorie bestimmt den Tab.">
      <template #actions>
        <button class="btn btn-primary" @click="openProjectEditor()"><Plus />Neues Projekt</button>
      </template>
      <div v-if="projects.length" class="overflow-hidden rounded-lg border border-border">
        <table class="w-full table-fixed text-left">
          <thead class="bg-foreground/[0.03] text-xs text-muted-foreground">
            <tr>
              <th class="w-[24%] px-3 py-2 font-medium">Name</th>
              <th class="w-[16%] px-3 py-2 font-medium">Kategorie</th>
              <th class="px-3 py-2 font-medium">Pfad</th>
              <th class="w-32 px-3 py-2 text-right font-medium whitespace-nowrap">Apps / Befehle</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            <tr
              v-for="p in projects"
              :key="p.id"
              class="cursor-pointer transition-colors hover:bg-foreground/[0.04]"
              @click="openProjectEditor(p)"
            >
              <td class="truncate px-3 py-2 font-medium">
                {{ p.name }}
                <span v-if="p.version" class="ml-1 text-xs text-muted-foreground">{{ p.version }}</span>
              </td>
              <td class="truncate px-3 py-2 capitalize">{{ p.category }}</td>
              <td class="truncate px-3 py-2 font-mono text-xs text-muted-foreground">{{ p.path }}</td>
              <td class="px-3 py-2 text-right text-muted-foreground tabular-nums">
                {{ p.apps.length }} / {{ p.commands.length }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-else class="text-muted-foreground">Noch keine Projekte angelegt.</p>
    </Section>

    <Section
      :icon="Eye"
      title="Überwachte Ordner"
      description="Jeder Unterordner erscheint automatisch als Projekt im angegebenen Tab. Gleicher Tab-Name wie eine Projekt-Kategorie = gemeinsamer Tab."
    >
      <template #actions>
        <button class="btn btn-outline" @click="addWatched"><FolderPlus />Ordner hinzufügen</button>
      </template>
      <ul v-if="settings.watchedFolders.length" class="divide-y divide-border rounded-lg border border-border">
        <li
          v-for="(folder, index) in settings.watchedFolders"
          :key="folder.path"
          class="flex items-center gap-2 py-1.5 pr-1.5 pl-3"
        >
          <span class="min-w-0 flex-1 truncate font-mono text-[13px]">{{ folder.path }}</span>
          <label class="text-xs text-muted-foreground" :for="`watched-tab-${index}`">Tab</label>
          <input
            :id="`watched-tab-${index}`"
            v-model="folder.category"
            class="input h-7 w-44 text-[13px]"
            list="watched-categories"
            :placeholder="folderName(folder.path)"
            spellcheck="false"
          />
          <Tip text="Im Explorer öffnen">
            <button class="icon-btn" aria-label="Im Explorer öffnen" @click="openPath(folder.path)"><FolderOpen /></button>
          </Tip>
          <Tip text="Nicht mehr überwachen">
            <button
              class="icon-btn"
              aria-label="Nicht mehr überwachen"
              @click="settings.watchedFolders.splice(index, 1)"
            >
              <X />
            </button>
          </Tip>
        </li>
      </ul>
      <p v-else class="text-muted-foreground">Keine Ordner überwacht.</p>
      <datalist id="watched-categories">
        <option v-for="c in categories" :key="c" :value="c" />
      </datalist>
    </Section>

    <Section
      :icon="Sparkles"
      title="Standard-Apps"
      description="Dateien, die bei überwachten Ordnern und neuen Projekten automatisch als App-Button erscheinen."
    >
      <div class="flex flex-wrap items-center gap-1.5">
        <span
          v-for="(pattern, index) in settings.defaultApps"
          :key="pattern"
          class="inline-flex h-7 items-center gap-1 rounded-md border border-border bg-foreground/[0.03] pr-1 pl-2.5 font-mono text-[13px]"
        >
          {{ pattern }}
          <button
            class="icon-btn size-5 [&_svg]:size-3.5"
            :aria-label="`${pattern} entfernen`"
            @click="settings.defaultApps.splice(index, 1)"
          >
            <X />
          </button>
        </span>
        <form class="flex items-center gap-1.5" @submit.prevent="addPattern">
          <input
            v-model="newPattern"
            class="input h-7 w-48 font-mono text-[13px]"
            placeholder="*.sln oder build.cmd"
            spellcheck="false"
          />
          <button type="submit" class="btn btn-ghost h-7 px-2" :disabled="!newPattern.trim()"><Plus /></button>
        </form>
      </div>
    </Section>

    <Section
      :icon="SquareTerminal"
      title="Programme"
      description="Befehle für die Schnellaktionen jeder Kachel. {path} wird durch den Projektordner ersetzt; leer = Standard."
    >
      <div class="grid grid-cols-[7rem_1fr] items-center gap-x-3 gap-y-2">
        <template v-for="field in launchFields" :key="field.key">
          <label class="text-sm text-muted-foreground" :for="field.key">{{ field.label }}</label>
          <input
            :id="field.key"
            v-model="settings[field.key]"
            class="input font-mono text-[13px]"
            :placeholder="field.placeholder"
            spellcheck="false"
          />
        </template>
      </div>
    </Section>

    <Section :icon="GitBranch" title="Git" description="Programm für Änderungen und Log. Pull/Push nutzen git aus dem PATH.">
      <div class="flex flex-wrap items-center gap-3">
        <Segmented v-model="settings.gitTool" :options="gitTools" />
        <div class="flex min-w-72 flex-1 gap-2">
          <input
            v-model="settings[gitToolPathKey]"
            class="input font-mono text-[13px]"
            placeholder="Pfad zur .exe – leer = automatisch erkennen"
            spellcheck="false"
          />
          <button class="btn btn-outline" @click="browseGitTool"><FolderOpen />Wählen</button>
        </div>
      </div>
    </Section>

    <Section
      :icon="GitPullRequest"
      title="Pull Requests"
      description="Offene PRs des ausgecheckten Branches werden über den Kacheln angezeigt (GitHub und ein Gitea-Server)."
    >
      <div class="grid grid-cols-[7rem_1fr] items-center gap-x-3 gap-y-2">
        <label class="text-sm text-muted-foreground" for="github-token">GitHub-Token</label>
        <SecretField id="github-token" name="github" />
        <label class="text-sm text-muted-foreground" for="gitea-host">Gitea-Host</label>
        <div>
          <input
            id="gitea-host"
            v-model.trim="settings.giteaHost"
            class="input font-mono text-[13px]"
            placeholder="z. B. gitea.example.com"
            spellcheck="false"
          />
          <p v-if="giteaHostInvalid" class="mt-1 text-xs text-red-500">
            Nur den Hostnamen eintragen, ohne https:// und ohne Pfad.
          </p>
        </div>
        <label class="text-sm text-muted-foreground" for="gitea-token">Gitea-Token</label>
        <SecretField id="gitea-token" name="gitea" />
      </div>
      <div class="mt-2 space-y-1 text-xs text-muted-foreground">
        <p>
          Tokens werden verschlüsselt in der Windows-Anmeldeinformationsverwaltung abgelegt (nur auf diesem
          PC), nie in der Konfigurationsdatei, und lassen sich nicht wieder anzeigen. Sie gehen nur per HTTPS
          an api.github.com bzw. den Gitea-Host.
        </p>
        <p>
          Benötigte Rechte – <b>Gitea</b>: „repository“ und „issue“ lesen; zum Aktualisieren von Branches
          „repository“ schreiben. <b>GitHub</b> (Fine-grained): „Pull requests“ lesen; zum Aktualisieren lesen
          &amp; schreiben. Nur so viele Rechte vergeben wie nötig.
        </p>
      </div>
    </Section>

    <Section
      :icon="Sparkles"
      title="Review mit Claude"
      description="Wohin der Knopf „Mit Claude reviewen“ im Pull-Request-Dashboard den Review-Auftrag schickt."
    >
      <Segmented v-model="settings.reviewTarget" :options="reviewTargets" />
      <p class="mt-2 text-xs text-muted-foreground">
        <template v-if="settings.reviewTarget === 'desktop'">
          Der Auftrag wird nur vorausgefüllt und erst gesendet, wenn du ihn bestätigst.
        </template>
        <template v-else>
          Claude Code startet im Auto-Modus und schickt den Auftrag sofort ab. Claude Code muss installiert sein.
        </template>
        Liegt das Repository lokal als Projekt oder in einem überwachten Ordner, arbeitet Claude dort und
        kennt den übrigen Code.
      </p>
    </Section>

    <Section :icon="Palette" title="Darstellung">
      <div class="flex flex-wrap items-center gap-x-8 gap-y-3">
        <Segmented v-model="settings.theme" :options="themes" />
        <div class="flex flex-wrap gap-1.5">
          <Tip v-for="(value, name) in ACCENTS" :key="name" :text="name">
            <button
              class="size-7 rounded-full ring-offset-2 ring-offset-card transition-transform hover:scale-110"
              :class="settings.accent === name && 'ring-2 ring-foreground/70'"
              :style="{ background: value[0] }"
              :aria-label="`Akzentfarbe ${name}`"
              @click="settings.accent = name"
            />
          </Tip>
        </div>
      </div>
    </Section>

    <Section :icon="Info" title="Über mrstart" :description="`Version ${version}`">
      <div class="space-y-3">
        <div class="flex flex-wrap items-center gap-3">
          <button
            class="btn btn-outline"
            :disabled="updater.status === 'checking' || updater.status === 'downloading'"
            @click="checkForUpdate(true)"
          >
            <LoaderCircle v-if="updater.status === 'checking'" class="animate-spin" />
            <RefreshCw v-else />
            Nach Updates suchen
          </button>
          <button v-if="updater.status === 'available'" class="btn btn-primary" @click="updater.dialogOpen = true">
            Update {{ updater.version }} anzeigen
          </button>
          <span class="text-muted-foreground">{{ updateLabel }}</span>
        </div>
        <div class="flex items-center gap-2 rounded-lg border border-border bg-background py-1.5 pr-1.5 pl-3">
          <FileJson class="size-4 shrink-0 text-muted-foreground" />
          <span class="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground select-text">{{ configPath }}</span>
          <Tip text="Konfigurationsordner öffnen">
            <button class="icon-btn" aria-label="Konfigurationsordner öffnen" @click="openPath(configDirectory())">
              <FolderOpen />
            </button>
          </Tip>
        </div>
      </div>
    </Section>
  </div>
</template>
