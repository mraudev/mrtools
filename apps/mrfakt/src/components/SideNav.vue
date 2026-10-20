<script setup lang="ts">
import { BarChart3, BookOpenCheck, Box, Settings, Users } from "@lucide/vue";
import { nav, zeigeSeite, type Seite } from "@/lib/nav";

const eintraege: { seite: Seite; text: string; icon: unknown }[] = [
  { seite: "kunden", text: "Kunden & Projekte", icon: Users },
  { seite: "rechnungsbuch", text: "Rechnungsbuch", icon: BookOpenCheck },
  { seite: "auswertung", text: "Auswertung", icon: BarChart3 },
  { seite: "artikel", text: "Artikel & Lager", icon: Box },
];
</script>

<template>
  <nav class="flex w-52 shrink-0 flex-col gap-0.5 border-r border-border bg-card p-2">
    <button
      v-for="e in eintraege"
      :key="e.seite"
      class="flex h-9 items-center gap-2.5 rounded-md px-2.5 text-[13px] transition-colors"
      :class="nav.seite === e.seite && !nav.belegId && !nav.briefId ? 'bg-accent/15 font-medium text-accent-text' : 'text-foreground/80 hover:bg-foreground/5'"
      @click="zeigeSeite(e.seite)"
    >
      <component :is="e.icon" class="size-4" />{{ e.text }}
    </button>
    <div class="flex-1" />
    <button
      class="flex h-9 items-center gap-2.5 rounded-md px-2.5 text-[13px] transition-colors"
      :class="nav.seite === 'einstellungen' && !nav.belegId ? 'bg-accent/15 font-medium text-accent-text' : 'text-foreground/80 hover:bg-foreground/5'"
      @click="zeigeSeite('einstellungen')"
    >
      <Settings class="size-4" />Einstellungen
    </button>
  </nav>
</template>
