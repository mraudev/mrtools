import { describe, expect, it } from "vitest";
import { appStatus, compareVersions } from "./version";

describe("compareVersions", () => {
  it("compares numerically", () => {
    expect(compareVersions("1.10.0", "1.9.2")).toBe(1);
    expect(compareVersions("v1.4.0", "1.7.0")).toBe(-1);
    expect(compareVersions("1.2", "1.2.0")).toBe(0);
  });

  it("sorts pre-releases before the release", () => {
    expect(compareVersions("2.0.0-beta.1", "2.0.0")).toBe(-1);
    expect(compareVersions("2.0.0", "2.0.0-rc.1")).toBe(1);
  });
});

describe("appStatus", () => {
  it("detects missing, outdated and current installs", () => {
    expect(appStatus(undefined, "1.0.0")).toEqual({ kind: "missing" });
    expect(appStatus("1.4.0", "1.8.0")).toEqual({ kind: "outdated", latest: "1.8.0" });
    expect(appStatus("1.8.0", "1.8.0")).toEqual({ kind: "current", latest: "1.8.0" });
    expect(appStatus("1.8.0", null)).toEqual({ kind: "current", latest: null });
    expect(appStatus(null, "1.8.0")).toEqual({ kind: "current", latest: "1.8.0" });
  });
});
