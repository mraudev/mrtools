<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "reka-ui";
import {
  CalendarClock,
  Check,
  Flame,
  House,
  Lightbulb,
  LoaderCircle,
  LogOut,
  MapPin,
  Plane,
  PowerOff,
  RefreshCw,
  Settings,
  Thermometer,
  Unlink,
} from "@lucide/vue";
import logo from "@/assets/logo.svg";
import Tip from "@mrtools/ui/components/Tip";
import WindowControls from "@mrtools/ui/components/WindowControls";
import { callsPerDay, sameTermination, TERMINATIONS } from "@/lib/logic";
import { lights, loadHue, scanGovee, unpairHue } from "@/lib/lights";
import { chooseHome, logout, quickAction, refresh, setPresence, settings, state } from "@/lib/store";
import { ui } from "@/lib/ui";

const presence = computed(() => (!state.homeState ? null : state.homeState.locked ? state.homeState.presence : "AUTO"));
const presenceOptions = [
  { value: "HOME" as const, label: "Zuhause", icon: House },
  { value: "AWAY" as const, label: "Abwesend", icon: Plane },
  { value: "AUTO" as const, label: "Automatisch", icon: MapPin },
];
const intervals = [0, 15, 30, 60, 120];

function refreshLights() {
  if (lights.hue.bridge) loadHue();
  scanGovee();
}

function onKeydown(event: KeyboardEvent) {
  if (event.key !== "F5") return;
  event.preventDefault();
  if (ui.tab === "lights") refreshLights();
  else if (state.phase === "ready") refresh();
}

const tabs = [
  { value: "lights" as const, label: "Licht", icon: Lightbulb },
  { value: "heating" as const, label: "Heizung", icon: Thermometer },
];

onMounted(() => window.addEventListener("keydown", onKeydown));

onUnmounted(() => window.removeEventListener("keydown", onKeydown));

const menuItem = "menu-item";
</script>

<template>
  <header class="flex h-11 shrink-0 items-stretch border-b border-border bg-card select-none" data-tauri-drag-region>
    <div class="flex items-center gap-2 pr-4 pl-3.5" data-tauri-drag-region>
      <img :src="logo" alt="" class="pointer-events-none size-5" />
      <span class="pointer-events-none text-[13px] font-semibold tracking-tight">mrhome</span>
    </div>

    <nav class="flex items-center gap-1 pr-3">
      <button
        v-for="tab in tabs"
        :key="tab.value"
        class="inline-flex h-7 shrink-0 items-center gap-2 rounded-md px-2.5 text-[13px] transition-colors [&_svg]:size-3.5"
        :class="
          ui.tab === tab.value
            ? 'bg-accent/15 font-medium text-accent-text'
            : 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'
        "
        @click="ui.tab = tab.value"
      >
        <component :is="tab.icon" />{{ tab.label }}
      </button>
    </nav>

    <template v-if="ui.tab === 'heating' && state.phase === 'ready'">
      <div class="my-auto inline-flex rounded-lg border border-border bg-background p-0.5" role="radiogroup" aria-label="Anwesenheit">
        <Tip
          v-for="option in presenceOptions"
          :key="option.value"
          :text="option.value === 'AUTO' ? 'Automatisch per Standort (Geofencing)' : `Fest auf „${option.label}“ stellen`"
          side="bottom"
        >
          <button
            role="radio"
            :aria-checked="presence === option.value"
            class="inline-flex h-7 items-center gap-1.5 rounded-md px-2.5 text-[13px] transition-colors [&_svg]:size-3.5"
            :class="presence === option.value ? 'bg-accent text-accent-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
            @click="setPresence(option.value === 'AUTO' ? null : option.value)"
          >
            <component :is="option.icon" />{{ option.label }}
          </button>
        </Tip>
      </div>
    </template>

    <div class="min-w-6 flex-1" data-tauri-drag-region />

    <div v-if="ui.tab === 'lights'" class="flex items-center gap-1 pr-2">
      <Tip text="Aktualisieren (F5) – liest Hue neu und sucht Govee-Geräte" side="bottom">
        <button class="icon-btn" aria-label="Aktualisieren" :disabled="lights.govee.scanning" @click="refreshLights">
          <LoaderCircle v-if="lights.govee.scanning" class="animate-spin" />
          <RefreshCw v-else />
        </button>
      </Tip>
      <DropdownMenuRoot v-if="lights.hue.bridge">
        <DropdownMenuTrigger as-child>
          <button class="icon-btn" aria-label="Einstellungen"><Settings /></button>
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent align="end" :side-offset="6" class="menu anim-fade w-72">
            <DropdownMenuLabel class="px-2 pt-1.5 pb-1 text-xs font-medium text-muted-foreground">
              Hue Bridge {{ lights.hue.bridge.ip }}
            </DropdownMenuLabel>
            <DropdownMenuItem :class="menuItem" @select="unpairHue"><Unlink />Hue Bridge trennen</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
    </div>

    <div v-else-if="state.phase === 'ready'" class="flex items-center gap-1 pr-2">
      <Tip text="Boost: 30 Minuten volle Leistung in allen Räumen" side="bottom">
        <button class="icon-btn" aria-label="Boost überall" @click="quickAction('boost')"><Flame /></button>
      </Tip>
      <Tip text="Heizung überall aus (Frostschutz bleibt)" side="bottom">
        <button class="icon-btn" aria-label="Heizung überall aus" @click="quickAction('allOff')"><PowerOff /></button>
      </Tip>
      <Tip text="Überall zurück zum Zeitplan" side="bottom">
        <button class="icon-btn" aria-label="Überall zurück zum Zeitplan" @click="quickAction('resumeSchedule')"><CalendarClock /></button>
      </Tip>
      <div class="mx-1 h-4 w-px bg-border" />
      <Tip text="Jetzt aktualisieren (F5) – kostet einen Zugriff" side="bottom">
        <button class="icon-btn" aria-label="Jetzt aktualisieren" :disabled="state.loading" @click="refresh">
          <LoaderCircle v-if="state.loading" class="animate-spin" />
          <RefreshCw v-else />
        </button>
      </Tip>

      <DropdownMenuRoot>
        <DropdownMenuTrigger as-child>
          <button class="icon-btn" aria-label="Einstellungen"><Settings /></button>
        </DropdownMenuTrigger>
        <DropdownMenuPortal>
          <DropdownMenuContent align="end" :side-offset="6" class="menu anim-fade w-80">
            <DropdownMenuLabel class="px-2 pt-1.5 pb-1 text-xs font-medium text-muted-foreground">Änderungen gelten</DropdownMenuLabel>
            <DropdownMenuItem
              v-for="option in TERMINATIONS"
              :key="option.label"
              :class="menuItem"
              @select.prevent="settings.termination = option.value"
            >
              <Check :class="!sameTermination(settings.termination, option.value) && 'invisible'" />{{ option.label }}
            </DropdownMenuItem>
            <DropdownMenuSeparator class="menu-separator" />
            <DropdownMenuLabel class="px-2 pt-1.5 pb-1 text-xs font-medium text-muted-foreground">
              Automatisch aktualisieren, solange das Fenster sichtbar ist
            </DropdownMenuLabel>
            <DropdownMenuItem v-for="minutes in intervals" :key="minutes" :class="menuItem" @select.prevent="settings.autoRefresh = minutes">
              <Check :class="settings.autoRefresh !== minutes && 'invisible'" />
              <span class="flex-1">{{ minutes ? (minutes < 60 ? `alle ${minutes} min` : `alle ${minutes / 60} h`) : "nie" }}</span>
              <span v-if="minutes" class="text-xs text-muted-foreground">bis {{ callsPerDay(minutes) }}/Tag</span>
            </DropdownMenuItem>
            <template v-if="state.homes.length > 1">
              <DropdownMenuSeparator class="menu-separator" />
              <DropdownMenuLabel class="px-2 pt-1.5 pb-1 text-xs font-medium text-muted-foreground">Zuhause</DropdownMenuLabel>
              <DropdownMenuItem v-for="home in state.homes" :key="home.id" :class="menuItem" @select="chooseHome(home)">
                <Check :class="settings.home?.id !== home.id && 'invisible'" />{{ home.name }}
              </DropdownMenuItem>
            </template>
            <DropdownMenuSeparator class="menu-separator" />
            <DropdownMenuItem :class="menuItem" @select="logout"><LogOut />Von tado abmelden</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenuPortal>
      </DropdownMenuRoot>
    </div>

    <WindowControls />
  </header>
</template>
