<script setup lang="ts">
import { ref, watch } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "reka-ui";
import { ChevronRight, ClipboardList, Clock, EllipsisVertical, FileInput, FilePlus2, Pencil, Trash2 } from "@lucide/vue";
import { open } from "@tauri-apps/plugin-dialog";
import ProjektForm from "./ProjektForm.vue";
import RegieDialog from "../regie/RegieDialog.vue";
import ZeitenDialog from "../regie/ZeitenDialog.vue";
import BelegBadge from "../beleg/BelegBadge.vue";
import { api } from "@/lib/api";
import { artTitel, betrag, datum, istRechnung } from "@/lib/format";
import { frage, nav, oeffneBeleg } from "@/lib/nav";
import type { BelegKurz, ProjektZeile } from "@/lib/types";
import { toastError } from "@mrtools/ui/lib/toast";

const props = defineProps<{ projekt: ProjektZeile; offen: boolean }>();
const emit = defineEmits<{ umschalten: []; geaendert: [] }>();

const belege = ref<BelegKurz[]>([]);
const bearbeiten = ref(false);
const regie = ref(false);
const zeiten = ref(false);

async function laden() {
  try {
    belege.value = await api.belegeProjekt(props.projekt.id);
  } catch (e) {
    toastError("Belege konnten nicht geladen werden", e);
  }
}
watch(
  () => [props.offen, nav.version],
  () => {
    if (props.offen) laden();
  },
  { immediate: true },
);

const arten = ["angebot", "auftrag", "lieferschein", "abschlag", "schluss", "rechnung", "gutschrift", "aufstellung"];

async function neuerBeleg(art: string) {
  try {
    const v = await api.belegNeu(props.projekt.id, art);
    nav.projektId = props.projekt.id;
    await oeffneBeleg(v.beleg.id);
  } catch (e) {
    toastError("Beleg konnte nicht angelegt werden", e);
  }
}

async function gaeb() {
  const pfad = await open({ filters: [{ name: "GAEB", extensions: ["x81", "x82", "x83", "x84", "x86", "d81", "d83", "d84", "p83", "xml"] }] });
  if (typeof pfad !== "string") return;
  try {
    const id = await api.gaebImport(props.projekt.id, pfad);
    await oeffneBeleg(id);
  } catch (e) {
    toastError("GAEB-Datei konnte nicht gelesen werden", e);
  }
}

async function loeschen() {
  if (!(await frage("Projekt löschen", `Projekt ${props.projekt.nummer} mit allen Belegen löschen?`, "Löschen"))) return;
  try {
    await api.projektLoeschen(props.projekt.id);
    emit("geaendert");
  } catch (e) {
    toastError("Löschen nicht möglich", e);
  }
}

function zahlstatus(b: BelegKurz): { text: string; klasse: string } | null {
  if (!istRechnung(b.art) || b.status === "entwurf" || b.status === "storniert") return null;
  const offen = b.zahlbetrag - b.bezahlt;
  if (Math.abs(offen) < 0.005) return { text: "bezahlt", klasse: "text-emerald-600 dark:text-emerald-400" };
  return { text: `${betrag(offen)} offen`, klasse: "text-amber-600 dark:text-amber-400" };
}
</script>

<template>
  <div class="overflow-hidden rounded-xl border border-border bg-card" :class="nav.projektId === projekt.id ? 'ring-1 ring-accent/40' : ''">
    <div class="flex items-center gap-2 px-3 py-2">
      <button class="flex min-w-0 flex-1 items-center gap-2 text-left" @click="emit('umschalten')">
        <ChevronRight class="size-4 shrink-0 text-muted-foreground transition-transform" :class="offen ? 'rotate-90' : ''" />
        <span class="shrink-0 text-xs text-muted-foreground tabular-nums">{{ projekt.nummer }}</span>
        <span class="min-w-0 truncate text-[13px] font-medium">{{ projekt.bezeichnung || "(ohne Bezeichnung)" }}</span>
        <span v-if="projekt.status === 'offen'" class="shrink-0 rounded bg-sky-500/12 px-1.5 text-[10px] font-medium text-sky-700 dark:text-sky-300">offen</span>
      </button>
      <span class="shrink-0 text-xs text-muted-foreground tabular-nums">{{ datum(projekt.letzter || projekt.datum) }}</span>
      <span class="w-24 shrink-0 text-right text-xs tabular-nums">{{ projekt.summe ? betrag(projekt.summe) + " €" : "" }}</span>
      <DropdownMenuRoot>
        <DropdownMenuTrigger as-child>
          <button class="btn btn-ghost h-7 px-2 text-xs"><FilePlus2 />Neuer Beleg</button>
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent align="end" :side-offset="4" class="menu anim-fade w-56">
            <DropdownMenuItem v-for="a in arten" :key="a" class="menu-item" @select="neuerBeleg(a)">{{ artTitel[a] }}</DropdownMenuItem>
            <DropdownMenuSeparator class="menu-separator" />
            <DropdownMenuItem class="menu-item" @select="gaeb"><FileInput />Angebot aus GAEB-Datei …</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
      <DropdownMenuRoot>
        <DropdownMenuTrigger as-child>
          <button class="icon-btn" aria-label="Projektaktionen"><EllipsisVertical /></button>
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent align="end" :side-offset="4" class="menu anim-fade w-56">
            <DropdownMenuItem class="menu-item" @select="regie = true"><ClipboardList />Regieberichte</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" @select="zeiten = true"><Clock />Zeiten & Nachkalkulation</DropdownMenuItem>
            <DropdownMenuSeparator class="menu-separator" />
            <DropdownMenuItem class="menu-item" @select="bearbeiten = true"><Pencil />Projekt bearbeiten</DropdownMenuItem>
            <DropdownMenuItem class="menu-item text-red-500" @select="loeschen"><Trash2 />Projekt löschen</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
    </div>
    <div v-if="offen" class="border-t border-border">
      <p v-if="projekt.baustelle || projekt.notiz" class="px-4 pt-2 text-xs whitespace-pre-line text-muted-foreground">
        {{ [projekt.baustelle && "Baustelle: " + projekt.baustelle, projekt.notiz].filter(Boolean).join("\n") }}
      </p>
      <p v-if="!belege.length" class="px-4 py-3 text-xs text-muted-foreground">Noch keine Belege.</p>
      <button
        v-for="b in belege"
        :key="b.id"
        class="flex w-full items-center gap-3 border-b border-border/50 px-4 py-1.5 text-left last:border-0 hover:bg-foreground/4"
        @click="nav.projektId = projekt.id; oeffneBeleg(b.id)"
      >
        <span class="w-20 shrink-0 text-xs text-muted-foreground tabular-nums">{{ datum(b.datum) }}</span>
        <span class="min-w-0 flex-1 truncate text-[13px]">{{ b.titel }} <span class="text-muted-foreground">{{ b.nummer }}</span></span>
        <BelegBadge :status="b.status" />
        <span class="w-28 shrink-0 text-right text-[11px]" :class="zahlstatus(b)?.klasse">{{ zahlstatus(b)?.text }}</span>
        <span class="w-24 shrink-0 text-right text-[13px] tabular-nums">{{ betrag(b.netto) }}{{ b.waehrung === "DM" ? " DM" : "" }}</span>
      </button>
    </div>
    <ProjektForm v-if="bearbeiten" :projekt="projekt" @close="bearbeiten = false" @saved="bearbeiten = false; emit('geaendert')" />
    <RegieDialog v-if="regie" :projekt-id="projekt.id" @close="regie = false; emit('geaendert')" />
    <ZeitenDialog v-if="zeiten" :projekt-id="projekt.id" @close="zeiten = false" />
  </div>
</template>
