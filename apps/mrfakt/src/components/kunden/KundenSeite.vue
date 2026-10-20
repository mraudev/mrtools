<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { Plus, Search, X } from "@lucide/vue";
import Segmented from "@mrtools/ui/components/Segmented";
import KundeDetail from "./KundeDetail.vue";
import KundeForm from "./KundeForm.vue";
import { api } from "@/lib/api";
import { betrag, datum } from "@/lib/format";
import { nav } from "@/lib/nav";
import type { KundeZeile } from "@/lib/types";
import { toastError } from "@mrtools/ui/lib/toast";

const suche = ref("");
const typ = ref("");
const liste = ref<KundeZeile[]>([]);
const laedt = ref(false);
const neu = ref(false);
const listeEl = ref<HTMLElement>();
const typen = [
  { value: "", label: "Alle" },
  { value: "Kunde", label: "Kunden" },
  { value: "Lieferant", label: "Lieferanten" },
];

let timer: ReturnType<typeof setTimeout> | undefined;
async function laden() {
  laedt.value = true;
  try {
    liste.value = await api.kunden(suche.value, typ.value);
  } catch (e) {
    toastError("Kunden konnten nicht geladen werden", e);
  } finally {
    laedt.value = false;
  }
}
watch([suche, typ], () => {
  clearTimeout(timer);
  timer = setTimeout(laden, 150);
});
watch(() => nav.version, laden);
onMounted(laden);

const gewaehlt = computed(() => nav.kundeId);

function waehlen(id: number) {
  if (nav.kundeId !== id) {
    nav.kundeId = id;
    nav.projektId = 0;
  }
}

function onKeydown(e: KeyboardEvent) {
  const i = liste.value.findIndex((k) => k.id === nav.kundeId);
  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    e.preventDefault();
    const n = liste.value[Math.max(0, Math.min(liste.value.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
    if (n) {
      waehlen(n.id);
      nextTick(() => listeEl.value?.querySelector(`[data-id="${n.id}"]`)?.scrollIntoView({ block: "nearest" }));
    }
  }
}

async function angelegt(id: number) {
  neu.value = false;
  await laden();
  waehlen(id);
}
</script>

<template>
  <div class="flex min-h-0 flex-1">
    <section class="flex w-[22rem] shrink-0 flex-col border-r border-border bg-card">
      <div class="flex flex-col gap-2 border-b border-border p-3">
        <div class="flex gap-2">
          <div class="relative flex-1">
            <Search class="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
            <input v-model="suche" class="input pl-8" placeholder="Name, Ort, Nummer …" spellcheck="false" @keydown="onKeydown" />
            <button v-if="suche" class="icon-btn absolute top-0.5 right-0.5" aria-label="Leeren" @click="suche = ''"><X /></button>
          </div>
          <button class="btn btn-primary px-2.5" title="Neuer Kunde" @click="neu = true"><Plus />Neu</button>
        </div>
        <Segmented v-model="typ" :options="typen" />
      </div>
      <div ref="listeEl" class="min-h-0 flex-1 overflow-y-auto" tabindex="0" @keydown="onKeydown">
        <button
          v-for="k in liste"
          :key="k.id"
          :data-id="k.id"
          class="flex w-full flex-col gap-0.5 border-b border-border/60 px-3 py-2 text-left"
          :class="gewaehlt === k.id ? 'bg-accent/12' : 'hover:bg-foreground/4'"
          @click="waehlen(k.id)"
        >
          <span class="flex items-baseline gap-2">
            <span class="min-w-0 flex-1 truncate text-[13px] font-medium">{{ k.name || k.suchname || "(ohne Namen)" }}</span>
            <span v-if="k.offen > 0.004" class="shrink-0 text-[11px] font-medium text-amber-600 tabular-nums dark:text-amber-400">{{ betrag(k.offen) }} offen</span>
          </span>
          <span class="flex gap-2 text-[11px] text-muted-foreground">
            <span class="min-w-0 flex-1 truncate">{{ [k.strasse, k.ort].filter(Boolean).join(", ") }}</span>
            <span v-if="k.letzter_beleg" class="shrink-0 tabular-nums">{{ datum(k.letzter_beleg) }}</span>
          </span>
        </button>
        <p v-if="!liste.length && !laedt" class="p-4 text-center text-xs text-muted-foreground">
          {{ suche ? "Keine Treffer." : "Noch keine Kunden. Unter Einstellungen → Daten lassen sich die Daten aus GHFakt übernehmen." }}
        </p>
      </div>
      <div class="border-t border-border px-3 py-1.5 text-[11px] text-muted-foreground">{{ liste.length.toLocaleString("de-DE") }} Einträge</div>
    </section>
    <section class="min-w-0 flex-1 overflow-y-auto">
      <KundeDetail v-if="nav.kundeId" :key="nav.kundeId" :id="nav.kundeId" @geaendert="laden" />
      <div v-else class="grid h-full place-items-center text-sm text-muted-foreground">Kunden links auswählen.</div>
    </section>
    <KundeForm v-if="neu" :kunde="null" @close="neu = false" @saved="angelegt" />
  </div>
</template>
