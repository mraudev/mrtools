<script setup lang="ts">
import { ref } from "vue";
import { ContextMenuContent, ContextMenuItem, ContextMenuPortal, ContextMenuRoot, ContextMenuTrigger } from "reka-ui";
import { Copy, Folder, Monitor, Plus, X } from "@lucide/vue";
import Tip from "@mrtools/ui/components/Tip";
import { activateTab, closeTab, openTab, tabPath, tabs, tabTitle, type Tab } from "@/lib/tabs";

const menuTab = ref<Tab>();

async function closeOthers(keep: Tab) {
  await activateTab(keep.id);
  tabs.list = tabs.list.filter((t) => t.id === keep.id);
}

// Middle click closes a tab, like in a browser.
function onAuxclick(event: MouseEvent, tab: Tab) {
  if (event.button === 1) closeTab(tab.id);
}
</script>

<template>
  <ContextMenuRoot>
    <ContextMenuTrigger as-child>
      <div class="flex min-w-0 items-center gap-1" role="tablist" aria-label="Tabs" @contextmenu.capture="menuTab = undefined">
        <div
          v-for="tab in tabs.list"
          :key="tab.id"
          role="tab"
          :aria-selected="tab.id === tabs.active"
          class="group flex h-7 max-w-48 min-w-16 cursor-default items-center gap-1.5 rounded-md pr-1 pl-2 text-[13px] transition-colors"
          :class="tab.id === tabs.active ? 'bg-accent/15 font-medium text-accent-text' : 'text-foreground/75 hover:bg-foreground/5'"
          :title="tabPath(tab) || 'Dieser PC'"
          @mousedown.left="activateTab(tab.id)"
          @mousedown.middle.prevent
          @auxclick="onAuxclick($event, tab)"
          @contextmenu="menuTab = tab"
        >
          <Folder v-if="tabPath(tab)" class="size-3.5 shrink-0" fill="currentColor" fill-opacity="0.2" />
          <Monitor v-else class="size-3.5 shrink-0" />
          <span class="min-w-0 flex-1 truncate">{{ tabTitle(tab) }}</span>
          <button
            v-if="tabs.list.length > 1"
            class="grid size-5 shrink-0 place-items-center rounded opacity-0 group-hover:opacity-100 hover:bg-foreground/10"
            :class="tab.id === tabs.active && 'opacity-60'"
            aria-label="Tab schließen"
            @mousedown.stop
            @click="closeTab(tab.id)"
          >
            <X class="size-3" />
          </button>
        </div>
        <Tip text="Neuer Tab (Strg+T)" side="bottom">
          <button class="icon-btn size-7 shrink-0" aria-label="Neuer Tab" @click="openTab('')"><Plus /></button>
        </Tip>
      </div>
    </ContextMenuTrigger>
    <ContextMenuPortal>
      <ContextMenuContent v-if="menuTab" class="menu anim-pop">
        <ContextMenuItem class="menu-item" @select="openTab(tabPath(menuTab))"><Copy />Tab duplizieren</ContextMenuItem>
        <ContextMenuItem class="menu-item" :disabled="tabs.list.length < 2" @select="closeTab(menuTab.id)">
          <X />Tab schließen<span class="kbd ml-auto">Strg+W</span>
        </ContextMenuItem>
        <ContextMenuItem class="menu-item" :disabled="tabs.list.length < 2" @select="closeOthers(menuTab)">
          Andere Tabs schließen
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenuPortal>
  </ContextMenuRoot>
</template>
