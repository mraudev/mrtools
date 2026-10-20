<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { PopoverAnchor, PopoverContent, PopoverPortal, PopoverRoot } from "reka-ui";
import { Box, FileText, FolderOpen, Search, User, X } from "@lucide/vue";
import logo from "@/assets/logo.svg";
import WindowControls from "@mrtools/ui/components/WindowControls";
import { api } from "@/lib/api";
import { oeffneBeleg, oeffneKunde, zeigeSeite } from "@/lib/nav";
import type { Treffer } from "@/lib/types";

const suche = ref("");
const treffer = ref<Treffer[]>([]);
const offen = ref(false);
const aktiv = ref(0);
const feld = ref<HTMLInputElement>();
let timer: ReturnType<typeof setTimeout> | undefined;

watch(suche, (text) => {
  clearTimeout(timer);
  if (text.trim().length < 2) {
    treffer.value = [];
    offen.value = false;
    return;
  }
  timer = setTimeout(async () => {
    try {
      treffer.value = await api.suche(text);
      aktiv.value = 0;
      offen.value = true;
    } catch {
      treffer.value = [];
    }
  }, 180);
});

const icons = { kunde: User, projekt: FolderOpen, beleg: FileText, artikel: Box };
const artText = { kunde: "Kunde", projekt: "Projekt", beleg: "Beleg", artikel: "Artikel" };

async function waehlen(t: Treffer) {
  offen.value = false;
  suche.value = "";
  feld.value?.blur();
  if (t.art === "kunde") await oeffneKunde(t.id);
  else if (t.art === "projekt") await oeffneKunde(t.kunde_id, t.id);
  else if (t.art === "beleg") {
    await oeffneKunde(t.kunde_id, t.projekt_id);
    await oeffneBeleg(t.id);
  } else if (t.art === "artikel") await zeigeSeite("artikel");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    aktiv.value = Math.min(aktiv.value + 1, treffer.value.length - 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    aktiv.value = Math.max(aktiv.value - 1, 0);
  } else if (e.key === "Enter" && treffer.value[aktiv.value]) {
    waehlen(treffer.value[aktiv.value]);
  } else if (e.key === "Escape") {
    suche.value = "";
    feld.value?.blur();
  }
}

function global(e: KeyboardEvent) {
  if (e.ctrlKey && e.key.toLowerCase() === "k") {
    e.preventDefault();
    feld.value?.focus();
    feld.value?.select();
  }
}
onMounted(() => window.addEventListener("keydown", global));
onUnmounted(() => window.removeEventListener("keydown", global));
</script>

<template>
  <header class="flex h-11 shrink-0 items-stretch border-b border-border bg-card select-none" data-tauri-drag-region>
    <div class="flex w-52 items-center gap-2 pr-4 pl-3.5" data-tauri-drag-region>
      <img :src="logo" alt="" class="pointer-events-none size-5" />
      <span class="pointer-events-none text-[13px] font-semibold tracking-tight">mrfakt</span>
    </div>
    <div class="min-w-6 flex-1" data-tauri-drag-region />
    <PopoverRoot v-model:open="offen">
      <PopoverAnchor as-child>
        <div class="relative my-auto w-[28rem] min-w-48 shrink">
          <Search class="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
          <input
            ref="feld"
            v-model="suche"
            class="input h-8 pr-16 pl-8"
            placeholder="Kunde, Projekt, Beleg-Nr. oder Artikel suchen"
            spellcheck="false"
            @keydown="onKeydown"
            @focus="offen = treffer.length > 0"
          />
          <button v-if="suche" class="icon-btn absolute top-0.5 right-0.5" aria-label="Suche leeren" @click="suche = ''"><X /></button>
          <span v-else class="kbd pointer-events-none absolute top-1.5 right-2">Strg+K</span>
        </div>
      </PopoverAnchor>
      <PopoverPortal>
        <PopoverContent
          align="start"
          :side-offset="6"
          class="menu anim-fade z-50 w-[28rem] p-1"
          @open-auto-focus.prevent
          @interact-outside="(e: Event) => { if (feld && feld.contains(e.target as Node)) e.preventDefault(); }"
        >
          <p v-if="!treffer.length" class="px-2 py-3 text-xs text-muted-foreground">Nichts gefunden.</p>
          <button
            v-for="(t, i) in treffer"
            :key="t.art + t.id"
            class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left"
            :class="i === aktiv ? 'bg-foreground/8' : 'hover:bg-foreground/5'"
            @mouseenter="aktiv = i"
            @click="waehlen(t)"
          >
            <component :is="icons[t.art]" class="size-4 shrink-0 text-muted-foreground" />
            <span class="min-w-0 flex-1">
              <span class="block truncate text-[13px]">{{ t.titel }}</span>
              <span class="block truncate text-[11px] text-muted-foreground">{{ artText[t.art] }} · {{ t.info }}</span>
            </span>
          </button>
        </PopoverContent>
      </PopoverPortal>
    </PopoverRoot>
    <div class="min-w-6 flex-1" data-tauri-drag-region />
    <div class="w-52" data-tauri-drag-region />
    <WindowControls />
  </header>
</template>
