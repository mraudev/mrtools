import { describe, expect, it } from "vitest";
import { iconKey } from "./icons";

describe("iconKey", () => {
  it("shares one icon per extension for plain files", () => {
    expect(iconKey({ name: "Bericht.PDF", isDir: false, path: "C:\\a\\Bericht.PDF" })).toBe("ext:.pdf");
    expect(iconKey({ name: "Makefile", isDir: false, path: "C:\\a\\Makefile" })).toBe("ext:");
  });

  it("asks per path for programs, shortcuts and folders", () => {
    expect(iconKey({ name: "app.exe", isDir: false, path: "C:\\a\\app.exe" })).toBe("path:C:\\a\\app.exe");
    expect(iconKey({ name: "Start.lnk", isDir: false, path: "C:\\a\\Start.lnk" })).toBe("path:C:\\a\\Start.lnk");
    expect(iconKey({ name: "Desktop", isDir: true, path: "C:\\Users\\me\\Desktop" })).toBe("path:C:\\Users\\me\\Desktop");
  });

  it("falls back to generic icons without a path", () => {
    expect(iconKey({ name: "app.exe", isDir: false })).toBe("ext:.exe");
    expect(iconKey({ name: "Ordner", isDir: true })).toBe("dir");
  });
});
