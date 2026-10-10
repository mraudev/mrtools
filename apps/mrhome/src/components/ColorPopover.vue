<script setup lang="ts">
import { ref } from "vue";
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from "reka-ui";
import { css, hsvToRgb, kelvinToRgb, PALETTE, type Rgb } from "@/lib/color";

/** Colour and white choice for one light. */
defineProps<{
  color: Rgb | null;
  canColor: boolean;
  /** Supported white range in Kelvin; omitted if the light has no white mode. */
  kelvinRange?: [number, number];
  kelvin?: number | null;
  disabled?: boolean;
}>();
const emit = defineEmits<{ rgb: [Rgb]; kelvin: [number] }>();

const hue = ref(30);

const gradient = `linear-gradient(to right, ${[0, 60, 120, 180, 240, 300, 360].map((h) => css(hsvToRgb(h, 1, 1))).join(", ")})`;
const whiteGradient = (range: [number, number]) =>
  `linear-gradient(to right, ${css(kelvinToRgb(range[0]))}, ${css(kelvinToRgb((range[0] + range[1]) / 2))}, ${css(kelvinToRgb(range[1]))})`;

const kelvinPresets = (range: [number, number]) =>
  [2200, 2700, 3500, 4500, 6500].filter((k) => k >= range[0] && k <= range[1]);

function onHue(event: Event) {
  hue.value = Number((event.target as HTMLInputElement).value);
  emit("rgb", hsvToRgb(hue.value, 1, 1));
}

function onKelvin(event: Event) {
  emit("kelvin", Number((event.target as HTMLInputElement).value));
}

</script>

<template>
  <PopoverRoot>
    <PopoverTrigger as-child>
      <button
        class="grid size-7 shrink-0 place-items-center rounded-full border border-border transition-transform hover:scale-110 disabled:pointer-events-none disabled:opacity-40"
        :style="{ background: color ? css(color) : 'transparent' }"
        :disabled="disabled || (!canColor && !kelvinRange)"
        aria-label="Farbe wählen"
      />
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent :side-offset="6" align="start" class="anim-pop z-50 w-64 rounded-lg border border-border bg-popover p-3 shadow-xl">
        <template v-if="canColor">
          <p class="mb-2 text-xs font-medium text-muted-foreground">Farbe</p>
          <div class="grid grid-cols-9 gap-1.5">
            <button
              v-for="c in PALETTE"
              :key="c.join()"
              class="aspect-square rounded-full border border-black/10 transition-transform hover:scale-110"
              :style="{ background: css(c) }"
              :aria-label="`Farbe ${c.join(', ')}`"
              @click="emit('rgb', c)"
            />
          </div>
          <input
            type="range"
            min="0"
            max="360"
            :value="hue"
            class="spectrum mt-3 w-full"
            :style="{ background: gradient }"
            aria-label="Farbton"
            @input="onHue"
          />
        </template>
        <template v-if="kelvinRange">
          <p class="mt-3 mb-2 text-xs font-medium text-muted-foreground first:mt-0">Weiß</p>
          <div class="flex gap-1.5">
            <button
              v-for="k in kelvinPresets(kelvinRange)"
              :key="k"
              class="h-7 flex-1 rounded-md border border-black/10 text-[10px] text-black/70 transition-transform hover:scale-105"
              :style="{ background: css(kelvinToRgb(k)) }"
              @click="emit('kelvin', k)"
            >
              {{ k }} K
            </button>
          </div>
          <input
            type="range"
            :min="kelvinRange[0]"
            :max="kelvinRange[1]"
            step="50"
            :value="kelvin ?? Math.round((kelvinRange[0] + kelvinRange[1]) / 2)"
            class="spectrum mt-3 w-full"
            :style="{ background: whiteGradient(kelvinRange) }"
            aria-label="Weißton"
            @input="onKelvin"
          />
        </template>
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>

<style scoped>
.spectrum {
  appearance: none;
  height: 0.75rem;
  border-radius: 999px;
  outline: none;
}
.spectrum::-webkit-slider-thumb {
  appearance: none;
  width: 1rem;
  height: 1rem;
  border-radius: 999px;
  background: white;
  border: 2px solid rgb(0 0 0 / 0.35);
  box-shadow: 0 1px 3px rgb(0 0 0 / 0.3);
}
</style>
