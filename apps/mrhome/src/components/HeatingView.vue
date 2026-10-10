<script setup lang="ts">
import { LoaderCircle } from "@lucide/vue";
import LoginView from "./LoginView.vue";
import RoomCard from "./RoomCard.vue";
import { state } from "@/lib/store";
</script>

<template>
  <div v-if="state.phase === 'checking'" class="grid h-full place-items-center text-muted-foreground">
    <LoaderCircle class="size-5 animate-spin" />
  </div>
  <LoginView v-else-if="state.phase === 'login'" />
  <div v-else class="mx-auto grid max-w-6xl grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3 p-4">
    <RoomCard v-for="room in state.rooms" :key="room.id" :room="room" />
    <p v-if="!state.rooms.length && !state.loading" class="col-span-full py-12 text-center text-muted-foreground">
      Keine Räume gefunden.
    </p>
  </div>
</template>
