<script setup lang="ts">
import { ref } from "vue";
import { Eye, EyeOff } from "@lucide/vue";

const model = defineModel<string>({ required: true });
defineProps<{ id?: string; placeholder?: string }>();
const visible = ref(false);
</script>

<template>
  <div class="relative">
    <input
      :id="id"
      v-model="model"
      :type="visible ? 'text' : 'password'"
      class="input pr-9 font-mono text-[13px]"
      :placeholder="placeholder"
      autocomplete="off"
      spellcheck="false"
    />
    <button
      type="button"
      class="icon-btn absolute top-0.5 right-0.5"
      :aria-label="visible ? 'Verbergen' : 'Anzeigen'"
      @click="visible = !visible"
    >
      <EyeOff v-if="visible" />
      <Eye v-else />
    </button>
  </div>
</template>
