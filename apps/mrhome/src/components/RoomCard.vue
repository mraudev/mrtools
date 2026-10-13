<script setup lang="ts">
import { computed } from "vue";
import { CalendarClock, Droplets, Flame, LoaderCircle, Minus, Plus, Power, WifiOff, Wind, X } from "@lucide/vue";
import Tip from "@mrtools/ui/components/Tip";
import { formatTemp, formatWhen, MAX_TEMP, MIN_TEMP, modeLabel, STEP } from "@/lib/logic";
import { endOpenWindow, resumeRoom, setTarget, state } from "@/lib/store";
import type { Room } from "@/lib/types";

const props = defineProps<{ room: Room }>();

const now = computed(() => new Date(state.lastUpdate || Date.now()));
const pending = computed(() => state.pending.has(props.room.id));
const target = computed(() => props.room.target ?? props.room.temperature ?? 20);
const heating = computed(() => (props.room.heatingPower ?? 0) > 0);

function step(delta: number) {
  setTarget(props.room, (props.room.power ? target.value : (props.room.temperature ?? 20)) + delta);
}

function togglePower() {
  setTarget(props.room, props.room.power ? null : Math.round(props.room.temperature ?? 20));
}
</script>

<template>
  <article
    class="flex flex-col gap-3 rounded-xl border bg-card p-4 transition-colors"
    :class="heating && room.power ? 'border-accent/40' : 'border-border'"
  >
    <header class="flex items-start gap-2">
      <h2 class="min-w-0 flex-1 truncate font-semibold">{{ room.name }}</h2>
      <Tip v-if="!room.connected" text="Thermostat nicht erreichbar">
        <WifiOff class="size-4 text-red-500" />
      </Tip>
      <Tip v-if="room.openWindow !== null" text="Fenster offen erkannt – Heizung pausiert">
        <button
          class="inline-flex h-6 items-center gap-1 rounded-md bg-sky-500/15 px-1.5 text-xs text-sky-600 dark:text-sky-400 hover:bg-sky-500/25"
          @click="endOpenWindow(room)"
        >
          <Wind class="size-3.5" />Fenster offen<X class="size-3" />
        </button>
      </Tip>
    </header>

    <div class="flex items-end justify-between gap-2">
      <div>
        <p class="text-4xl font-light tracking-tight tabular-nums">{{ formatTemp(room.temperature) }}</p>
        <p v-if="room.humidity !== null" class="mt-1 inline-flex items-center gap-1 text-xs text-muted-foreground tabular-nums">
          <Droplets class="size-3.5" />{{ Math.round(room.humidity) }} % Luftfeuchte
        </p>
      </div>

      <div class="flex items-center gap-1">
        <button class="icon-btn size-8 rounded-full border border-border" aria-label="Kälter" :disabled="room.power && target <= MIN_TEMP" @click="step(-STEP)">
          <Minus />
        </button>
        <div class="w-20 text-center">
          <p class="text-2xl font-semibold tabular-nums" :class="room.power ? 'text-accent-text' : 'text-muted-foreground'">
            {{ room.power ? formatTemp(target) : "Aus" }}
          </p>
          <p class="text-[11px] text-muted-foreground">Soll</p>
        </div>
        <button class="icon-btn size-8 rounded-full border border-border" aria-label="Wärmer" :disabled="room.power && target >= MAX_TEMP" @click="step(STEP)">
          <Plus />
        </button>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <Flame class="size-4 shrink-0" :class="heating ? 'text-accent-text' : 'text-muted-foreground/50'" />
      <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-foreground/8">
        <div class="h-full rounded-full bg-accent transition-[width]" :style="{ width: `${room.heatingPower ?? 0}%` }" />
      </div>
      <span class="w-9 text-right text-xs text-muted-foreground tabular-nums">{{ Math.round(room.heatingPower ?? 0) }} %</span>
    </div>

    <footer class="flex items-center gap-2 border-t border-border pt-3 text-xs">
      <span class="min-w-0 flex-1 truncate text-muted-foreground">
        <template v-if="pending"><LoaderCircle class="mr-1 inline size-3 animate-spin" />wird gesendet …</template>
        <template v-else>
          {{ modeLabel(room, now) }}
          <template v-if="room.mode === 'schedule' && room.nextChange">
            · ab {{ formatWhen(room.nextChange.start, now) }}
            {{ room.nextChange.power ? formatTemp(room.nextChange.temperature) : "aus" }}
          </template>
        </template>
      </span>
      <Tip v-if="room.mode !== 'schedule'" text="Zurück zum Zeitplan">
        <button class="icon-btn size-7" aria-label="Zurück zum Zeitplan" @click="resumeRoom(room)"><CalendarClock /></button>
      </Tip>
      <Tip :text="room.power ? 'Heizung aus' : 'Heizung an'">
        <button
          class="icon-btn size-7"
          :class="!room.power && 'bg-foreground/8 text-foreground'"
          :aria-label="room.power ? 'Heizung aus' : 'Heizung an'"
          @click="togglePower"
        >
          <Power />
        </button>
      </Tip>
    </footer>
  </article>
</template>
