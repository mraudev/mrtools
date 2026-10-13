import type { Quota, Room, Termination } from "./types";

/** tado X accepts 5–25 °C. */
export const MIN_TEMP = 5;
export const MAX_TEMP = 25;
export const STEP = 0.5;

/** Calls kept back for changes by hand: automatic refreshing stops below this. */
export const RESERVE = 15;

export function clampTarget(value: number): number {
  return Math.min(MAX_TEMP, Math.max(MIN_TEMP, Math.round(value / STEP) * STEP));
}

/** Automatic refreshing is allowed while the budget is unknown or above the reserve. */
export function budgetAllowsAuto(quota: Quota | null): boolean {
  return quota?.remaining == null || quota.remaining > RESERVE;
}

export function isDue(lastUpdate: number, now: number, minutes: number): boolean {
  return minutes > 0 && now - lastUpdate >= minutes * 60_000;
}

/** Calls a day if the window stays open the whole time (one call per refresh). */
export function callsPerDay(minutes: number): number {
  return minutes > 0 ? Math.ceil((24 * 60) / minutes) : 0;
}

export function formatTemp(value: number | null | undefined, digits = 1): string {
  if (value == null) return "–";
  return `${value.toLocaleString("de-DE", { minimumFractionDigits: digits, maximumFractionDigits: digits })}°`;
}

export function formatTime(iso: string | number): string {
  return new Date(iso).toLocaleTimeString("de-DE", { hour: "2-digit", minute: "2-digit" });
}

function sameDay(a: Date, b: Date) {
  return a.toDateString() === b.toDateString();
}

/** "21:00", or "morgen 06:00" / "Sa. 06:00" for later days. */
export function formatWhen(iso: string, now: Date): string {
  const d = new Date(iso);
  if (sameDay(d, now)) return formatTime(iso);
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  if (sameDay(d, tomorrow)) return `morgen ${formatTime(iso)}`;
  return `${d.toLocaleDateString("de-DE", { weekday: "short" })} ${formatTime(iso)}`;
}

/** What the room is doing, e.g. "Manuell bis 21:00" or "Zeitplan". */
export function modeLabel(room: Room, now: Date): string {
  const until = room.until ? ` bis ${formatWhen(room.until, now)}` : "";
  switch (room.mode) {
    case "boost":
      return `Boost${until}`;
    case "timer":
    case "nextBlock":
      return `Manuell${until || " bis zur nächsten Planänderung"}`;
    case "manual":
      return "Manuell, dauerhaft";
    default:
      return "Zeitplan";
  }
}

/** The room as it looks right after a change by hand – before tado reports back. */
export function applyManual(room: Room, temperature: number | null, termination: Termination, now: number): Room {
  const until =
    termination.kind === "timer"
      ? new Date(now + termination.seconds * 1000).toISOString()
      : termination.kind === "nextBlock"
        ? (room.nextChange?.start ?? null)
        : null;
  return {
    ...room,
    power: temperature !== null,
    target: temperature,
    mode: termination.kind,
    until,
  };
}

export const TERMINATIONS: { label: string; value: Termination }[] = [
  { label: "Bis zur nächsten Planänderung", value: { kind: "nextBlock" } },
  { label: "Für 1 Stunde", value: { kind: "timer", seconds: 3600 } },
  { label: "Für 2 Stunden", value: { kind: "timer", seconds: 7200 } },
  { label: "Für 3 Stunden", value: { kind: "timer", seconds: 10800 } },
  { label: "Dauerhaft", value: { kind: "manual" } },
];

export const sameTermination = (a: Termination, b: Termination) =>
  a.kind === b.kind && (a.kind !== "timer" || (b.kind === "timer" && a.seconds === b.seconds));
