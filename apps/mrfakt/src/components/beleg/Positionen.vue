<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "reka-ui";
import { ArrowDown, ArrowUp, Box, Calculator, Copy, EllipsisVertical, Plus, Ruler, Trash2 } from "@lucide/vue";
import ZahlFeld from "../ZahlFeld.vue";
import { betrag } from "@/lib/format";
import type { Position, PosArt, Summen } from "@/lib/types";

const positionen = defineModel<Position[]>({ required: true });
const props = defineProps<{ gesperrt: boolean; einheiten: string[]; summen: Summen | null; preise: boolean; waehrung: string }>();
const emit = defineEmits<{ kalkulation: [index: number]; artikel: [index: number]; aufmass: [] }>();

const arten: { value: PosArt; label: string; kurz: string }[] = [
  { value: "N", label: "Position", kurz: "" },
  { value: "A", label: "Alternativposition", kurz: "Alt." },
  { value: "B", label: "Bedarfsposition", kurz: "Bed." },
  { value: "T", label: "Titel", kurz: "Titel" },
  { value: "G", label: "Gewerk (Obertitel)", kurz: "Gewerk" },
  { value: "K", label: "Text", kurz: "Text" },
  { value: "S", label: "Zwischensumme", kurz: "Σ" },
];
const kurz = Object.fromEntries(arten.map((a) => [a.value, a.kurz]));

const tabelle = ref<HTMLElement>();
// Stabile Schlüssel für die Zeilen, auch für neue Positionen ohne Id
const schluessel = new WeakMap<Position, number>();
let zaehler = 0;
function key(p: Position) {
  if (!schluessel.has(p)) schluessel.set(p, ++zaehler);
  return schluessel.get(p)!;
}
const titelSumme = computed(() => new Map(props.summen?.titel ?? []));
const zwischen = computed(() => new Map(props.summen?.zwischen ?? []));

function neuePosNr(index: number): string {
  // Nächste Nummer nach der letzten nummerierten Position davor
  for (let i = index - 1; i >= 0; i--) {
    const nr = positionen.value[i]?.posnr;
    if (!nr) continue;
    const m = /^(.*?)(\d+)$/.exec(nr);
    if (m) return m[1] + String(Number(m[2]) + 1).padStart(m[2].length, "0");
  }
  return String(positionen.value.filter((p) => p.art === "N").length + 1);
}

function leer(art: PosArt = "N", index = positionen.value.length): Position {
  return {
    id: 0, beleg_id: 0, pos: 0, art, posnr: art === "N" ? neuePosNr(index) : "", text: "", menge: art === "N" ? 1 : 0, einheit: "", ep: 0, rabatt: 0, gp: 0, lohn: 0,
    artikel_id: null, menge_aus_aufmass: false,
  };
}

function einfuegen(index: number, art: PosArt = "N") {
  positionen.value.splice(index, 0, leer(art, index));
  nextTick(() => fokus(index));
}

function fokus(index: number) {
  const el = tabelle.value?.querySelectorAll<HTMLTextAreaElement>("textarea[data-text]")[index];
  el?.focus();
}

function verschieben(index: number, richtung: number) {
  const ziel = index + richtung;
  if (ziel < 0 || ziel >= positionen.value.length) return;
  const [p] = positionen.value.splice(index, 1);
  positionen.value.splice(ziel, 0, p);
}

function duplizieren(index: number) {
  const p = positionen.value[index];
  positionen.value.splice(index + 1, 0, { ...p, id: 0, posnr: p.art === "N" ? neuePosNr(index + 1) : p.posnr });
}

function entfernen(index: number) {
  positionen.value.splice(index, 1);
}

function gp(p: Position): number {
  const roh = Math.round(p.menge * p.ep * 100) / 100;
  return p.rabatt ? roh - Math.round(roh * p.rabatt) / 100 : roh;
}

function zeilen(text: string) {
  return Math.min(12, Math.max(1, text.split("\n").length));
}

/** Enter im Preisfeld der letzten Zeile legt eine neue Position an. */
function weiter(index: number) {
  if (index === positionen.value.length - 1) einfuegen(index + 1);
  else fokus(index + 1);
}

// Drag & Drop zum Umsortieren
const ziehen = ref<number | null>(null);
function dragStart(i: number, e: DragEvent) {
  ziehen.value = i;
  e.dataTransfer?.setData("text/plain", String(i));
}
function drop(i: number) {
  if (ziehen.value === null || ziehen.value === i) return;
  const [p] = positionen.value.splice(ziehen.value, 1);
  positionen.value.splice(i, 0, p);
  ziehen.value = null;
}
</script>

<template>
  <div ref="tabelle" class="rounded-xl border border-border bg-card">
    <div class="grid grid-cols-[2.2rem_4.2rem_1fr_6.5rem_4.5rem_6.5rem_4rem_7rem_2rem] gap-x-1.5 border-b border-border px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
      <span />
      <span>Pos.</span>
      <span>Leistung (erste Zeile = Kurztext)</span>
      <span class="text-right">Menge</span>
      <span>Einheit</span>
      <span class="text-right">EP {{ waehrung }}</span>
      <span class="text-right">Rab. %</span>
      <span class="text-right">GP {{ waehrung }}</span>
      <span />
    </div>
    <div
      v-for="(p, i) in positionen"
      :key="key(p)"
      class="group grid grid-cols-[2.2rem_4.2rem_1fr_6.5rem_4.5rem_6.5rem_4rem_7rem_2rem] items-start gap-x-1.5 border-b border-border/50 px-2 py-1"
      :class="[p.art === 'T' || p.art === 'G' ? 'bg-foreground/[0.03]' : '', ziehen === i ? 'opacity-40' : '']"
      @dragover.prevent
      @drop="drop(i)"
    >
      <div class="flex flex-col items-center pt-1">
        <span
          class="cursor-grab rounded px-1 text-[10px] font-semibold text-muted-foreground select-none"
          :class="p.art !== 'N' ? 'bg-foreground/8' : ''"
          :draggable="!gesperrt"
          :title="arten.find((a) => a.value === p.art)?.label"
          @dragstart="dragStart(i, $event)"
          @dragend="ziehen = null"
        >{{ kurz[p.art] || i + 1 }}</span>
      </div>
      <input v-model="p.posnr" class="input h-7 px-1.5 text-xs" :disabled="gesperrt || p.art === 'K' || p.art === 'S'" />
      <textarea
        v-model="p.text"
        data-text
        class="input h-auto min-h-7 resize-none px-2 py-1 text-[13px] leading-snug"
        :class="p.art === 'T' || p.art === 'G' ? 'font-semibold' : ''"
        :rows="zeilen(p.text)"
        :disabled="gesperrt"
        :placeholder="p.art === 'S' ? 'Zwischensumme' : p.art === 'T' ? 'Titel' : 'Kurztext\nLangtext …'"
        spellcheck="true"
      />
      <template v-if="p.art === 'N' || p.art === 'A' || p.art === 'B'">
        <div class="relative">
          <ZahlFeld v-model="p.menge" class="h-7 px-1.5 text-[13px]" :disabled="gesperrt" @update:model-value="p.menge_aus_aufmass = false" />
          <Ruler v-if="p.menge_aus_aufmass" class="absolute top-2 left-1 size-3 text-accent-text" title="Menge aus dem Aufmaß" />
        </div>
        <input v-model="p.einheit" class="input h-7 px-1.5 text-[13px]" list="einheiten" :disabled="gesperrt" />
        <ZahlFeld v-model="p.ep" class="h-7 px-1.5 text-[13px]" :disabled="gesperrt || !preise" @enter="weiter(i)" />
        <ZahlFeld v-model="p.rabatt" class="h-7 px-1.5 text-[13px]" :stellen="0" :max-stellen="2" :disabled="gesperrt" />
        <span class="pt-1.5 text-right text-[13px] tabular-nums" :class="p.art !== 'N' ? 'text-muted-foreground italic' : ''">
          {{ p.art !== "N" ? "(" + betrag(gp(p)) + ")" : betrag(gp(p)) }}
        </span>
      </template>
      <template v-else>
        <span class="col-span-4" />
        <span class="pt-1.5 text-right text-[13px] font-semibold tabular-nums">
          <template v-if="p.art === 'T' && titelSumme.get(i) !== undefined">{{ betrag(titelSumme.get(i)) }}</template>
          <template v-else-if="p.art === 'S' && zwischen.get(i) !== undefined">{{ betrag(zwischen.get(i)) }}</template>
        </span>
      </template>
      <DropdownMenuRoot v-if="!gesperrt">
        <DropdownMenuTrigger as-child>
          <button class="icon-btn mt-0.5 opacity-0 group-hover:opacity-100 data-[state=open]:opacity-100" aria-label="Zeilenaktionen"><EllipsisVertical /></button>
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent align="end" :side-offset="4" class="menu anim-fade w-60">
            <DropdownMenuItem v-for="a in arten" :key="a.value" class="menu-item" :class="p.art === a.value ? 'font-semibold' : ''" @select="p.art = a.value">
              <span class="w-4 text-center text-[10px]">{{ p.art === a.value ? "●" : "" }}</span>{{ a.label }}
            </DropdownMenuItem>
            <DropdownMenuSeparator class="menu-separator" />
            <DropdownMenuItem v-if="p.art === 'N' || p.art === 'A' || p.art === 'B'" class="menu-item" @select="emit('kalkulation', i)"><Calculator />Kalkulation …</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" @select="emit('artikel', i)"><Box />Artikel übernehmen …</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" @select="einfuegen(i)"><Plus />Position darüber einfügen</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" @select="duplizieren(i)"><Copy />Duplizieren</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" :disabled="i === 0" @select="verschieben(i, -1)"><ArrowUp />Nach oben</DropdownMenuItem>
            <DropdownMenuItem class="menu-item" :disabled="i === positionen.length - 1" @select="verschieben(i, 1)"><ArrowDown />Nach unten</DropdownMenuItem>
            <DropdownMenuSeparator class="menu-separator" />
            <DropdownMenuItem class="menu-item text-red-500" @select="entfernen(i)"><Trash2 />Entfernen</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
      <span v-else />
    </div>
    <div v-if="!gesperrt" class="flex flex-wrap items-center gap-1 px-2 py-1.5">
      <button class="btn btn-ghost h-7 px-2 text-xs" @click="einfuegen(positionen.length)"><Plus />Position</button>
      <button class="btn btn-ghost h-7 px-2 text-xs" @click="einfuegen(positionen.length, 'T')"><Plus />Titel</button>
      <button class="btn btn-ghost h-7 px-2 text-xs" @click="einfuegen(positionen.length, 'K')"><Plus />Text</button>
      <button class="btn btn-ghost h-7 px-2 text-xs" @click="emit('artikel', positionen.length)"><Box />Artikel</button>
      <div class="flex-1" />
      <button class="btn btn-ghost h-7 px-2 text-xs" @click="emit('aufmass')"><Ruler />Aufmaß</button>
    </div>
    <datalist id="einheiten"><option v-for="e in einheiten" :key="e" :value="e" /></datalist>
  </div>
</template>
