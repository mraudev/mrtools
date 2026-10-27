import { describe, expect, it } from "vitest";
import { changesBelow, prepare, relative, statusOf } from "./git";

const repo = prepare({
  root: "C:\\dev\\Repo",
  branch: "main",
  upstream: "origin/main",
  ahead: 1,
  behind: 0,
  files: { "src/Store.ts": "modified", "README.md": "staged", "notes.txt": "untracked", "src/debug.log": "ignored" },
  whole: { scratch: "untracked", node_modules: "ignored" },
  dirs: { src: "modified" },
});

const file = (path: string) => ({ path: `C:\\dev\\repo\\${path}`, isDir: false });
const dir = (path: string) => ({ path: `C:\\dev\\repo\\${path}`, isDir: true });

describe("git status", () => {
  it("makes paths relative to the repository", () => {
    expect(relative("C:\\Dev\\Repo\\src\\a.ts", "c:\\dev\\repo")).toBe("src/a.ts");
    expect(relative("C:\\dev\\repo", "C:\\dev\\repo\\")).toBe("");
    expect(relative("C:\\dev\\repository\\a", "C:\\dev\\repo")).toBe(null);
  });

  it("reports files case-insensitively and unchanged ones as clean", () => {
    expect(statusOf(repo, file("SRC\\store.ts"), true)).toBe("modified");
    expect(statusOf(repo, file("README.md"), true)).toBe("staged");
    expect(statusOf(repo, file("src\\other.ts"), true)).toBe("clean");
    expect(repo.changed).toBe(2);
  });

  it("aggregates folders and inherits untracked or ignored folders", () => {
    expect(statusOf(repo, dir("src"), true)).toBe("modified");
    expect(statusOf(repo, dir("docs"), true)).toBe("clean");
    expect(statusOf(repo, dir("scratch"), true)).toBe("untracked");
    expect(statusOf(repo, file("scratch\\a\\b.txt"), true)).toBe("untracked");
    expect(statusOf(repo, file("node_modules\\x\\index.js"), true)).toBe("ignored");
  });

  it("lists changes below a folder in git's spelling, most important first", () => {
    expect(changesBelow(repo, "").map((c) => c.path)).toEqual(["src/Store.ts", "README.md", "notes.txt", "scratch/"]);
    expect(changesBelow(repo, "src").map((c) => [c.path, c.status])).toEqual([["src/Store.ts", "modified"]]);
    expect(changesBelow(repo, "docs")).toEqual([]);
  });

  it("hides ignored entries on request and shows nothing outside or for .git", () => {
    expect(statusOf(repo, file("src\\debug.log"), false)).toBe(null);
    expect(statusOf(repo, dir("node_modules"), false)).toBe(null);
    expect(statusOf(repo, dir(".git"), true)).toBe(null);
    expect(statusOf(repo, { path: "D:\\other", isDir: true }, true)).toBe(null);
    expect(statusOf(null, file("a"), true)).toBe(null);
  });
});
