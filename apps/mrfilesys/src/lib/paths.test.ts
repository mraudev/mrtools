import { describe, expect, it } from "vitest";
import { baseName, crumbs, isInside, joinPath, normalizePath, parentPath } from "./paths";

describe("normalizePath", () => {
  it("makes drive letters roots and drops trailing backslashes", () => {
    expect(normalizePath("c:")).toBe("C:\\");
    expect(normalizePath(" c:/Users/ ")).toBe("C:\\Users");
    expect(normalizePath("C:\\")).toBe("C:\\");
    expect(normalizePath("\\\\nas\\share\\")).toBe("\\\\nas\\share\\");
  });
});

describe("parentPath", () => {
  it("walks up to the drive and then to 'Dieser PC'", () => {
    expect(parentPath("C:\\Users\\me")).toBe("C:\\Users");
    expect(parentPath("C:\\Users")).toBe("C:\\");
    expect(parentPath("C:\\")).toBe("");
    expect(parentPath("")).toBe("");
    expect(parentPath("\\\\nas\\share\\dir")).toBe("\\\\nas\\share");
    expect(parentPath("\\\\nas\\share")).toBe("");
  });
});

describe("names", () => {
  it("joins and splits", () => {
    expect(joinPath("C:\\", "a")).toBe("C:\\a");
    expect(joinPath("C:\\x", "a")).toBe("C:\\x\\a");
    expect(baseName("C:\\x\\a.txt")).toBe("a.txt");
    expect(baseName("D:\\")).toBe("D:");
  });

  it("builds breadcrumbs", () => {
    expect(crumbs("C:\\Users\\me")).toEqual([
      { label: "C:", path: "C:\\" },
      { label: "Users", path: "C:\\Users" },
      { label: "me", path: "C:\\Users\\me" },
    ]);
    expect(crumbs("")).toEqual([]);
  });

  it("checks containment case-insensitively", () => {
    expect(isInside("C:\\Users\\Me\\x", "c:\\users\\me")).toBe(true);
    expect(isInside("C:\\Users\\Me", "C:\\Users\\Me")).toBe(true);
    expect(isInside("C:\\Users\\Meier", "C:\\Users\\Me")).toBe(false);
    expect(isInside("C:\\a", "C:\\")).toBe(true);
  });
});
