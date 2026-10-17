<script setup lang="ts">
import { ref } from "vue";
import { DropdownMenuContent, DropdownMenuItem, DropdownMenuPortal, DropdownMenuRoot, DropdownMenuTrigger } from "reka-ui";
import { TextQuote } from "@lucide/vue";
import { api } from "@/lib/api";
import { htmlText } from "@/lib/format";
import type { Textbaustein } from "@/lib/types";

defineProps<{ kategorie?: string }>();
const emit = defineEmits<{ waehlen: [html: string] }>();

const bausteine = ref<Textbaustein[]>([]);
async function laden(offen: boolean) {
  if (offen) bausteine.value = await api.textbausteine().catch(() => []);
}
</script>

<template>
  <DropdownMenuRoot @update:open="laden">
    <DropdownMenuTrigger as-child>
      <button class="btn btn-ghost h-6 px-1.5 text-[11px]"><TextQuote />Textbaustein</button>
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent align="end" :side-offset="4" class="menu anim-fade max-h-96 w-96 overflow-y-auto">
        <p v-if="!bausteine.length" class="px-2 py-2 text-xs text-muted-foreground">Keine Textbausteine (Einstellungen → Texte).</p>
        <DropdownMenuItem v-for="b in bausteine" :key="b.id" class="menu-item h-auto flex-col items-start gap-0 py-1.5" @select="emit('waehlen', b.text)">
          <span class="text-[13px]">{{ b.name }}</span>
          <span class="line-clamp-2 text-[11px] text-muted-foreground">{{ htmlText(b.text) }}</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>
