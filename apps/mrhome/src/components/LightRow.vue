<script setup lang="ts">
import { nextTick, ref } from "vue";
import { Unplug } from "@lucide/vue";
import ColorPopover from "./ColorPopover.vue";
import Toggle from "./Toggle.vue";
import Tip from "@mrtools/ui/components/Tip";
import type { Rgb } from "@/lib/color";

/** One lamp: colour, name, brightness and switch – the same for Hue and Govee. */
const props = defineProps<{
  name: string;
  on: boolean;
  brightness: number | null;
  color: Rgb | null;
  reachable: boolean;
  canColor: boolean;
  kelvinRange?: [number, number];
  kelvin?: number | null;
  /** Name can be changed (Govee – the LAN API has no names). */
  renamable?: boolean;
}>();
const emit = defineEmits<{ on: [boolean]; brightness: [number]; rgb: [Rgb]; kelvin: [number]; rename: [string] }>();

const editing = ref(false);
const input = ref<HTMLInputElement>();

async function startRename() {
  if (!props.renamable) return;
  editing.value = true;
  await nextTick();
  input.value?.select();
}

function finishRename(save: boolean) {
  if (save && input.value) emit("rename", input.value.value);
  editing.value = false;
}
</script>

<template>
  <div class="flex h-10 items-center gap-3 rounded-lg px-2 hover:bg-foreground/[0.03]" :class="!reachable && 'opacity-50'">
    <ColorPopover
      :color="color"
      :can-color="canColor"
      :kelvin-range="kelvinRange"
      :kelvin="kelvin"
      :disabled="!reachable"
      @rgb="emit('rgb', $event)"
      @kelvin="emit('kelvin', $event)"
    />
    <div class="flex min-w-0 flex-1 items-center gap-1.5">
      <input
        v-if="editing"
        ref="input"
        :value="name"
        class="input h-7"
        aria-label="Name"
        @keydown.enter="finishRename(true)"
        @keydown.esc="finishRename(false)"
        @blur="finishRename(true)"
      />
      <span
        v-else
        class="truncate text-[13px]"
        :class="renamable && 'cursor-text'"
        :title="renamable ? 'Doppelklick zum Umbenennen' : name"
        @dblclick="startRename"
      >
        {{ name }}
      </span>
      <Tip v-if="!reachable" text="Nicht erreichbar – Strom aus oder außer Funkreichweite">
        <Unplug class="size-3.5 shrink-0 text-muted-foreground" />
      </Tip>
    </div>
    <input
      v-if="brightness !== null"
      type="range"
      min="1"
      max="100"
      :value="brightness"
      :disabled="!reachable"
      class="w-28 accent-[var(--accent)]"
      :aria-label="`Helligkeit ${name}`"
      @input="emit('brightness', Number(($event.target as HTMLInputElement).value))"
    />
    <span v-if="brightness !== null" class="w-9 text-right text-xs text-muted-foreground tabular-nums">
      {{ on ? `${Math.round(brightness)} %` : "" }}
    </span>
    <Toggle :model-value="on" :label="`${name} schalten`" :disabled="!reachable" @update:model-value="emit('on', $event)" />
  </div>
</template>
