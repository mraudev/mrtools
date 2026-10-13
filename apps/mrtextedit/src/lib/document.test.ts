// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { fileName, fromFile, toDocument } from "./document";

describe("document", () => {
  it("round-trips the editor HTML through a saved document", () => {
    const body = '<h1>Titel</h1><p class="zitat">a &amp; b</p>';
    const saved = toDocument(body, "Brief <1>", "p.zitat { font-style: italic }");
    expect(saved).toContain("<title>Brief &lt;1&gt;</title>");
    expect(saved).toContain("p.zitat { font-style: italic }");
    expect(fromFile(saved, "C:\\brief.html").trim()).toBe(body);
  });

  it("keeps HTML fragments and turns text lines into paragraphs", () => {
    expect(fromFile("<p>x</p>", "a.html")).toBe("<p>x</p>");
    expect(fromFile("a < b\r\nzwei", "notiz.TXT")).toBe("<p>a &lt; b</p><p>zwei</p>");
  });

  it("takes the file name from a Windows or Unix path", () => {
    expect(fileName("C:\\Texte\\brief.html")).toBe("brief.html");
    expect(fileName("/home/x/a.txt")).toBe("a.txt");
  });
});
