<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { ArrowLeft, ArrowRight, ArrowUp, ChevronRight, Globe, LoaderCircle, Monitor, RefreshCw, Search, X } from "@lucide/vue";
import Tip from "@mrtools/ui/components/Tip";
import { toastError } from "@mrtools/ui/lib/toast";
import { api } from "@/lib/api";
import { openEntry } from "@/lib/actions";
import { drag } from "@/lib/dnd";
import { search } from "@/lib/search";
import { crumbs, normalizePath, parentPath } from "@/lib/paths";
import { canGoBack, canGoForward, goBack, goForward, goUp, navigate, refresh, state } from "@/lib/store";

const editing = ref(false);
const draft = ref("");
const input = ref<HTMLInputElement>();
const parts = computed(() => crumbs(state.path));

async function editAddress() {
  draft.value = state.path;
  editing.value = true;
  await nextTick();
  input.value?.focus();
  input.value?.select();
}

defineExpose({ editAddress });

async function submit() {
  const path = normalizePath(draft.value.replace(/^"|"$/g, ""));
  editing.value = false;
  if (!path) return navigate("");
  try {
    const entry = await api.stat(path);
    if (entry.isDir) navigate(entry.path);
    // A file: show its folder and open it, like Explorer.
    else if (await navigate(parentPath(entry.path), { select: entry.path })) openEntry(entry);
  } catch (e) {
    toastError(`„${path}“ wurde nicht gefunden`, e);
  }
}

function onSearchKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    state.filter = "";
    search.everywhere = false;
    (event.target as HTMLInputElement).blur();
  } else if (event.key === "ArrowDown" || event.key === "Enter") {
    event.preventDefault();
    (document.getElementById("search-results") ?? document.getElementById("file-list"))?.focus();
  }
}
</script>

<template>
  <div class="flex h-11 shrink-0 items-center gap-1 border-b border-border bg-card px-2">
    <Tip text="Zurück (Alt+←)" side="bottom">
      <button class="icon-btn" aria-label="Zurück" :disabled="!canGoBack" @click="goBack"><ArrowLeft /></button>
    </Tip>
    <Tip text="Vorwärts (Alt+→)" side="bottom">
      <button class="icon-btn" aria-label="Vorwärts" :disabled="!canGoForward" @click="goForward"><ArrowRight /></button>
    </Tip>
    <Tip text="Übergeordneter Ordner (Alt+↑)" side="bottom">
      <button class="icon-btn" aria-label="Übergeordneter Ordner" :disabled="!state.path" @click="goUp"><ArrowUp /></button>
    </Tip>
    <Tip text="Aktualisieren (F5)" side="bottom">
      <button class="icon-btn" aria-label="Aktualisieren" @click="refresh()">
        <LoaderCircle v-if="state.loading" class="animate-spin" />
        <RefreshCw v-else />
      </button>
    </Tip>

    <div class="ml-1 min-w-0 flex-1">
      <input
        v-if="editing"
        ref="input"
        v-model="draft"
        class="input font-mono text-[13px]"
        spellcheck="false"
        aria-label="Adresse"
        @keydown.enter="submit"
        @keydown.esc="editing = false"
        @blur="editing = false"
      />
      <div
        v-else
        class="flex h-8 min-w-0 cursor-text items-center rounded-md border border-border bg-background pr-2 pl-1 text-[13px]"
        title="Klicken, um einen Pfad einzugeben (Strg+L) – auch mit Variablen wie %appdata% oder %temp%"
        @click.self="editAddress"
      >
        <button
          class="inline-flex h-6 shrink-0 items-center gap-1.5 rounded px-1.5 hover:bg-foreground/8 [&_svg]:size-3.5"
          @click="navigate('')"
        >
          <Monitor class="text-muted-foreground" />
          <span v-if="!parts.length">Dieser PC</span>
        </button>
        <template v-for="part in parts" :key="part.path">
          <ChevronRight class="size-3.5 shrink-0 text-muted-foreground" />
          <button
            class="h-6 min-w-0 truncate rounded px-1.5 hover:bg-foreground/8"
            :class="[part.path === state.path && 'font-medium', drag.op && drag.target === part.path && 'bg-accent/25 ring-2 ring-accent']"
            :data-drop="part.path"
            @click="navigate(part.path)"
          >
            {{ part.label }}
          </button>
        </template>
        <div class="h-full min-w-8 flex-1" @click="editAddress" />
      </div>
    </div>

    <Tip :text="search.everywhere ? 'Nur im Ordner filtern (Strg+F)' : 'Alle Laufwerke durchsuchen (Strg+Umschalt+F)'" side="bottom">
      <button
        class="icon-btn ml-1"
        :class="search.everywhere && 'bg-accent/15 text-accent-text'"
        aria-label="Alle Laufwerke durchsuchen"
        :aria-pressed="search.everywhere"
        @click="(search.everywhere = !search.everywhere), ($refs.searchInput as HTMLInputElement).focus()"
      >
        <Globe />
      </button>
    </Tip>
    <div class="relative w-72 shrink-0">
      <Search class="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
      <input
        id="search"
        ref="searchInput"
        v-model="state.filter"
        class="input pr-7 pl-8"
        :class="search.everywhere && 'border-accent/60'"
        :placeholder="
          search.everywhere ? 'Alle Laufwerke durchsuchen …' : state.path ? 'Im Ordner filtern (Strg+F)' : 'Filtern (Strg+F)'
        "
        spellcheck="false"
        @keydown="onSearchKeydown"
      />
      <button
        v-if="state.filter"
        class="icon-btn absolute top-1/2 right-0.5 size-6 -translate-y-1/2 [&_svg]:size-3.5"
        aria-label="Filter löschen"
        @click="state.filter = ''"
      >
        <X />
      </button>
    </div>
  </div>
</template>
