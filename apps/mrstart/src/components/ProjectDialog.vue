<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Command, FilePlus, FolderCode, FolderOpen, Plus, Sparkles, Trash2, X } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import Tip from "@mrtools/ui/components/Tip";
import { api } from "@/lib/api";
import { baseName } from "@/lib/actions";
import { folderName } from "@/lib/types";
import { categories, categoryView, deleteProject, editor, saveProject, store } from "@/lib/store";
import { toastError } from "@mrtools/ui/lib/toast";

const submitted = ref(false);
const confirmDelete = ref(false);
let confirmTimer: ReturnType<typeof setTimeout> | undefined;

watch(
  () => editor.open,
  (open) => {
    if (open) {
      submitted.value = false;
      confirmDelete.value = false;
    }
  },
);

const draft = computed(() => editor.draft);

const errors = computed(() => ({
  path: draft.value.path.trim() ? "" : "Bitte einen Projektordner wählen.",
  name: draft.value.name.trim() ? "" : "Bitte einen Namen angeben.",
  category: draft.value.category.trim() ? "" : "Bitte eine Kategorie angeben.",
}));

async function addDefaultApps() {
  if (!draft.value.path.trim()) return;
  try {
    const found = await api.existingFiles(draft.value.path, store.config.settings.defaultApps);
    for (const file of found) {
      if (!draft.value.apps.includes(file)) draft.value.apps.push(file);
    }
  } catch (e) {
    toastError("Standard-Apps konnten nicht gesucht werden", e);
  }
}

async function browseFolder() {
  const directory = await openDialog({
    directory: true,
    defaultPath: draft.value.path || undefined,
    title: "Projektordner wählen",
  });
  if (typeof directory !== "string") return;
  draft.value.path = directory;
  if (!draft.value.name.trim()) draft.value.name = folderName(directory);
  if (editor.isNew) await addDefaultApps();
}

async function addApps() {
  const files = await openDialog({
    multiple: true,
    defaultPath: draft.value.path || undefined,
    title: "Apps hinzufügen",
  });
  for (const file of files ?? []) {
    if (!draft.value.apps.includes(file)) draft.value.apps.push(file);
  }
}

function save() {
  submitted.value = true;
  if (Object.values(errors.value).some(Boolean)) return;
  const p = draft.value;
  const category = p.category.trim();
  saveProject({
    ...p,
    name: p.name.trim(),
    category,
    path: p.path.trim(),
    version: p.version.trim(),
    commands: p.commands
      .map((c) => ({ caption: c.caption.trim(), command: c.command.trim() }))
      .filter((c) => c.command),
  });
  store.view = categoryView(category);
  editor.open = false;
}

function remove() {
  if (!confirmDelete.value) {
    confirmDelete.value = true;
    clearTimeout(confirmTimer);
    confirmTimer = setTimeout(() => (confirmDelete.value = false), 4000);
    return;
  }
  deleteProject(draft.value.id);
  editor.open = false;
}
</script>

<template>
  <Dialog
    v-model:open="editor.open"
    :title="editor.isNew ? 'Neues Projekt' : draft.name || 'Projekt bearbeiten'"
    :description="editor.isNew ? 'Ordner wählen – Name und Standard-Apps werden übernommen.' : draft.path"
    size="lg"
  >
    <template #icon>
      <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent/15 text-accent-text [&_svg]:size-[18px]">
        <FolderCode />
      </div>
    </template>

    <form class="space-y-5" @submit.prevent="save">
      <div>
        <label class="label" for="project-path">Projektordner</label>
        <div class="flex gap-2">
          <input
            id="project-path"
            v-model="draft.path"
            class="input font-mono text-[13px]"
            placeholder="C:\development\projekt"
            spellcheck="false"
          />
          <button type="button" class="btn btn-outline" @click="browseFolder"><FolderOpen />Wählen</button>
        </div>
        <p v-if="submitted && errors.path" class="mt-1 text-xs text-red-500">{{ errors.path }}</p>
      </div>

      <div class="grid grid-cols-[2fr_1.4fr_1fr] gap-3">
        <div>
          <label class="label" for="project-name">Name</label>
          <input id="project-name" v-model="draft.name" class="input" spellcheck="false" />
          <p v-if="submitted && errors.name" class="mt-1 text-xs text-red-500">{{ errors.name }}</p>
        </div>
        <div>
          <label class="label" for="project-category">Kategorie (Tab)</label>
          <input
            id="project-category"
            v-model="draft.category"
            class="input"
            list="project-categories"
            placeholder="development"
            spellcheck="false"
          />
          <datalist id="project-categories">
            <option v-for="c in categories" :key="c" :value="c" />
          </datalist>
          <p v-if="submitted && errors.category" class="mt-1 text-xs text-red-500">{{ errors.category }}</p>
        </div>
        <div>
          <label class="label" for="project-version">Version (optional)</label>
          <input id="project-version" v-model="draft.version" class="input" spellcheck="false" />
        </div>
      </div>

      <section>
        <div class="mb-1.5 flex items-center gap-2">
          <h4 class="text-sm font-semibold">Apps</h4>
          <span class="text-xs text-muted-foreground">Dateien, die per Klick geöffnet werden (.sln, .exe, .bat …)</span>
          <div class="flex-1" />
          <Tip text="Standard-Apps aus den Einstellungen im Ordner suchen">
            <button type="button" class="btn btn-ghost h-7 px-2 text-xs" @click="addDefaultApps"><Sparkles />Suchen</button>
          </Tip>
          <button type="button" class="btn btn-ghost h-7 px-2 text-xs" @click="addApps"><FilePlus />Hinzufügen</button>
        </div>
        <ul v-if="draft.apps.length" class="divide-y divide-border rounded-lg border border-border">
          <li v-for="(app, index) in draft.apps" :key="app" class="flex items-center gap-3 py-1.5 pr-1.5 pl-3">
            <span class="w-40 shrink-0 truncate font-medium">{{ baseName(app) }}</span>
            <span class="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground">{{ app }}</span>
            <button type="button" class="icon-btn" aria-label="Entfernen" @click="draft.apps.splice(index, 1)">
              <X />
            </button>
          </li>
        </ul>
        <p v-else class="rounded-lg border border-dashed border-border px-3 py-2.5 text-xs text-muted-foreground">
          Noch keine Apps.
        </p>
      </section>

      <section>
        <div class="mb-1.5 flex items-center gap-2">
          <h4 class="text-sm font-semibold">Befehle</h4>
          <span class="text-xs text-muted-foreground">
            laufen unsichtbar im Projektordner – für ein Fenster z.&nbsp;B. <code class="kbd">start cmd /k npm run dev</code>
          </span>
          <div class="flex-1" />
          <button
            type="button"
            class="btn btn-ghost h-7 px-2 text-xs"
            @click="draft.commands.push({ caption: '', command: '' })"
          >
            <Plus />Hinzufügen
          </button>
        </div>
        <div v-if="draft.commands.length" class="space-y-1.5">
          <div v-for="(command, index) in draft.commands" :key="index" class="flex items-center gap-2">
            <Command class="size-4 shrink-0 text-muted-foreground" />
            <input v-model="command.caption" class="input w-48 shrink-0" placeholder="Bezeichnung" spellcheck="false" />
            <input
              v-model="command.command"
              class="input font-mono text-[13px]"
              placeholder="start cmd /k npm run dev"
              spellcheck="false"
            />
            <button type="button" class="icon-btn" aria-label="Entfernen" @click="draft.commands.splice(index, 1)">
              <X />
            </button>
          </div>
        </div>
        <p v-else class="rounded-lg border border-dashed border-border px-3 py-2.5 text-xs text-muted-foreground">
          Noch keine Befehle.
        </p>
      </section>

      <!-- Enables submitting with Enter from any input -->
      <button type="submit" class="hidden" />
    </form>

    <template #footer>
      <button v-if="!editor.isNew" class="btn btn-danger" @click="remove">
        <Trash2 />{{ confirmDelete ? "Wirklich löschen?" : "Löschen" }}
      </button>
      <div class="flex-1" />
      <button class="btn btn-ghost" @click="editor.open = false">Abbrechen</button>
      <button class="btn btn-primary" @click="save">Speichern</button>
    </template>
  </Dialog>
</template>
