export interface Quota {
  remaining: number | null;
  limit: number | null;
  /** ms since 1970 when tado renews the budget. */
  resetAt: number | null;
}

export interface DeviceLogin {
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  expiresIn: number;
  interval: number;
}

export interface Home {
  id: number;
  name: string;
}

export type Mode = "schedule" | "boost" | "nextBlock" | "timer" | "manual";

export interface Room {
  id: number;
  name: string;
  temperature: number | null;
  humidity: number | null;
  power: boolean;
  target: number | null;
  heatingPower: number | null;
  connected: boolean;
  /** Seconds until open-window mode ends, if active. */
  openWindow: number | null;
  mode: Mode;
  /** ISO time when manual control or boost ends. */
  until: string | null;
  nextChange: { start: string; power: boolean; temperature: number | null } | null;
}

export interface HomeState {
  presence: "HOME" | "AWAY";
  /** Set by hand instead of by geofencing. */
  locked: boolean;
}

export type Termination = { kind: "nextBlock" } | { kind: "timer"; seconds: number } | { kind: "manual" };

// ---------------------------------------------------------------------------
// Lights

export interface FoundBridge {
  id: string;
  ip: string;
}

export interface HueLight {
  id: string;
  name: string;
  archetype: string;
  on: boolean;
  /** 0–100, null for on/off-only lights. */
  brightness: number | null;
  /** CIE xy, null for lights without colour. */
  xy: [number, number] | null;
  /** White temperature in mirek, null while in colour mode. */
  mirek: number | null;
  mirekMin: number | null;
  mirekMax: number | null;
  reachable: boolean;
}

export interface HueGroup {
  id: string;
  kind: "room" | "zone";
  name: string;
  archetype: string;
  lights: string[];
  groupedLight: string | null;
}

export interface HueScene {
  id: string;
  name: string;
  group: string;
}

export interface HueState {
  lights: HueLight[];
  groups: HueGroup[];
  scenes: HueScene[];
}

export interface HueChange {
  on?: boolean;
  brightness?: number;
  xy?: [number, number];
  mirek?: number;
}

export type HueUpdate = { id: string } & { [K in keyof HueChange]: HueChange[K] | null };

export interface GoveeDevice {
  id: string;
  sku: string;
  ip: string;
  on: boolean | null;
  brightness: number | null;
  rgb: [number, number, number] | null;
  kelvin: number | null;
}

export interface GoveeChange {
  on?: boolean;
  brightness?: number;
  rgb?: [number, number, number];
  kelvin?: number;
}
