import { reactive, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { kelvinToRgb, mirekToKelvin, xyToRgb, type Rgb } from "./color";
import type {
  FoundBridge,
  GoveeChange,
  GoveeDevice,
  HueChange,
  HueGroup,
  HueLight,
  HueScene,
  HueState,
  HueUpdate,
} from "./types";

export const lights = reactive({
  hue: {
    bridge: null as FoundBridge | null,
    found: [] as FoundBridge[],
    searching: false,
    /** IP of the bridge being paired (waiting for the button). */
    pairing: "",
    /** The live event stream is up. */
    connected: true,
    loaded: false,
    state: { lights: [], groups: [], scenes: [] } as HueState,
  },
  govee: {
    devices: [] as GoveeDevice[],
    scanning: false,
    scanned: false,
    error: null as string | null,
  },
});

// ---------------------------------------------------------------------------
// Govee names – the LAN API knows only model and id.

function loadNames(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem("goveeNames") ?? "{}");
  } catch {
    return {};
  }
}

export const goveeNames = reactive<Record<string, string>>(loadNames());

watch(goveeNames, () => {
  try {
    localStorage.setItem("goveeNames", JSON.stringify(goveeNames));
  } catch {
    // Not persisted – still applies for this session.
  }
});

export const goveeName = (d: GoveeDevice) => goveeNames[d.id] || `Govee ${d.sku}`;

export function renameGovee(d: GoveeDevice, name: string) {
  if (name.trim()) goveeNames[d.id] = name.trim();
  else delete goveeNames[d.id];
}

// ---------------------------------------------------------------------------
// Sending: per light one request at a time, the newest value wins (smooth sliders).

const queues = new Map<string, { busy: boolean; next: object | null }>();

async function push<T extends object>(key: string, change: T, send: (change: T) => Promise<void>, title: string) {
  const queue = queues.get(key) ?? { busy: false, next: null };
  queues.set(key, queue);
  if (queue.busy) {
    queue.next = { ...queue.next, ...change };
    return;
  }
  queue.busy = true;
  try {
    await send(change);
  } catch (e) {
    toastError(title, e);
  } finally {
    queue.busy = false;
    const next = queue.next as T | null;
    queue.next = null;
    if (next) push(key, next, send, title);
  }
}

// ---------------------------------------------------------------------------
// Hue

export async function loadHue() {
  try {
    lights.hue.state = await api.hueState();
    lights.hue.loaded = true;
  } catch (e) {
    toastError("Hue Bridge konnte nicht gelesen werden", e);
  }
}

export async function discoverHue() {
  lights.hue.searching = true;
  try {
    lights.hue.found = await api.hueDiscover();
  } catch (e) {
    toastError("Suche nach der Hue Bridge fehlgeschlagen", e);
  } finally {
    lights.hue.searching = false;
  }
}

export async function pairHue(ip: string) {
  lights.hue.pairing = ip;
  try {
    await api.huePair(ip);
    lights.hue.bridge = await api.hueStatus();
    toast("success", "Hue Bridge verbunden");
    await loadHue();
  } catch (e) {
    toastError("Verbinden mit der Hue Bridge fehlgeschlagen", e);
  } finally {
    lights.hue.pairing = "";
  }
}

export async function unpairHue() {
  try {
    await api.hueUnpair();
    lights.hue.bridge = null;
    lights.hue.loaded = false;
    lights.hue.state = { lights: [], groups: [], scenes: [] };
    discoverHue();
  } catch (e) {
    toastError("Trennen fehlgeschlagen", e);
  }
}

function patchLight(id: string, change: HueChange | HueUpdate) {
  const light = lights.hue.state.lights.find((l) => l.id === id);
  if (!light) return;
  if (change.on != null) light.on = change.on;
  if (change.brightness != null) light.brightness = change.brightness;
  // Colour and white exclude each other.
  if (change.xy != null) {
    light.xy = change.xy;
    light.mirek = null;
  }
  if (change.mirek != null) light.mirek = change.mirek;
}

export function setHueLight(light: HueLight, change: HueChange) {
  patchLight(light.id, change);
  push(`hue:${light.id}`, change, (c) => api.hueSetLight(light.id, c), `${light.name} konnte nicht geschaltet werden`);
}

export function setHueGroup(group: HueGroup, change: HueChange) {
  if (!group.groupedLight) return;
  for (const id of group.lights) {
    const light = lights.hue.state.lights.find((l) => l.id === id);
    if (light && (change.on !== undefined || light.on)) patchLight(id, change);
  }
  const target = group.groupedLight;
  push(`hue-group:${target}`, change, (c) => api.hueSetGroup(target, c), `${group.name} konnte nicht geschaltet werden`);
}

export function recallScene(scene: HueScene) {
  api.hueScene(scene.id).catch((e) => toastError(`Szene „${scene.name}“ fehlgeschlagen`, e));
}

export const groupLights = (group: HueGroup) =>
  group.lights.map((id) => lights.hue.state.lights.find((l) => l.id === id)).filter((l): l is HueLight => !!l);

/** Lights that are in no room – shown in their own group. */
export function lightsWithoutRoom(): HueLight[] {
  const inRoom = new Set(lights.hue.state.groups.filter((g) => g.kind === "room").flatMap((g) => g.lights));
  return lights.hue.state.lights.filter((l) => !inRoom.has(l.id));
}

/** Colour a Hue light shines in, `null` when off. */
export function hueColor(light: HueLight): Rgb | null {
  if (!light.on) return null;
  if (light.mirek) return kelvinToRgb(mirekToKelvin(light.mirek));
  if (light.xy) return xyToRgb(light.xy);
  return kelvinToRgb(2700);
}

// ---------------------------------------------------------------------------
// Govee

export async function scanGovee() {
  lights.govee.scanning = true;
  try {
    const scan = await api.goveeScan();
    lights.govee.devices = scan.devices;
    lights.govee.error = scan.error;
    lights.govee.scanned = true;
  } catch (e) {
    toastError("Suche nach Govee-Geräten fehlgeschlagen", e);
  } finally {
    lights.govee.scanning = false;
  }
}

export function setGovee(device: GoveeDevice, change: GoveeChange) {
  if (change.on !== undefined) device.on = change.on;
  if (change.brightness !== undefined) device.brightness = change.brightness;
  if (change.rgb) {
    device.rgb = change.rgb;
    device.kelvin = null;
  }
  if (change.kelvin) device.kelvin = change.kelvin;
  push(`govee:${device.id}`, change, (c) => api.goveeSet(device.ip, c), `${goveeName(device)} konnte nicht geschaltet werden`);
}

export function goveeColor(device: GoveeDevice): Rgb | null {
  if (!device.on) return null;
  if (device.kelvin) return kelvinToRgb(device.kelvin);
  return device.rgb ?? [255, 255, 255];
}

// ---------------------------------------------------------------------------
// Start

export async function initLights() {
  await listen<HueUpdate[]>("hue-update", ({ payload }) => payload.forEach((u) => patchLight(u.id, u)));
  await listen<boolean>("hue-connected", ({ payload }) => {
    const reconnected = payload && !lights.hue.connected;
    lights.hue.connected = payload;
    // Changes made while the stream was down are picked up again.
    if (reconnected) loadHue();
  });
  await listen<GoveeDevice>("govee-update", ({ payload }) => {
    const index = lights.govee.devices.findIndex((d) => d.id === payload.id);
    if (index >= 0) lights.govee.devices[index] = payload;
    else lights.govee.devices.push(payload);
  });

  lights.hue.bridge = await api.hueStatus().catch(() => null);
  if (lights.hue.bridge) loadHue();
  else discoverHue();
  scanGovee();
}
