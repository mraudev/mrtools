<script setup lang="ts">
import { computed, ref } from "vue";
import { HISTORY } from "@/lib/store";

/** Single-series history chart with a hover crosshair; newest sample on the right. */
const props = defineProps<{
  label: string;
  values: number[];
  /** Top of the y-axis. */
  max: number;
  format: (value: number) => string;
  /** CSS color of the line. */
  color: string;
  /** Seconds between samples, for the hover text. */
  step: number;
}>();

const W = 300;
const H = 72;

const x = (i: number) => ((HISTORY - props.values.length + i) / (HISTORY - 1)) * W;
const y = (v: number) => H - (Math.min(v, props.max) / (props.max || 1)) * (H - 2) - 1;

const line = computed(() => props.values.map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" "));
const area = computed(() =>
  props.values.length ? `${x(0)},${H} ${line.value} ${x(props.values.length - 1)},${H}` : "",
);

const hover = ref<number | null>(null);

function onMove(event: MouseEvent) {
  const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const slot = Math.round(((event.clientX - box.left) / box.width) * (HISTORY - 1));
  const i = slot - (HISTORY - props.values.length);
  hover.value = i >= 0 && i < props.values.length ? i : null;
}

const hoverText = computed(() => {
  if (hover.value === null) return "";
  const ago = Math.round((props.values.length - 1 - hover.value) * props.step);
  return `${props.format(props.values[hover.value])} · ${ago ? `vor ${ago} s` : "jetzt"}`;
});
</script>

<template>
  <div class="rounded-lg border border-border bg-card p-2.5">
    <div class="mb-1.5 flex items-baseline gap-2 text-xs">
      <span class="font-medium">{{ label }}</span>
      <span class="ml-auto font-semibold text-foreground tabular-nums">
        {{ values.length ? format(values[values.length - 1]) : "–" }}
      </span>
    </div>
    <div class="relative h-[72px]" @mousemove="onMove" @mouseleave="hover = null">
      <svg :viewBox="`0 0 ${W} ${H}`" preserveAspectRatio="none" class="absolute inset-0 size-full overflow-visible">
        <line x1="0" :y1="H / 2" :x2="W" :y2="H / 2" stroke="var(--chart-grid)" stroke-width="1" vector-effect="non-scaling-stroke" />
        <line x1="0" :y1="H" :x2="W" :y2="H" stroke="var(--chart-baseline)" stroke-width="1" vector-effect="non-scaling-stroke" />
        <polygon v-if="area" :points="area" :fill="color" fill-opacity="0.14" />
        <polyline
          :points="line"
          fill="none"
          :stroke="color"
          stroke-width="2"
          stroke-linejoin="round"
          stroke-linecap="round"
          vector-effect="non-scaling-stroke"
        />
        <line
          v-if="hover !== null"
          :x1="x(hover)"
          y1="0"
          :x2="x(hover)"
          :y2="H"
          stroke="var(--muted-foreground)"
          stroke-width="1"
          stroke-dasharray="3 3"
          vector-effect="non-scaling-stroke"
        />
      </svg>
      <span
        v-if="hover !== null"
        class="pointer-events-none absolute size-2.5 -translate-1/2 rounded-full border-2 border-card"
        :style="{ left: `${(x(hover) / W) * 100}%`, top: `${(y(values[hover]) / H) * 100}%`, background: color }"
      />
      <span class="pointer-events-none absolute top-0 left-0 text-[10px] text-muted-foreground tabular-nums">
        {{ format(max) }}
      </span>
      <div
        v-if="hover !== null"
        class="pointer-events-none absolute -top-7 rounded-md border border-border bg-popover px-2 py-0.5 text-xs whitespace-nowrap shadow-lg tabular-nums"
        :class="x(hover) > W / 2 ? '-translate-x-full' : ''"
        :style="{ left: `${(x(hover) / W) * 100}%` }"
      >
        {{ hoverText }}
      </div>
    </div>
  </div>
</template>
