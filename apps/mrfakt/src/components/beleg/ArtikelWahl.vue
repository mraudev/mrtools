<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { Search } from "@lucide/vue";
import Dialog from "@mrtools/ui/components/Dialog";
import { api } from "@/lib/api";
import { betrag, kurztext } from "@/lib/format";
import type { Artikel } from "@/lib/types";

const emit = defineEmits<{ close: []; waehlen: [a: Artikel] }>();
const offen = ref(true);
const suche = ref("");
const liste = ref<Artikel[]>([]);
const aktiv = ref(0);

async function laden() {
  liste.value = await api.artikel(suche.value).catch(() => []);
  aktiv.value = 0;
}
let timer: ReturnType<typeof setTimeout> | undefined;
watch(suche, () => {
  clearTimeout(timer);
  timer = setTimeout(laden, 150);
});
onMounted(laden);

function tasten(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    aktiv.value = Math.min(aktiv.value + 1, liste.value.length - 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    aktiv.value = Math.max(aktiv.value - 1, 0);
  } else if (e.key === "Enter" && liste.value[aktiv.value]) {
    emit("waehlen", liste.value[aktiv.value]);
  }
}
</script>

<template>
  <Dialog v-model:open="offen" title="Artikel übernehmen" size="lg" @update:open="(v: boolean) => !v && emit('close')">
    <div class="relative mb-2">
      <Search class="pointer-events-none absolute top-2 left-2.5 size-4 text-muted-foreground" />
      <input v-model="suche" class="input pl-8" placeholder="Suchtext, Nummer, Lieferant …" autofocus @keydown="tasten" />
    </div>
    <div class="h-[50vh] overflow-y-auto rounded-md border border-border">
      <button
        v-for="(a, i) in liste"
        :key="a.id"
        class="flex w-full items-center gap-3 border-b border-border/60 px-3 py-1.5 text-left"
        :class="i === aktiv ? 'bg-accent/12' : 'hover:bg-foreground/4'"
        @mouseenter="aktiv = i"
        @click="emit('waehlen', a)"
      >
        <span class="w-16 shrink-0 text-xs text-muted-foreground">{{ a.nummer }}</span>
        <span class="min-w-0 flex-1 truncate text-[13px]">{{ kurztext(a.text) || a.suchtext }}</span>
        <span v-if="a.lager_aktiv" class="text-[11px] text-muted-foreground">Lager {{ a.bestand }}</span>
        <span class="w-28 shrink-0 text-right text-[13px] tabular-nums">{{ betrag(a.vk1) }} € / {{ a.einheit }}</span>
      </button>
      <p v-if="!liste.length" class="p-4 text-center text-xs text-muted-foreground">Keine Artikel gefunden.</p>
    </div>
  </Dialog>
</template>
