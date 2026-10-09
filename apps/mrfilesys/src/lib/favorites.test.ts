import { describe, expect, it } from "vitest";
import { insertFavorites, readFavorites, samePlace, type Favorite } from "./favorites";

const fav = (path: string, isDir = true): Favorite => ({ path, isDir });
const paths = (list: Favorite[]) => list.map((f) => f.path);
const list = [fav("C:\\a"), fav("C:\\b"), fav("C:\\c")];

describe("readFavorites", () => {
  it("takes over plain folder paths of older versions", () => {
    expect(readFavorites(["C:\\a", { path: "C:\\x.txt", isDir: false }, 3, null])).toEqual([
      fav("C:\\a"),
      fav("C:\\x.txt", false),
    ]);
    expect(readFavorites(undefined)).toEqual([]);
  });
});

describe("insertFavorites", () => {
  it("pins new items at the index", () => {
    expect(paths(insertFavorites(list, [fav("C:\\n")], 1))).toEqual(["C:\\a", "C:\\n", "C:\\b", "C:\\c"]);
    expect(paths(insertFavorites(list, [fav("C:\\n")], 3))).toEqual(["C:\\a", "C:\\b", "C:\\c", "C:\\n"]);
    expect(paths(insertFavorites([], [fav("C:\\n"), fav("c:\\N")], 0))).toEqual(["C:\\n"]);
  });

  it("moves existing items instead of duplicating them", () => {
    expect(paths(insertFavorites(list, [fav("C:\\a")], 3))).toEqual(["C:\\b", "C:\\c", "C:\\a"]);
    expect(paths(insertFavorites(list, [fav("C:\\c")], 0))).toEqual(["C:\\c", "C:\\a", "C:\\b"]);
    expect(paths(insertFavorites(list, [fav("c:\\A")], 2))).toEqual(["C:\\b", "c:\\A", "C:\\c"]);
  });

  it("knows when a reorder changes nothing", () => {
    expect(samePlace(list, ["C:\\b"], 1)).toBe(true);
    expect(samePlace(list, ["C:\\b"], 2)).toBe(true);
    expect(samePlace(list, ["C:\\b"], 0)).toBe(false);
    expect(samePlace(list, ["C:\\n"], 3)).toBe(false);
  });
});
