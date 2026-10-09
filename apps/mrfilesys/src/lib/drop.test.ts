import { describe, expect, it } from "vitest";
import { dropOp } from "./drop";

describe("dropOp", () => {
  it("moves within a drive and copies across drives", () => {
    expect(dropOp(["C:\\a\\x.txt"], "C:\\b")).toBe("move");
    expect(dropOp(["C:\\a\\x.txt"], "D:\\b")).toBe("copy");
    expect(dropOp(["\\\\nas\\share\\x"], "\\\\nas\\share\\y")).toBe("move");
    expect(dropOp(["\\\\nas\\share\\x"], "\\\\nas\\other")).toBe("copy");
  });

  it("follows Ctrl and Shift", () => {
    expect(dropOp(["C:\\a\\x.txt"], "C:\\b", { ctrl: true })).toBe("copy");
    expect(dropOp(["C:\\a\\x.txt"], "D:\\b", { shift: true })).toBe("move");
  });

  it("rejects drops that are impossible or change nothing", () => {
    expect(dropOp(["C:\\a"], "C:\\a")).toBe(null);
    expect(dropOp(["C:\\a"], "C:\\a\\sub")).toBe(null);
    expect(dropOp(["C:\\a\\x.txt"], "C:\\a")).toBe(null);
    expect(dropOp(["C:\\a\\x.txt"], "c:\\A", { ctrl: true })).toBe("copy");
    expect(dropOp([], "C:\\a")).toBe(null);
    expect(dropOp(["C:\\a\\x.txt"], "")).toBe(null);
  });
});
