import { describe, expect, it } from "vitest";
import { hsvToRgb, kelvinToMirek, kelvinToRgb, mirekToKelvin, rgbToXy, xyToRgb } from "./color";

describe("colour conversions", () => {
  it("converts hsv", () => {
    expect(hsvToRgb(0, 1, 1)).toEqual([255, 0, 0]);
    expect(hsvToRgb(120, 1, 1)).toEqual([0, 255, 0]);
    expect(hsvToRgb(0, 0, 1)).toEqual([255, 255, 255]);
  });

  it("maps rgb to xy and back to a similar colour", () => {
    const [x, y] = rgbToXy([255, 0, 0]);
    expect(x).toBeGreaterThan(0.6);
    expect(y).toBeLessThan(0.35);
    const [r, g, b] = xyToRgb(rgbToXy([0, 0, 255]));
    expect(b).toBeGreaterThan(200);
    expect(r).toBeLessThan(80);
    expect(g).toBeLessThan(80);
    expect(rgbToXy([0, 0, 0])).toEqual([0.3227, 0.329]);
  });

  it("handles white temperatures", () => {
    expect(mirekToKelvin(500)).toBe(2000);
    expect(kelvinToMirek(6500)).toBe(154);
    const warm = kelvinToRgb(2700);
    const cold = kelvinToRgb(6500);
    expect(warm[0]).toBe(255);
    expect(warm[2]).toBeLessThan(cold[2]);
  });
});
