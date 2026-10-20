<script setup lang="ts">
import { betrag, zahl } from "@/lib/format";
import type { Beleg, Summen } from "@/lib/types";

defineProps<{ beleg: Beleg; summen: Summen | null }>();
</script>

<template>
  <div v-if="summen" class="rounded-lg border border-border bg-card p-3 text-[13px]">
    <div class="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 tabular-nums">
      <template v-if="summen.rabatt || summen.zuschlaege_netto.length">
        <span class="text-muted-foreground">Positionen</span><span class="text-right">{{ betrag(summen.positionen) }}</span>
        <template v-if="summen.rabatt">
          <span class="text-muted-foreground">{{ beleg.rabatt_text }} {{ zahl(beleg.rabatt_prozent, 2) }} %</span>
          <span class="text-right">−{{ betrag(summen.rabatt) }}</span>
        </template>
        <template v-for="(z, i) in summen.zuschlaege_netto" :key="'n' + i">
          <span class="truncate text-muted-foreground">{{ z.text }}</span><span class="text-right">{{ betrag(z.betrag) }}</span>
        </template>
      </template>
      <span class="text-muted-foreground">Netto</span><span class="text-right">{{ betrag(summen.netto) }}</span>
      <span class="text-muted-foreground">MwSt. {{ zahl(summen.mwst_satz, 2) }} %</span><span class="text-right">{{ betrag(summen.mwst) }}</span>
      <span class="border-t border-border pt-1 font-semibold">Brutto</span>
      <span class="border-t border-border pt-1 text-right font-semibold">{{ betrag(summen.brutto) }}</span>
      <template v-for="(z, i) in summen.zuschlaege_brutto" :key="'b' + i">
        <span class="truncate text-muted-foreground">{{ z.text }}</span><span class="text-right">{{ betrag(z.betrag) }}</span>
      </template>
      <template v-if="summen.abzug">
        <span class="text-muted-foreground">abzgl. Abschläge</span><span class="text-right">−{{ betrag(summen.abzug) }}</span>
      </template>
      <template v-if="summen.abzug || summen.zuschlaege_brutto.length">
        <span class="border-t border-border pt-1 font-semibold">Zu zahlen</span>
        <span class="border-t border-border pt-1 text-right font-semibold">{{ betrag(summen.zahlbetrag) }}</span>
      </template>
    </div>
    <p v-if="beleg.lohn_ausweisen && summen.lohn_netto" class="mt-2 text-[11px] text-muted-foreground">
      Lohnanteil § 35a: {{ betrag(summen.lohn_netto) }} + {{ betrag(summen.lohn_mwst) }} MwSt.
    </p>
    <p class="mt-1 text-right text-[11px] text-muted-foreground">{{ beleg.waehrung }}</p>
  </div>
</template>
