<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { api } from "@/lib/api";
import { betrag } from "@/lib/format";
import { nav } from "@/lib/nav";
import type { Umsatz } from "@/lib/types";
import { toastError } from "@mrtools/ui/lib/toast";

const jahr = ref(String(new Date().getFullYear()));
const u = ref<Umsatz>();
const monate = ["Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"];

async function laden() {
  try {
    u.value = await api.umsatz(jahr.value);
    if (!u.value.monate.length && u.value.jahre.length && !u.value.jahre.some(([j]) => j === jahr.value)) {
      jahr.value = u.value.jahre[u.value.jahre.length - 1][0];
    }
  } catch (e) {
    toastError("Auswertung konnte nicht geladen werden", e);
  }
}
watch([jahr, () => nav.version], laden);
onMounted(laden);

const balken = computed(() => {
  const werte = monate.map((_, i) => u.value?.monate.find(([m]) => m === i + 1)?.[1] ?? 0);
  const max = Math.max(1, ...werte);
  return werte.map((w) => ({ w, h: Math.max(0, (w / max) * 100) }));
});
const jahresBalken = computed(() => {
  const j = (u.value?.jahre ?? []).slice(-15);
  const max = Math.max(1, ...j.map(([, w]) => w));
  return j.map(([n, w]) => ({ n, w, h: Math.max(0, (w / max) * 100) }));
});
const kundenMax = computed(() => Math.max(1, ...(u.value?.kunden ?? []).map(([, w]) => w)));
</script>

<template>
  <div class="min-h-0 flex-1 overflow-y-auto">
    <div v-if="u" class="mx-auto flex max-w-6xl flex-col gap-4 p-5">
      <div class="flex items-center gap-3">
        <h1 class="text-[15px] font-semibold">Auswertung</h1>
        <select v-model="jahr" class="input h-8 w-28">
          <option v-for="[j] in [...u.jahre].reverse()" :key="j" :value="j">{{ j }}</option>
        </select>
        <span class="text-xs text-muted-foreground">Umsatz = Rechnungen und Schlussrechnungen abzüglich Gutschriften, netto</span>
      </div>
      <div class="grid grid-cols-4 gap-3">
        <div class="rounded-xl border border-border bg-card p-4">
          <div class="text-xs text-muted-foreground">Umsatz netto {{ jahr }}</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums">{{ betrag(u.netto) }} €</div>
        </div>
        <div class="rounded-xl border border-border bg-card p-4">
          <div class="text-xs text-muted-foreground">Umsatzsteuer</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums">{{ betrag(u.mwst) }} €</div>
        </div>
        <div class="rounded-xl border border-border bg-card p-4">
          <div class="text-xs text-muted-foreground">Offene Posten {{ jahr }}</div>
          <div class="mt-1 text-2xl font-semibold text-amber-600 tabular-nums dark:text-amber-400">{{ betrag(u.offen) }} €</div>
        </div>
        <div class="rounded-xl border border-border bg-card p-4">
          <div class="text-xs text-muted-foreground">Angebote {{ jahr }}</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums">{{ u.angebote_anzahl }}</div>
          <div class="text-xs text-muted-foreground tabular-nums">{{ betrag(u.angebote) }} € netto</div>
        </div>
      </div>
      <div class="grid grid-cols-[2fr_1fr] gap-3">
        <div class="rounded-xl border border-border bg-card p-4">
          <div class="mb-3 text-sm font-medium">Umsatz je Monat</div>
          <div class="flex h-56 items-end gap-2 border-b border-[var(--chart-baseline)]">
            <div v-for="(b, i) in balken" :key="i" class="group relative flex h-full flex-1 flex-col justify-end">
              <div class="rounded-t-sm bg-[var(--series-1)] transition-opacity group-hover:opacity-80" :style="{ height: b.h + '%' }" />
              <div class="pointer-events-none absolute -top-5 left-1/2 hidden -translate-x-1/2 rounded bg-popover px-1.5 text-[11px] whitespace-nowrap shadow group-hover:block">{{ betrag(b.w) }} €</div>
            </div>
          </div>
          <div class="mt-1 flex gap-2 text-center text-[11px] text-muted-foreground">
            <span v-for="m in monate" :key="m" class="flex-1">{{ m }}</span>
          </div>
        </div>
        <div class="rounded-xl border border-border bg-card p-4">
          <div class="mb-3 text-sm font-medium">Größte Kunden {{ jahr }}</div>
          <div v-for="[name, w] in u.kunden" :key="name" class="mb-1.5">
            <div class="flex justify-between text-xs"><span class="truncate">{{ name }}</span><span class="tabular-nums">{{ betrag(w) }}</span></div>
            <div class="h-1.5 rounded bg-foreground/6"><div class="h-1.5 rounded bg-[var(--series-1)]" :style="{ width: (w / kundenMax) * 100 + '%' }" /></div>
          </div>
          <p v-if="!u.kunden.length" class="text-xs text-muted-foreground">Keine Umsätze.</p>
        </div>
      </div>
      <div class="rounded-xl border border-border bg-card p-4">
        <div class="mb-3 text-sm font-medium">Umsatz je Jahr</div>
        <div class="flex h-40 items-end gap-2 border-b border-[var(--chart-baseline)]">
          <button v-for="b in jahresBalken" :key="b.n" class="group relative flex h-full flex-1 flex-col justify-end" @click="jahr = b.n">
            <div class="rounded-t-sm transition-opacity group-hover:opacity-80" :class="b.n === jahr ? 'bg-[var(--series-2)]' : 'bg-[var(--series-1)]'" :style="{ height: b.h + '%' }" />
            <div class="pointer-events-none absolute -top-5 left-1/2 hidden -translate-x-1/2 rounded bg-popover px-1.5 text-[11px] whitespace-nowrap shadow group-hover:block">{{ betrag(b.w) }} €</div>
          </button>
        </div>
        <div class="mt-1 flex gap-2 text-center text-[11px] text-muted-foreground">
          <span v-for="b in jahresBalken" :key="b.n" class="flex-1">{{ b.n }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
