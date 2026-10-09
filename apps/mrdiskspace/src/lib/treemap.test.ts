import { describe, expect, it } from "vitest";
import { squarify } from "./treemap";

describe("squarify", () => {
  const rect = { x: 0, y: 0, w: 600, h: 400 };

  it("covers the area proportionally", () => {
    const values = [6, 6, 4, 3, 2, 2, 1];
    const rects = squarify(values, rect);
    expect(rects).toHaveLength(values.length);
    const total = values.reduce((a, b) => a + b, 0);
    rects.forEach((r, i) => expect(r.w * r.h).toBeCloseTo((values[i] / total) * 600 * 400, 6));
    for (const r of rects) {
      expect(r.x).toBeGreaterThanOrEqual(-1e-9);
      expect(r.y).toBeGreaterThanOrEqual(-1e-9);
      expect(r.x + r.w).toBeLessThanOrEqual(600 + 1e-9);
      expect(r.y + r.h).toBeLessThanOrEqual(400 + 1e-9);
    }
  });

  it("handles empty input", () => {
    expect(squarify([], rect)).toEqual([]);
    expect(squarify([0, 0], rect)).toEqual([
      { x: 0, y: 0, w: 0, h: 0 },
      { x: 0, y: 0, w: 0, h: 0 },
    ]);
  });
});
