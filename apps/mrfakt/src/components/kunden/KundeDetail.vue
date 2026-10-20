<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "reka-ui";
import { EllipsisVertical, FolderPlus, Mail, MailPlus, Pencil, Phone, Smartphone, Tag, Trash2 } from "@lucide/vue";
import { save } from "@tauri-apps/plugin-dialog";
import KundeForm from "./KundeForm.vue";
import ProjektKarte from "./ProjektKarte.vue";
import ProjektForm from "./ProjektForm.vue";
import { api } from "@/lib/api";
import { betrag, datum, euro, htmlText } from "@/lib/format";
import { frage, nav, neuLaden } from "@/lib/nav";
import type { Brief, KundeDetail, Projekt } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ id: number }>();
const emit = defineEmits<{ geaendert: [] }>();

const d = ref<KundeDetail>();
const briefe = ref<Brief[]>([]);
const bearbeiten = ref(false);
const projektNeu = ref<Projekt | null>(null);
const offen = ref<Set<number>>(new Set());

async function laden() {
  try {
    d.value = await api.kunde(props.id);
    briefe.value = await api.briefe(props.id);
    if (nav.projektId) offen.value.add(nav.projektId);
    else if (d.value.projekte[0] && offen.value.size === 0) offen.value.add(d.value.projekte[0].id);
  } catch (e) {
    toastError("Kunde konnte nicht geladen werden", e);
  }
}
onMounted(laden);
watch(() => nav.version, laden);
watch(
  () => nav.projektId,
  (id) => {
    if (id) offen.value.add(id);
  },
);

function umschalten(id: number) {
  if (offen.value.has(id)) offen.value.delete(id);
  else offen.value.add(id);
}

function neuesProjekt() {
  projektNeu.value = { id: 0, kunde_id: props.id, nummer: "", bezeichnung: "", datum: "", baustelle: "", status: "offen", notiz: "" };
}

async function projektAngelegt(id: number) {
  projektNeu.value = null;
  nav.projektId = id;
  offen.value.add(id);
  await laden();
  emit("geaendert");
}

async function loeschen() {
  if (!d.value) return;
  if (!(await frage("Kunde löschen", `${d.value.kunde.name1 || d.value.kunde.suchname} wirklich löschen?`, "Löschen"))) return;
  try {
    await api.kundeLoeschen(props.id);
    nav.kundeId = 0;
    neuLaden();
  } catch (e) {
    toastError("Löschen nicht möglich", e);
  }
}

async function neuerBrief() {
  try {
    const b = await api.briefNeu(props.id);
    nav.briefId = b.id;
  } catch (e) {
    toastError("Brief konnte nicht angelegt werden", e);
  }
}

async function etikett() {
  const pfad = await save({ defaultPath: "Adressetikett.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!pfad) return;
  try {
    await api.kundenEtiketten([props.id], pfad, 3, 37);
    await api.oeffnen(pfad);
  } catch (e) {
    toastError("Etikett konnte nicht erstellt werden", e);
  }
}

async function gespeichert() {
  bearbeiten.value = false;
  await laden();
  emit("geaendert");
  toast("success", "Kunde gespeichert");
}

const kontakt = (t: string) => t.trim();
</script>

<template>
  <div v-if="d" class="mx-auto flex max-w-5xl flex-col gap-4 p-5">
    <!-- Kopf -->
    <div class="flex items-start gap-4 rounded-xl border border-border bg-card p-4">
      <div class="min-w-0 flex-1">
        <div class="flex items-baseline gap-2">
          <h1 class="truncate text-lg font-semibold">{{ [d.kunde.name1, d.kunde.name2].filter(Boolean).join(", ") || d.kunde.suchname }}</h1>
          <span class="rounded bg-foreground/6 px-1.5 py-0.5 text-[11px] text-muted-foreground">{{ d.kunde.typ }} {{ d.kunde.nummer }}</span>
        </div>
        <p class="mt-1 text-[13px] whitespace-pre-line text-muted-foreground">{{ [d.kunde.anrede, d.kunde.strasse, [d.kunde.plz, d.kunde.ort].filter(Boolean).join(" ") + (d.kunde.ortsteil && !d.kunde.ort.includes(d.kunde.ortsteil) ? "-" + d.kunde.ortsteil : "")].filter(Boolean).join("\n") }}</p>
        <div class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[13px]">
          <span v-if="kontakt(d.kunde.telefon)" class="flex items-center gap-1.5"><Phone class="size-3.5 text-muted-foreground" />{{ d.kunde.telefon }}</span>
          <span v-if="kontakt(d.kunde.mobil)" class="flex items-center gap-1.5"><Smartphone class="size-3.5 text-muted-foreground" />{{ d.kunde.mobil }}</span>
          <span v-if="kontakt(d.kunde.telefon_privat)" class="flex items-center gap-1.5"><Phone class="size-3.5 text-muted-foreground" />{{ d.kunde.telefon_privat }} (privat)</span>
          <span v-if="kontakt(d.kunde.email)" class="flex items-center gap-1.5"><Mail class="size-3.5 text-muted-foreground" />{{ d.kunde.email }}</span>
        </div>
        <p v-if="d.kunde.notiz" class="mt-2 rounded-md bg-amber-500/8 px-2 py-1 text-xs whitespace-pre-line">{{ d.kunde.notiz }}</p>
      </div>
      <div class="flex flex-col items-end gap-2">
        <div class="flex gap-1">
          <button class="btn btn-outline h-8" @click="bearbeiten = true"><Pencil />Bearbeiten</button>
          <button class="btn btn-primary h-8" @click="neuesProjekt"><FolderPlus />Neues Projekt</button>
          <DropdownMenuRoot>
            <DropdownMenuTrigger as-child>
              <button class="icon-btn size-8" aria-label="Weitere Aktionen"><EllipsisVertical /></button>
            </DropdownMenuTrigger>
            <DropdownMenuPortal>
              <DropdownMenuContent align="end" :side-offset="6" class="menu anim-fade w-56">
                <DropdownMenuItem class="menu-item" @select="neuerBrief"><MailPlus />Neuer Brief</DropdownMenuItem>
                <DropdownMenuItem class="menu-item" @select="etikett"><Tag />Adressetikett</DropdownMenuItem>
                <DropdownMenuSeparator class="menu-separator" />
                <DropdownMenuItem class="menu-item text-red-500" @select="loeschen"><Trash2 />Kunde löschen</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenuPortal>
          </DropdownMenuRoot>
        </div>
        <div class="text-right text-xs text-muted-foreground">
          <div v-if="d.offen > 0.004" class="text-sm font-semibold text-amber-600 dark:text-amber-400">{{ euro(d.offen) }} offen</div>
          <div v-for="[jahr, summe] in d.umsatz.slice(0, 3)" :key="jahr" class="tabular-nums">Umsatz {{ jahr }}: {{ betrag(summe) }} €</div>
        </div>
      </div>
    </div>

    <!-- Projekte -->
    <div class="flex items-center gap-2">
      <h2 class="text-sm font-semibold">Projekte</h2>
      <span class="text-xs text-muted-foreground">{{ d.projekte.length }}</span>
    </div>
    <p v-if="!d.projekte.length" class="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
      Noch kein Projekt. Mit „Neues Projekt“ beginnt ein Auftrag (Angebot, Rechnung, Aufmaß …).
    </p>
    <ProjektKarte
      v-for="p in d.projekte"
      :key="p.id"
      :projekt="p"
      :offen="offen.has(p.id)"
      @umschalten="umschalten(p.id)"
      @geaendert="laden(); emit('geaendert')"
    />

    <!-- Briefe -->
    <template v-if="briefe.length">
      <h2 class="mt-2 text-sm font-semibold">Briefe</h2>
      <div class="overflow-hidden rounded-xl border border-border bg-card">
        <button
          v-for="b in briefe"
          :key="b.id"
          class="flex w-full items-baseline gap-3 border-b border-border/60 px-4 py-2 text-left last:border-0 hover:bg-foreground/4"
          @click="nav.briefId = b.id"
        >
          <span class="w-20 shrink-0 text-xs text-muted-foreground tabular-nums">{{ datum(b.datum) }}</span>
          <span class="shrink-0 text-[13px] font-medium">{{ b.betreff || "(ohne Betreff)" }}</span>
          <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">{{ htmlText(b.text).slice(0, 160) }}</span>
        </button>
      </div>
    </template>

    <KundeForm v-if="bearbeiten" :kunde="d.kunde" @close="bearbeiten = false" @saved="gespeichert" />
    <ProjektForm v-if="projektNeu" :projekt="projektNeu" @close="projektNeu = null" @saved="projektAngelegt" />
  </div>
</template>
