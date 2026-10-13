import { afterEach, describe, expect, it, vi } from "vitest";
import { TextEditor } from "./editor";
import { findParagraphFormat, formatsCss, resolveConfig } from "./formats";
import type { EditorConfig } from "./types";

const config: EditorConfig = {
  toolbar: ["paragraphFormat", "characterFormat", "|", "bold"],
  paragraphFormats: [
    { label: "Standard" },
    { label: "Überschrift", tag: "h2" },
    { label: "Zitat", class: "zitat", css: "font-style: italic" },
  ],
  characterFormats: [{ label: "Markiert", class: "marker", css: "background: yellow" }],
};

let editor: TextEditor | undefined;
afterEach(() => {
  editor?.destroy();
  editor = undefined;
});

function create(options: EditorConfig & { content?: string; onChange?: () => void } = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  editor = new TextEditor(host, { ...config, ...options });
  return editor;
}

const selects = () => [...editor!.element.querySelectorAll<HTMLSelectElement>("select")];

describe("resolveConfig", () => {
  it("fills in defaults", () => {
    const resolved = resolveConfig();
    expect(resolved.paragraphFormats.map((f) => f.tag)).toEqual(["p", "h1", "h2", "h3"]);
    expect(resolved.characterFormats).toEqual([]);
    expect(resolved.toolbar).toContain("bold");
  });

  it("drops unknown toolbar items, tags and invalid class names", () => {
    const resolved = resolveConfig({
      toolbar: ["bold", "nope" as never, "|"],
      paragraphFormats: [{ label: "A", tag: "div" as never }, { label: "B", class: "x y" }, { label: "C", class: "ok" }],
      characterFormats: [{ label: "D", class: "" }, { label: "E", class: "e-1" }],
    });
    expect(resolved.toolbar).toEqual(["bold", "|"]);
    expect(resolved.paragraphFormats.map((f) => f.label)).toEqual(["C"]);
    expect(resolved.characterFormats.map((f) => f.label)).toEqual(["E"]);
  });
});

describe("formats", () => {
  it("finds the paragraph format by tag and class", () => {
    const formats = resolveConfig(config).paragraphFormats;
    expect(findParagraphFormat(formats, "p", null)).toBe(0);
    expect(findParagraphFormat(formats, "h2", null)).toBe(1);
    expect(findParagraphFormat(formats, "p", "zitat")).toBe(2);
    expect(findParagraphFormat(formats, "p", "other")).toBe(-1);
  });

  it("builds scoped CSS and keeps declarations inside their rule", () => {
    expect(formatsCss(config, ".scope")).toBe(
      ".scope p.zitat { font-style: italic }\n.scope span.marker { background: yellow }",
    );
    expect(formatsCss({ characterFormats: [{ label: "x", class: "x", css: "color: red } body { color: red" }] })).toBe(
      "span.x { color: red  body  color: red }",
    );
  });
});

describe("TextEditor", () => {
  it("keeps paragraph and character formats when HTML goes in and out", () => {
    const html =
      '<h2>Titel</h2><p class="zitat">Ein <span class="marker">wichtiges</span> Zitat</p><p style="text-align: center;">Mitte</p>';
    expect(create({ content: html }).getHTML()).toBe(html);
  });

  it("returns an empty string for an empty document and text with one line per paragraph", () => {
    const ed = create();
    expect(ed.getHTML()).toBe("");
    ed.setHTML("<p>a</p><p>b</p>");
    expect(ed.getText()).toBe("a\nb");
  });

  it("does not report setHTML as a change", () => {
    const onChange = vi.fn();
    const ed = create({ onChange });
    ed.setHTML("<p>x</p>");
    expect(onChange).not.toHaveBeenCalled();
    ed.tiptap.commands.insertContent("y");
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("applies a paragraph format from the combobox", () => {
    const ed = create({ content: "<p>Text</p>" });
    ed.tiptap.commands.setTextSelection(2);
    const [paragraph] = selects();
    expect(paragraph.value).toBe("0");
    paragraph.value = "2";
    paragraph.dispatchEvent(new Event("change"));
    expect(ed.getHTML()).toBe('<p class="zitat">Text</p>');
    paragraph.value = "1";
    paragraph.dispatchEvent(new Event("change"));
    expect(ed.getHTML()).toBe("<h2>Text</h2>");
    expect(paragraph.value).toBe("1");
  });

  it("applies and removes a character format from the combobox", () => {
    const ed = create({ content: "<p>Hallo Welt</p>" });
    ed.tiptap.commands.setTextSelection({ from: 1, to: 6 });
    const [, character] = selects();
    character.value = "0";
    character.dispatchEvent(new Event("change"));
    expect(ed.getHTML()).toBe('<p><span class="marker">Hallo</span> Welt</p>');
    character.value = "";
    character.dispatchEvent(new Event("change"));
    expect(ed.getHTML()).toBe("<p>Hallo Welt</p>");
  });

  it("builds the toolbar from the configuration and rebuilds it on setConfig", () => {
    const ed = create();
    expect(selects()).toHaveLength(2);
    expect(ed.element.querySelectorAll(".mrte-toolbar .mrte-btn")).toHaveLength(1);
    ed.setConfig({ toolbar: ["undo", "redo", "characterFormat"] });
    // no character formats configured → no combobox
    expect(selects()).toHaveLength(0);
    expect(ed.element.querySelectorAll(".mrte-toolbar .mrte-btn")).toHaveLength(2);
    ed.setConfig({ toolbar: [] });
    expect(ed.element.querySelector<HTMLElement>(".mrte-toolbar")!.hidden).toBe(true);
  });

  it("shows the HTML source, takes edits from it and locks the other controls meanwhile", () => {
    const onChange = vi.fn();
    const ed = create({ content: "<p>a</p><ul><li><p>b</p></li></ul>", onChange, toolbar: ["paragraphFormat", "source"] });
    const source = ed.element.querySelector<HTMLTextAreaElement>(".mrte-source")!;
    ed.element.querySelector<HTMLButtonElement>(".mrte-toolbar .mrte-btn")!.click();
    expect(ed.sourceMode).toBe(true);
    expect(source.value).toBe("<p>a</p>\n<ul>\n  <li><p>b</p></li>\n</ul>");
    expect(selects()[0].disabled).toBe(true);

    source.value = '<p class="zitat">neu</p>';
    source.dispatchEvent(new Event("input"));
    expect(ed.getHTML()).toBe('<p class="zitat">neu</p>');
    expect(onChange).toHaveBeenCalledTimes(1);

    ed.toggleSource(false);
    expect(source.hidden).toBe(true);
    expect(selects()[0].value).toBe("2");
  });

  it("leaves the source view when the toolbar no longer offers it", () => {
    const ed = create({ toolbar: ["source"] });
    ed.toggleSource(true);
    ed.setConfig({ toolbar: ["bold"] });
    expect(ed.sourceMode).toBe(false);
  });

  it("disables editing and the toolbar when read-only", () => {
    const ed = create({ content: "<p>x</p>", readOnly: true });
    expect(ed.tiptap.isEditable).toBe(false);
    expect(selects().every((s) => s.disabled)).toBe(true);
    ed.setReadOnly(false);
    expect(ed.tiptap.isEditable).toBe(true);
  });
});
