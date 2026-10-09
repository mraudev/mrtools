import { describe, expect, it } from "vitest";
import { formatBytes, formatDuration, formatPercent } from "./format";

describe("formatBytes", () => {
  it("uses binary units with German decimals", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1536)).toBe("1,50 KB");
    expect(formatBytes(150 * 1024 ** 3)).toBe("150 GB");
    expect(formatBytes(12.34 * 1024 ** 2)).toBe("12,3 MB");
  });
});

describe("formatPercent", () => {
  it("handles an empty whole", () => {
    expect(formatPercent(1, 0)).toBe("–");
    expect(formatPercent(1, 4)).toBe("25,0 %");
  });
});

describe("formatDuration", () => {
  it("switches to minutes", () => {
    expect(formatDuration(1500)).toBe("1,5 s");
    expect(formatDuration(125_000)).toBe("2 min 5 s");
  });
});
