import { describe, expect, it } from "vitest";
import { groupApps, sortApps } from "./apps";
import type { App, Release } from "./types";

const app = (folder: string, version?: string | null): App => ({
  folder,
  name: folder,
  description: null,
  icon: null,
  installed: version === undefined ? null : { version, exe: null },
});
const release = (version: string): Release => ({ version, url: "", publishedAt: null, installerSize: null });

const releases = { mrports: release("1.1.0"), mrprocs: release("1.1.0"), mrhome: release("1.0.1"), mrstart: release("1.8.0") };

describe("sortApps", () => {
  it("puts updates first, then installed, then the rest", () => {
    const apps = [app("mrstart"), app("mrprocs", "1.1.0"), app("mrhome", "1.0.1"), app("mrports", "1.0.0"), app("mrfoo", null)];
    expect(sortApps(apps, releases).map((a) => a.folder)).toEqual(["mrports", "mrfoo", "mrhome", "mrprocs", "mrstart"]);
  });
});

describe("groupApps", () => {
  it("groups by category in a fixed order, unknown apps last", () => {
    const apps = [app("mrhome", "1.0.1"), app("mrnew"), app("mrprocs", "1.1.0"), app("mrports", "1.0.0"), app("mrstart")];
    const groups = groupApps(apps, releases);
    expect(groups.map((g) => g.name)).toEqual(["System", "Entwicklung & Werkzeuge", "Zuhause", "Sonstige"]);
    expect(groups[0].apps.map((a) => a.folder)).toEqual(["mrports", "mrprocs"]);
  });
});
