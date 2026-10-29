<script setup lang="ts">
import { computed, ref } from "vue";
import { FolderOpen, GitBranch, LayoutGrid, List, Settings, TriangleAlert } from "@lucide/vue";
import EntryIcon from "./EntryIcon.vue";
import Toggle from "./Toggle.vue";
import Dialog from "@mrtools/ui/components/Dialog";
import Segmented from "@mrtools/ui/components/Segmented";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "@/lib/api";
import { git, type GitStatus } from "@/lib/git";
import { settings, state } from "@/lib/store";

const open = computed({
  get: () => state.settingsOpen,
  set: (value) => (state.settingsOpen = value),
});

const savingMenu = ref(false);

async function setContextMenu(enabled: boolean) {
  savingMenu.value = true;
  try {
    await api.setContextMenu(enabled);
    settings.contextMenu = enabled;
  } catch (e) {
    toastError("Kontextmenü konnte nicht geändert werden", e);
  } finally {
    savingMenu.value = false;
  }
}

const views = [
  { value: "list" as const, label: "Liste", icon: List },
  { value: "grid" as const, label: "Kacheln", icon: LayoutGrid },
];

const gitOptions: { key: "clean" | "colorNames" | "ignored" | "branch"; label: string; hint: string }[] = [
  { key: "clean", label: "Unveränderte Dateien markieren", hint: "Grünes Häkchen an allem, was dem letzten Commit entspricht" },
  { key: "colorNames", label: "Dateinamen einfärben", hint: "Geänderte, neue und ignorierte Dateien farbig wie in VS Code" },
  { key: "ignored", label: "Ignorierte Dateien kennzeichnen", hint: "Was .gitignore ausschließt, z. B. node_modules" },
  { key: "branch", label: "Branch in der Statusleiste", hint: "Aktueller Branch, Commits vor/hinter dem Remote, Zahl der Änderungen" },
];

const legend: { status: GitStatus; label: string }[] = [
  { status: "clean", label: "unverändert" },
  { status: "modified", label: "geändert" },
  { status: "staged", label: "vorgemerkt (git add)" },
  { status: "untracked", label: "nicht versioniert" },
  { status: "conflict", label: "Konflikt" },
  { status: "ignored", label: "ignoriert" },
];
</script>

<template>
  <Dialog v-model:open="open" title="Einstellungen" size="md">
    <template #icon>
      <div class="grid size-8 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text">
        <Settings class="size-4" />
      </div>
    </template>

    <div class="flex flex-col gap-5 text-[13px]">
      <section>
        <h3 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Ansicht</h3>
        <div class="flex flex-col divide-y divide-border rounded-lg border border-border">
          <div class="flex items-center gap-3 px-3 py-2.5">
            <span class="flex-1">Darstellung</span>
            <Segmented v-model="settings.view" :options="views" />
          </div>
          <label class="flex cursor-pointer items-center gap-3 px-3 py-2.5">
            <span class="flex-1">Versteckte Dateien anzeigen <span class="kbd ml-1">Strg+H</span></span>
            <Toggle v-model="settings.showHidden" label="Versteckte Dateien anzeigen" />
          </label>
          <label class="flex cursor-pointer items-center gap-3 px-3 py-2.5">
            <span class="flex-1">Detailbereich anzeigen <span class="kbd ml-1">Alt+P</span></span>
            <Toggle v-model="settings.showDetails" label="Detailbereich anzeigen" />
          </label>
        </div>
      </section>

      <section>
        <h3 class="mb-2 flex items-center gap-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          <FolderOpen class="size-3.5" />Windows-Explorer
        </h3>
        <label class="flex cursor-pointer items-start gap-3 rounded-lg border border-border px-3 py-2.5">
          <span class="flex-1">
            <span class="block font-medium">„mrfilesys hier öffnen“ im Kontextmenü</span>
            <span class="block text-xs text-muted-foreground">
              Rechtsklick auf einen Ordner, ein Laufwerk oder in einen Ordner öffnet ihn in mrfilesys – in einem neuen
              Tab, wenn mrfilesys schon läuft. Unter Windows 11 steht der Eintrag unter „Weitere Optionen anzeigen“.
            </span>
          </span>
          <Toggle
            :model-value="settings.contextMenu"
            label="Im Explorer-Kontextmenü"
            :disabled="savingMenu"
            @update:model-value="setContextMenu"
          />
        </label>
      </section>

      <section>
        <h3 class="mb-2 flex items-center gap-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          <GitBranch class="size-3.5" />Git
        </h3>
        <div class="flex flex-col divide-y divide-border rounded-lg border border-border">
          <label class="flex cursor-pointer items-start gap-3 px-3 py-2.5">
            <span class="flex-1">
              <span class="block font-medium">Git-Status an Dateien und Ordnern</span>
              <span class="block text-xs text-muted-foreground">
                In Ordnern mit Git-Repository zeigt ein Symbol, ob eine Datei geändert, neu oder unverändert ist.
              </span>
            </span>
            <Toggle v-model="settings.git.enabled" label="Git-Status anzeigen" />
          </label>
          <label
            v-for="option in gitOptions"
            :key="option.key"
            class="flex items-start gap-3 py-2.5 pr-3 pl-6"
            :class="settings.git.enabled ? 'cursor-pointer' : 'opacity-50'"
          >
            <span class="flex-1">
              <span class="block">{{ option.label }}</span>
              <span class="block text-xs text-muted-foreground">{{ option.hint }}</span>
            </span>
            <Toggle v-model="settings.git[option.key]" :label="option.label" :disabled="!settings.git.enabled" />
          </label>
        </div>

        <p v-if="git.version === null" class="mt-2 flex items-start gap-2 text-xs text-amber-600 dark:text-amber-400">
          <TriangleAlert class="mt-0.5 size-3.5 shrink-0" />
          Git ist nicht installiert oder nicht im PATH – ohne Git bleibt die Anzeige leer.
        </p>
        <p v-else-if="git.version" class="mt-2 text-xs text-muted-foreground">Verwendet git {{ git.version }}.</p>
        <p v-if="git.error" class="mt-1 text-xs text-red-500">Letzter Fehler: {{ git.error }}</p>

        <div class="mt-3 grid grid-cols-3 gap-x-4 gap-y-2 rounded-lg bg-foreground/[0.03] px-3 py-2.5 text-xs">
          <span v-for="item in legend" :key="item.status" class="inline-flex items-center gap-2">
            <EntryIcon :entry="{ name: 'datei.txt', isDir: false, link: false }" :git="item.status" class="size-4" />
            {{ item.label }}
          </span>
        </div>
      </section>
    </div>

    <template #footer>
      <div class="flex-1" />
      <button class="btn btn-primary h-7" @click="state.settingsOpen = false">Schließen</button>
    </template>
  </Dialog>
</template>
