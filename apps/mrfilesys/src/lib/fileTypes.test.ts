import { describe, expect, it } from "vitest";
import { extension, kindOf, previewOf, sortEntries, typeLabel } from "./fileTypes";
import type { Entry } from "./types";

const entry = (name: string, isDir = false, size = 0, modified = 0): Entry => ({
  name,
  path: `C:\\${name}`,
  isDir,
  size,
  modified,
  created: 0,
  hidden: false,
  system: false,
  readonly: false,
  link: false,
});

describe("file types", () => {
  it("reads extensions", () => {
    expect(extension("a.TXT")).toBe("txt");
    expect(extension("archive.tar.gz")).toBe("gz");
    expect(extension("Makefile")).toBe("");
    expect(extension("a.")).toBe("");
  });

  it("classifies", () => {
    expect(kindOf(entry("x.png"))).toBe("image");
    expect(kindOf(entry("x.png", true))).toBe("folder");
    expect(kindOf(entry("x.unknown"))).toBe("file");
    expect(typeLabel(entry("x.png"))).toBe("PNG-Datei");
    expect(typeLabel(entry("x"))).toBe("Datei");
    expect(typeLabel(entry("x", true))).toBe("Dateiordner");
    expect(previewOf(entry("x.jpg"))).toBe("image");
    expect(previewOf(entry("x.rs"))).toBe("text");
    expect(previewOf(entry("x.zip"))).toBe(null);
  });
});

describe("sortEntries", () => {
  const list = [entry("b10.txt", false, 5, 3), entry("B2.txt", false, 50, 1), entry("z", true), entry("a.png", false, 1, 2)];
  const names = (key: Parameters<typeof sortEntries>[1], asc: boolean) =>
    sortEntries(list, key, asc).map((e) => e.name);

  it("keeps folders first and sorts names naturally", () => {
    expect(names("name", true)).toEqual(["z", "a.png", "B2.txt", "b10.txt"]);
    expect(names("name", false)).toEqual(["z", "b10.txt", "B2.txt", "a.png"]);
  });

  it("sorts by size, date and type", () => {
    expect(names("size", false)).toEqual(["z", "B2.txt", "b10.txt", "a.png"]);
    expect(names("modified", true)).toEqual(["z", "B2.txt", "a.png", "b10.txt"]);
    expect(names("type", true)).toEqual(["z", "a.png", "B2.txt", "b10.txt"]);
  });
});
