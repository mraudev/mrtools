<script setup lang="ts">
import { onMounted, ref } from "vue";
import { commitRename } from "@/lib/actions";
import { state } from "@/lib/store";

const props = defineProps<{ path: string; name: string }>();

const input = ref<HTMLInputElement>();
const value = ref(props.name);
let done = false;

onMounted(() => {
  const el = input.value!;
  el.focus();
  // Like Explorer: the name without extension is selected.
  const dot = props.name.lastIndexOf(".");
  el.setSelectionRange(0, dot > 0 ? dot : props.name.length);
});

function finish(save: boolean) {
  if (done) return;
  done = true;
  if (save) commitRename(props.path, value.value);
  else state.renaming = "";
  document.getElementById("file-list")?.focus();
}
</script>

<template>
  <input
    ref="input"
    v-model="value"
    class="h-6 min-w-0 rounded border border-accent bg-background px-1 text-[13px] outline-none ring-2 ring-accent/25"
    spellcheck="false"
    aria-label="Neuer Name"
    @keydown.stop.enter="finish(true)"
    @keydown.stop.esc="finish(false)"
    @keydown.stop
    @blur="finish(true)"
    @click.stop
    @dblclick.stop
    @mousedown.stop
  />
</template>
