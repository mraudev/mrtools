<script setup lang="ts">
import { Moon, Sun } from "@lucide/vue";
import Tip from "./Tip.vue";
import { theme, toggleTheme } from "../lib/theme";
</script>

<template>
  <Tip :text="theme === 'dark' ? 'Helles Design' : 'Dunkles Design'">
    <button class="icon-btn size-6 [&_svg]:size-3.5" aria-label="Design umschalten" @click="toggleTheme">
      <Sun v-if="theme === 'dark'" />
      <Moon v-else />
    </button>
  </Tip>
</template>
