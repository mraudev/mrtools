import { describe, expect, it } from "vitest";
import { formatBytes, formatCpu, formatElapsed, formatRate } from "./format";

describe("formatBytes", () => {
  it("uses binary units with German decimals", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1,50 KB");
    expect(formatBytes(12.34 * 1024 ** 2)).toBe("12,3 MB");
    expect(formatRate(0)).toBe("0");
    expect(formatRate(2048)).toBe("2,00 KB/s");
  });
});

describe("formatCpu", () => {
  it("shows one decimal and tiny values", () => {
    expect(formatCpu(0)).toBe("0");
    expect(formatCpu(0.04)).toBe("< 0,1");
    expect(formatCpu(12.345)).toBe("12,3");
  });
});

describe("formatElapsed", () => {
  it("grows from minutes to days", () => {
    expect(formatElapsed(65_000)).toBe("01:05");
    expect(formatElapsed(3_725_000)).toBe("01:02:05");
    expect(formatElapsed(90_000_000)).toBe("1 T 01:00:00");
  });
});
