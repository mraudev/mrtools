<script setup lang="ts">
import { computed } from "vue";
import { Info, LoaderCircle } from "@lucide/vue";
import GroupCard from "./GroupCard.vue";
import HuePairCard from "./HuePairCard.vue";
import LightRow from "./LightRow.vue";
import { goveeColor, goveeName, lights, lightsWithoutRoom, renameGovee, setGovee } from "@/lib/lights";

const rooms = computed(() => lights.hue.state.groups.filter((g) => g.kind === "room"));
const zones = computed(() => lights.hue.state.groups.filter((g) => g.kind === "zone"));
const members = (ids: string[]) => ids.map((id) => lights.hue.state.lights.find((l) => l.id === id)).filter((l) => !!l);
</script>

<template>
  <div class="mx-auto flex max-w-4xl flex-col gap-3 p-4">
    <HuePairCard v-if="!lights.hue.bridge" />
    <template v-else>
      <div v-if="!lights.hue.loaded" class="grid h-24 place-items-center text-muted-foreground">
        <LoaderCircle class="size-5 animate-spin" />
      </div>
      <GroupCard v-for="room in rooms" :key="room.id" :group="room" :title="room.name" :members="members(room.lights)" />
      <GroupCard v-if="lightsWithoutRoom().length" title="Ohne Raum" :members="lightsWithoutRoom()" />
      <template v-if="zones.length">
        <h3 class="mt-2 px-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Zonen</h3>
        <GroupCard v-for="zone in zones" :key="zone.id" :group="zone" :title="zone.name" :members="members(zone.lights)" />
      </template>
    </template>

    <section class="rounded-xl border border-border bg-card">
      <header class="flex items-center gap-2 border-b border-border px-4 py-2.5">
        <h2 class="flex-1 font-semibold">Govee</h2>
        <LoaderCircle v-if="lights.govee.scanning" class="size-4 animate-spin text-muted-foreground" />
      </header>
      <div v-if="lights.govee.devices.length" class="p-1.5">
        <LightRow
          v-for="device in lights.govee.devices"
          :key="device.id"
          :name="goveeName(device)"
          :on="device.on ?? false"
          :brightness="device.brightness"
          :color="goveeColor(device)"
          :reachable="true"
          :can-color="true"
          :kelvin-range="[2000, 9000]"
          :kelvin="device.kelvin"
          renamable
          @on="setGovee(device, { on: $event })"
          @brightness="setGovee(device, { on: true, brightness: $event })"
          @rgb="setGovee(device, { on: true, rgb: $event })"
          @kelvin="setGovee(device, { on: true, kelvin: $event })"
          @rename="renameGovee(device, $event)"
        />
      </div>
      <div v-else-if="lights.govee.scanned" class="flex gap-3 p-4 text-sm text-muted-foreground">
        <Info class="mt-0.5 size-4 shrink-0 text-accent-text" />
        <div>
          <p class="font-medium text-foreground">Keine Govee-Geräte gefunden</p>
          <p v-if="lights.govee.error" class="mt-1 text-red-500">{{ lights.govee.error }}</p>
          <ul class="mt-1 list-disc pl-4">
            <li>In der Govee-Home-App beim Gerät unter Einstellungen „LAN-Steuerung“ einschalten (gibt es nur bei neueren Modellen).</li>
            <li>Hat Windows beim Start gefragt, ob mrhome im Netzwerk kommunizieren darf? Für private Netzwerke erlauben.</li>
            <li>PC und Lampen müssen im selben Netz (WLAN/LAN) sein. Danach oben auf „Aktualisieren“.</li>
          </ul>
        </div>
      </div>
    </section>
  </div>
</template>
