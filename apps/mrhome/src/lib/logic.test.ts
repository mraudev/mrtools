import { describe, expect, it } from "vitest";
import { applyManual, budgetAllowsAuto, callsPerDay, clampTarget, formatTemp, formatWhen, isDue, modeLabel } from "./logic";
import type { Room } from "./types";

const room: Room = {
  id: 1,
  name: "Wohnzimmer",
  temperature: 20.4,
  humidity: 48,
  power: true,
  target: 21,
  heatingPower: 30,
  connected: true,
  openWindow: null,
  mode: "schedule",
  until: null,
  nextChange: { start: "2026-10-10T19:00:00.000Z", power: true, temperature: 18 },
};

describe("budget", () => {
  it("stops automatic refreshing near the end of the budget", () => {
    expect(budgetAllowsAuto(null)).toBe(true);
    expect(budgetAllowsAuto({ remaining: null, limit: 100, resetAt: null })).toBe(true);
    expect(budgetAllowsAuto({ remaining: 16, limit: 100, resetAt: null })).toBe(true);
    expect(budgetAllowsAuto({ remaining: 15, limit: 100, resetAt: null })).toBe(false);
  });

  it("knows when a refresh is due and what it costs", () => {
    expect(isDue(0, 30 * 60_000, 30)).toBe(true);
    expect(isDue(0, 29 * 60_000, 30)).toBe(false);
    expect(isDue(0, 1e12, 0)).toBe(false);
    expect(callsPerDay(30)).toBe(48);
    expect(callsPerDay(0)).toBe(0);
  });
});

describe("temperatures", () => {
  it("clamps to tado's range in half degrees", () => {
    expect(clampTarget(21.26)).toBe(21.5);
    expect(clampTarget(2)).toBe(5);
    expect(clampTarget(30)).toBe(25);
    expect(formatTemp(20.4)).toBe("20,4°");
    expect(formatTemp(null)).toBe("–");
  });
});

describe("labels", () => {
  const now = new Date("2026-10-10T10:00:00");

  it("formats times relative to today", () => {
    expect(formatWhen("2026-10-10T21:00:00", now)).toBe("21:00");
    expect(formatWhen("2026-10-11T06:00:00", now)).toBe("morgen 06:00");
    expect(formatWhen("2026-10-13T06:00:00", now)).toMatch(/^Di\.? 06:00$/);
  });

  it("describes the mode", () => {
    expect(modeLabel(room, now)).toBe("Zeitplan");
    expect(modeLabel({ ...room, mode: "timer", until: "2026-10-10T12:00:00" }, now)).toBe("Manuell bis 12:00");
    expect(modeLabel({ ...room, mode: "manual" }, now)).toBe("Manuell, dauerhaft");
    expect(modeLabel({ ...room, mode: "boost", until: "2026-10-10T10:30:00" }, now)).toBe("Boost bis 10:30");
  });
});

describe("applyManual", () => {
  it("shows a change by hand before tado confirms it", () => {
    const now = Date.parse("2026-10-10T10:00:00Z");
    const timer = applyManual(room, 22.5, { kind: "timer", seconds: 3600 }, now);
    expect([timer.target, timer.mode, timer.until]).toEqual([22.5, "timer", "2026-10-10T11:00:00.000Z"]);
    const next = applyManual(room, 19, { kind: "nextBlock" }, now);
    expect(next.until).toBe(room.nextChange!.start);
    const off = applyManual(room, null, { kind: "manual" }, now);
    expect([off.power, off.target, off.until]).toEqual([false, null, null]);
  });
});
