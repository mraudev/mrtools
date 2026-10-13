import { afterEach, describe, expect, it, vi } from "vitest";
import { TextEditor } from "./editor";
import { findMatches, gotoMatch, replaceAll, replaceCurrent, searchState, setSearch } from "./search";
import type { EditorOptions } from "./types";

let editor: TextEditor | undefined;
afterEach(() => {
  editor?.destroy();
  editor = undefined;
});

function create(options: EditorOptions = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  editor = new TextEditor(host, options);
  return editor;
}

describe("content round trip", () => {
  it.each([
    ["text colour, font and size", '<p><span style="color: rgb(220, 38, 38); font-family: Georgia, serif; font-size: 14pt;">rot</span></p>'],
    ["highlight", '<p><mark data-color="#fef08a" style="background-color: #fef08a; color: inherit;">gelb</mark></p>'],
    ["task list", '<ul data-type="taskList"><li data-checked="true" data-type="taskItem"><label><input type="checkbox" checked="checked"><span></span></label><div><p>erledigt</p></div></li></ul>'],
    ["code block", "<pre><code>let x = 1;</code></pre>"],
  ])("keeps %s", (_, html) => {
    // The DOM may normalise colours (#hex → rgb); after that the HTML must be stable.
    const ed = create({ content: html });
    const once = ed.getHTML();
    ed.setHTML(once);
    expect(ed.getHTML()).toBe(once);
    expect(once.replace(/<[^>]+>/g, "")).toBe(html.replace(/<[^>]+>/g, ""));
    expect(once.match(/<[a-z]+/g)).toEqual(html.match(/<[a-z]+/g));
  });

  it("keeps tables and images", () => {
    const html = create({
      content: '<table><tbody><tr><th><p>A</p></th></tr><tr><td><p>1</p></td></tr></tbody></table><img src="data:image/png;base64,AAAA" alt="x">',
    }).getHTML();
    expect(html).toContain("<tr><th><p>A</p></th></tr><tr><td><p>1</p></td></tr>");
    expect(html).toContain('<img src="data:image/png;base64,AAAA" alt="x">');
  });

  it("keeps a format class next to an inline style", () => {
    const html = '<p><span class="marker">a</span><span style="color: rgb(220, 38, 38);">b</span></p>';
    expect(create({ content: html, characterFormats: [{ label: "M", class: "marker" }] }).getHTML()).toBe(html);
  });
});

describe("search", () => {
  it("finds matches inside text blocks, case-insensitive by default", () => {
    const ed = create({ content: "<p>Haus und haus</p><p>Maus</p>" });
    expect(findMatches(ed.tiptap.state.doc, "haus", false)).toEqual([
      { from: 1, to: 5 },
      { from: 10, to: 14 },
    ]);
    expect(findMatches(ed.tiptap.state.doc, "haus", true)).toHaveLength(1);
    expect(findMatches(ed.tiptap.state.doc, "", false)).toEqual([]);
  });

  it("goes through the matches and replaces one or all", () => {
    const onChange = vi.fn();
    const ed = create({ content: "<p>a b a b a</p>", onChange });
    setSearch(ed.tiptap, "a", false);
    expect(searchState(ed.tiptap).matches).toHaveLength(3);
    gotoMatch(ed.tiptap, 1);
    expect(searchState(ed.tiptap).current).toBe(1);
    replaceCurrent(ed.tiptap, "X");
    expect(ed.getText()).toBe("a b X b a");
    expect(replaceAll(ed.tiptap, "Y")).toBe(2);
    expect(ed.getText()).toBe("Y b X b Y");
    expect(onChange).toHaveBeenCalled();
  });

  it("opens the search bar with Ctrl+F and shows the count", () => {
    const ed = create({ content: "<p>eins zwei eins</p>", toolbar: ["search"] });
    ed.tiptap.view.dom.dispatchEvent(new KeyboardEvent("keydown", { key: "f", ctrlKey: true, bubbles: true }));
    const bar = ed.element.querySelector<HTMLElement>(".mrte-search")!;
    expect(bar.hidden).toBe(false);
    const input = bar.querySelector<HTMLInputElement>("input")!;
    input.value = "eins";
    input.dispatchEvent(new Event("input"));
    expect(bar.querySelector(".mrte-search-count")!.textContent).toBe("1 von 2");
    expect(ed.element.querySelectorAll(".mrte-match")).toHaveLength(2);
  });
});

describe("TextEditor extras", () => {
  it("inserts HTML at the cursor as a user change", () => {
    const onChange = vi.fn();
    const ed = create({ content: "<p>Hallo Welt</p>", onChange });
    ed.tiptap.commands.setTextSelection(7);
    ed.insertHTML("<strong>schöne </strong>");
    expect(ed.getHTML()).toBe("<p>Hallo <strong>schöne </strong>Welt</p>");
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("drops unknown classes from pasted HTML (Word) but keeps format classes", () => {
    const ed = create({ characterFormats: [{ label: "M", class: "marker" }] });
    const view = ed.tiptap.view;
    const cleaned = view.someProp("transformPastedHTML", (transform) =>
      transform('<p class="MsoNormal">a <span class="marker x">b</span> <span class="MsoFoo">c</span></p>', view),
    );
    expect(cleaned).toBe('<p>a <span class="marker">b</span> <span>c</span></p>');
  });

  it("shows words and characters in the optional status bar", () => {
    const ed = create({ content: "<p>zwei Wörter</p>", statusbar: true });
    const bar = ed.element.querySelector<HTMLElement>(".mrte-statusbar")!;
    expect(bar.hidden).toBe(false);
    expect(bar.textContent).toBe("2 Wörter · 11 Zeichen");
    ed.setConfig({ statusbar: false });
    expect(bar.hidden).toBe(true);
  });

  it("applies text colour and font size from the toolbar", () => {
    const ed = create({ content: "<p>Text</p>", toolbar: ["fontSize", "textColor"], colors: ["#ff0000"] });
    ed.tiptap.commands.setTextSelection({ from: 1, to: 5 });
    const size = ed.element.querySelector<HTMLSelectElement>(".mrte-toolbar select")!;
    size.value = String(["8pt", "9pt", "10pt", "11pt", "12pt", "14pt"].indexOf("14pt"));
    size.dispatchEvent(new Event("change"));
    ed.element.querySelector<HTMLButtonElement>('.mrte-toolbar [aria-label="Schriftfarbe"]')!.click();
    ed.element.querySelector<HTMLButtonElement>(".mrte-swatch")!.click();
    expect(ed.getHTML()).toMatch(/^<p><span style="color: (#ff0000|rgb\(255, 0, 0\)); font-size: 14pt;">Text<\/span><\/p>$/);
  });

  it("inserts a table from the grid and offers row/column commands inside it", () => {
    const ed = create({ toolbar: ["table"] });
    const button = ed.element.querySelector<HTMLButtonElement>('.mrte-toolbar [aria-label="Tabelle"]')!;
    button.click();
    ed.element.querySelector<HTMLButtonElement>('[aria-label="3 Spalten, 2 Zeilen"]')!.click();
    expect(ed.getHTML().match(/<tr>/g)).toHaveLength(2);
    expect(ed.getHTML().match(/<th>/g)).toHaveLength(3);
    button.click();
    const items = [...ed.element.querySelectorAll<HTMLButtonElement>(".mrte-menu-item")];
    items.find((item) => item.textContent === "Zeile darunter einfügen")!.click();
    expect(ed.getHTML().match(/<tr>/g)).toHaveLength(3);
  });
});

describe("empty document", () => {
  it("is only a single empty paragraph – a lone image or empty table is content", () => {
    const ed = create();
    expect(ed.getHTML()).toBe("");
    ed.setHTML('<img src="data:image/png;base64,AAAA">');
    expect(ed.getHTML()).toContain("<img");
    ed.setHTML("<table><tbody><tr><td><p></p></td></tr></tbody></table>");
    expect(ed.getHTML()).toContain("<table");
  });
});

describe("plain text", () => {
  it("has one line per paragraph and tab-separated table cells", () => {
    const ed = create({
      content:
        "<h1>T</h1><p>a<br>b</p><table><tbody><tr><th><p>A</p></th><th><p>B</p></th></tr><tr><td><p>1</p></td><td><p>2</p></td></tr></tbody></table><ul><li><p>x</p></li></ul><p></p><p>z</p>",
    });
    expect(ed.getText()).toBe("T\na\nb\nA\tB\n1\t2\nx\n\nz");
  });
});
