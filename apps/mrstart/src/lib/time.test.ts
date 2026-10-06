import { describe, expect, it } from "vitest";
import { ago, olderThan, WEEK_MS } from "./time";

describe("ago", () => {
  it("formats relative times in German", () => {
    expect(ago(Date.now())).toBe("gerade eben");
    expect(ago(Date.now() - 3 * 86_400_000)).toBe("vor 3 Tagen");
    expect(ago(Date.now() - 2 * 3_600_000)).toBe("vor 2 Stunden");
  });

  it("returns an empty string for invalid dates", () => {
    expect(ago("kein Datum")).toBe("");
  });
});

describe("olderThan", () => {
  it("detects pull requests without activity for more than a week", () => {
    expect(olderThan(new Date(Date.now() - 8 * 86_400_000).toISOString(), WEEK_MS)).toBe(true);
    expect(olderThan(new Date(Date.now() - 2 * 86_400_000).toISOString(), WEEK_MS)).toBe(false);
    expect(olderThan("kein Datum", WEEK_MS)).toBe(false);
  });
});
