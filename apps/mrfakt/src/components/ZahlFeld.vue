<script setup lang="ts">
import { ref, watch } from "vue";
import { parseZahl } from "@/lib/format";

/** Zahleneingabe mit deutschem Format; erlaubt einfache Rechnungen („12+3“). */
const model = defineModel<number>({ required: true });
const props = withDefaults(defineProps<{ stellen?: number; maxStellen?: number; disabled?: boolean; placeholder?: string }>(), {
  stellen: 2,
  maxStellen: 3,
});
const emit = defineEmits<{ enter: [] }>();

const text = ref("");
const fokus = ref(false);

function anzeigen(n: number) {
  return new Intl.NumberFormat("de-DE", { minimumFractionDigits: props.stellen, maximumFractionDigits: props.maxStellen }).format(n ?? 0);
}

watch(
  model,
  (n) => {
    if (!fokus.value) text.value = anzeigen(n);
  },
  { immediate: true },
);

function uebernehmen() {
  const n = parseZahl(text.value);
  if (n !== model.value) model.value = n;
  text.value = anzeigen(n);
}

function onFocus(e: FocusEvent) {
  fokus.value = true;
  (e.target as HTMLInputElement).select();
}

function onBlur() {
  fokus.value = false;
  uebernehmen();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    uebernehmen();
    emit("enter");
  }
}
</script>

<template>
  <input
    v-model="text"
    class="input text-right tabular-nums"
    inputmode="decimal"
    :disabled="disabled"
    :placeholder="placeholder"
    spellcheck="false"
    @focus="onFocus"
    @blur="onBlur"
    @keydown="onKeydown"
  />
</template>
