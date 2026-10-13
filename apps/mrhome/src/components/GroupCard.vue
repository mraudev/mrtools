<script setup lang="ts">
import { computed } from "vue";
import { Sparkles } from "@lucide/vue";
import LightRow from "./LightRow.vue";
import Toggle from "./Toggle.vue";
import { kelvinToMirek, mirekToKelvin, rgbToXy } from "@/lib/color";
import { hueColor, lights, recallScene, setHueGroup, setHueLight } from "@/lib/lights";
import type { HueGroup, HueLight } from "@/lib/types";

/** A Hue room or zone (or the lights without a room, then without `group`). */
const props = defineProps<{ group?: HueGroup; title: string; members: HueLight[] }>();

const anyOn = computed(() => props.members.some((l) => l.on && l.reachable));
const dimmable = computed(() => props.members.filter((l) => l.on && l.brightness !== null));
const brightness = computed(() =>
  dimmable.value.length ? dimmable.value.reduce((sum, l) => sum + (l.brightness ?? 0), 0) / dimmable.value.length : 50,
);
const scenes = computed(() => (props.group ? lights.hue.state.scenes.filter((s) => s.group === props.group!.id) : []));

const kelvinRange = (l: HueLight): [number, number] | undefined =>
  l.mirekMin && l.mirekMax ? [mirekToKelvin(l.mirekMax), mirekToKelvin(l.mirekMin)] : undefined;
</script>

<template>
  <section class="rounded-xl border border-border bg-card">
    <header class="flex items-center gap-3 border-b border-border px-4 py-2.5">
      <h2 class="min-w-0 flex-1 truncate font-semibold">
        {{ title }}
        <span class="ml-1 text-xs font-normal text-muted-foreground">
          {{ members.filter((l) => l.on).length }} von {{ members.length }} an
        </span>
      </h2>
      <template v-if="group?.groupedLight">
        <input
          type="range"
          min="1"
          max="100"
          :value="brightness"
          :disabled="!anyOn"
          class="w-28 accent-[var(--accent)]"
          :aria-label="`Helligkeit ${title}`"
          @input="setHueGroup(group, { brightness: Number(($event.target as HTMLInputElement).value) })"
        />
        <Toggle :model-value="anyOn" :label="`${title} schalten`" @update:model-value="setHueGroup(group, { on: $event })" />
      </template>
    </header>

    <div v-if="scenes.length" class="flex flex-wrap gap-1.5 border-b border-border px-4 py-2">
      <button
        v-for="scene in scenes"
        :key="scene.id"
        class="inline-flex h-6 items-center gap-1 rounded-full border border-border px-2.5 text-xs transition-colors hover:border-accent/60 hover:bg-accent/10"
        @click="recallScene(scene)"
      >
        <Sparkles class="size-3 text-accent-text" />{{ scene.name }}
      </button>
    </div>

    <div class="p-1.5">
      <LightRow
        v-for="light in members"
        :key="light.id"
        :name="light.name"
        :on="light.on"
        :brightness="light.brightness"
        :color="hueColor(light)"
        :reachable="light.reachable"
        :can-color="light.xy !== null"
        :kelvin-range="kelvinRange(light)"
        :kelvin="light.mirek ? mirekToKelvin(light.mirek) : null"
        @on="setHueLight(light, { on: $event })"
        @brightness="setHueLight(light, { on: true, brightness: $event })"
        @rgb="setHueLight(light, { on: true, xy: rgbToXy($event) })"
        @kelvin="setHueLight(light, { on: true, mirek: kelvinToMirek($event) })"
      />
    </div>
  </section>
</template>
