import { describe, expect, it } from "vitest";
import { normalizeConfig, normalizeSettings, orderTabs, watchedCategory } from "./config";
import { defaultSettings } from "./types";

describe("normalizeSettings", () => {
  it("fills defaults for an empty or missing config", () => {
    expect(normalizeSettings(undefined)).toEqual(defaultSettings());
    expect(normalizeSettings({})).toEqual(defaultSettings());
  });

  it("drops unknown keys such as tokens from old radstart files", () => {
    const settings = normalizeSettings({ githubToken: "geheim", giteaToken: "x", theme: "light" }) as any;
    expect(settings.githubToken).toBeUndefined();
    expect(settings.giteaToken).toBeUndefined();
    expect(settings.theme).toBe("light");
  });

  it("ignores values of the wrong type", () => {
    const settings = normalizeSettings({ compactTiles: "ja", defaultApps: "*.sln", pinnedPaths: [1, "C:\\a"] });
    expect(settings.compactTiles).toBe(false);
    expect(settings.defaultApps).toEqual(defaultSettings().defaultApps);
    expect(settings.pinnedPaths).toEqual(["C:\\a"]);
  });

  it("only accepts the offered auto-fetch intervals and defaults to off", () => {
    expect(normalizeSettings({}).autoFetchMinutes).toBe(0);
    expect(normalizeSettings({ autoFetchMinutes: 15 }).autoFetchMinutes).toBe(15);
    expect(normalizeSettings({ autoFetchMinutes: 1 }).autoFetchMinutes).toBe(0);
  });

  it("migrates the old single watched list to one tab per folder", () => {
    const settings = normalizeSettings({ watchedDirectories: ["C:\\development", "D:\\work\\"] });
    expect(settings.watchedFolders).toEqual([
      { path: "C:\\development", category: "development" },
      { path: "D:\\work\\", category: "work" },
    ]);
  });

  it("keeps configured tabs of watched folders", () => {
    const settings = normalizeSettings({ watchedFolders: [{ path: "C:\\dev", category: "hotfix" }, { path: "" }] });
    expect(settings.watchedFolders).toEqual([{ path: "C:\\dev", category: "hotfix" }]);
  });
});

describe("normalizeConfig", () => {
  it("accepts radstart projects with numeric ids and versions", () => {
    const config = normalizeConfig({ projects: [{ id: 3, name: "app", category: "hotfix", path: "C:\\x", version: 514 }] });
    expect(config.projects[0]).toMatchObject({ id: "3", version: "514", apps: [], commands: [] });
  });
});

describe("watchedCategory", () => {
  it("falls back to the folder name when the tab name is empty", () => {
    expect(watchedCategory({ path: "C:\\development\\", category: " " })).toBe("development");
  });
});

describe("orderTabs", () => {
  it("follows the saved order and appends new tabs alphabetically", () => {
    expect(orderTabs(["b", "c", "a", "neu"], ["c", "a"])).toEqual(["c", "a", "b", "neu"]);
  });
});
