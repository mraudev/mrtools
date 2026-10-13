import type { ChainedCommands, Editor } from "@tiptap/core";
import {
  Baseline,
  Bold,
  Code,
  CodeXml,
  createElement,
  Highlighter,
  type IconNode,
  Image,
  Italic,
  Link,
  List,
  ListIndentDecrease,
  ListIndentIncrease,
  ListOrdered,
  ListTodo,
  Maximize2,
  Minimize2,
  Minus,
  Omega,
  Printer,
  Redo2,
  RemoveFormatting,
  Search,
  SquareCode,
  Strikethrough,
  Subscript,
  Superscript,
  Table,
  TextAlignCenter,
  TextAlignEnd,
  TextAlignJustify,
  TextAlignStart,
  TextQuote,
  Underline,
  Undo2,
} from "lucide";
import { findParagraphFormat, type ResolvedConfig } from "./formats";
import type { ToolbarItem } from "./types";

/** What the toolbar switches but the editor owns (views, search panel, images, printing). */
export interface EditorUi {
  sourceMode(): boolean;
  toggleSource(): void;
  searchOpen(): boolean;
  toggleSearch(): void;
  fullscreen(): boolean;
  toggleFullscreen(): void;
  print(): void;
  insertImageFiles(files: File[]): void;
}

export interface Toolbar {
  update(): void;
  destroy(): void;
}

interface ButtonDef {
  title: string;
  icon: IconNode;
  run: (chain: ChainedCommands) => ChainedCommands;
  active?: (editor: Editor) => boolean;
}

function alignment(editor: Editor): string {
  return editor.getAttributes(editor.isActive("heading") ? "heading" : "paragraph").textAlign ?? "left";
}

const BUTTONS: Partial<Record<ToolbarItem, ButtonDef>> = {
  undo: { title: "Rückgängig (Strg+Z)", icon: Undo2, run: (c) => c.undo() },
  redo: { title: "Wiederholen (Strg+Y)", icon: Redo2, run: (c) => c.redo() },
  bold: { title: "Fett (Strg+B)", icon: Bold, run: (c) => c.toggleBold(), active: (e) => e.isActive("bold") },
  italic: { title: "Kursiv (Strg+I)", icon: Italic, run: (c) => c.toggleItalic(), active: (e) => e.isActive("italic") },
  underline: {
    title: "Unterstrichen (Strg+U)",
    icon: Underline,
    run: (c) => c.toggleUnderline(),
    active: (e) => e.isActive("underline"),
  },
  strike: {
    title: "Durchgestrichen",
    icon: Strikethrough,
    run: (c) => c.toggleStrike(),
    active: (e) => e.isActive("strike"),
  },
  subscript: {
    title: "Tiefgestellt",
    icon: Subscript,
    run: (c) => c.toggleSubscript(),
    active: (e) => e.isActive("subscript"),
  },
  superscript: {
    title: "Hochgestellt",
    icon: Superscript,
    run: (c) => c.toggleSuperscript(),
    active: (e) => e.isActive("superscript"),
  },
  code: { title: "Code", icon: Code, run: (c) => c.toggleCode(), active: (e) => e.isActive("code") },
  alignLeft: {
    title: "Linksbündig",
    icon: TextAlignStart,
    run: (c) => c.setTextAlign("left"),
    active: (e) => alignment(e) === "left",
  },
  alignCenter: {
    title: "Zentriert",
    icon: TextAlignCenter,
    run: (c) => c.setTextAlign("center"),
    active: (e) => alignment(e) === "center",
  },
  alignRight: {
    title: "Rechtsbündig",
    icon: TextAlignEnd,
    run: (c) => c.setTextAlign("right"),
    active: (e) => alignment(e) === "right",
  },
  alignJustify: {
    title: "Blocksatz",
    icon: TextAlignJustify,
    run: (c) => c.setTextAlign("justify"),
    active: (e) => alignment(e) === "justify",
  },
  bulletList: {
    title: "Aufzählung",
    icon: List,
    run: (c) => c.toggleBulletList(),
    active: (e) => e.isActive("bulletList"),
  },
  orderedList: {
    title: "Nummerierung",
    icon: ListOrdered,
    run: (c) => c.toggleOrderedList(),
    active: (e) => e.isActive("orderedList"),
  },
  taskList: {
    title: "Checkliste",
    icon: ListTodo,
    run: (c) => c.toggleTaskList(),
    active: (e) => e.isActive("taskList"),
  },
  indent: {
    title: "Listeneinzug vergrößern (Tab)",
    icon: ListIndentIncrease,
    run: (c) => c.command(({ commands }) => commands.sinkListItem("listItem") || commands.sinkListItem("taskItem")),
  },
  outdent: {
    title: "Listeneinzug verkleinern (Umschalt+Tab)",
    icon: ListIndentDecrease,
    run: (c) => c.command(({ commands }) => commands.liftListItem("listItem") || commands.liftListItem("taskItem")),
  },
  blockquote: {
    title: "Zitat",
    icon: TextQuote,
    run: (c) => c.toggleBlockquote(),
    active: (e) => e.isActive("blockquote"),
  },
  codeBlock: {
    title: "Codeblock",
    icon: SquareCode,
    run: (c) => c.toggleCodeBlock(),
    active: (e) => e.isActive("codeBlock"),
  },
  horizontalRule: { title: "Trennlinie", icon: Minus, run: (c) => c.setHorizontalRule() },
  clearFormat: {
    title: "Formatierung entfernen",
    icon: RemoveFormatting,
    run: (c) => c.unsetAllMarks().clearNodes(),
  },
};

const SPECIAL_CHARS = [
  "–", "—", "„", "“", "‚", "‘", "»", "«", "…", "·", "•", "€", "£", "$", "¥", "©", "®", "™", "§", "¶",
  "°", "±", "×", "÷", "≠", "≈", "≤", "≥", "∞", "√", "½", "¼", "¾", "²", "³", "µ", "←", "→", "↑", "↓",
  "⇒", "↔", "✓", "✗", "★", "♥", "☐", "☑", "Ø", "∑", "Δ", "Ω", "α", "β", "π", "†", "‰", "¿", "¡",
];

function el<K extends keyof HTMLElementTagNameMap>(tag: K, className: string): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  element.className = className;
  return element;
}

function iconButton(title: string, icon: IconNode): HTMLButtonElement {
  const button = el("button", "mrte-btn");
  button.type = "button";
  button.title = title;
  button.setAttribute("aria-label", title);
  button.append(createElement(icon, { "aria-hidden": "true" }));
  // Keeps the focus (and selection) in the text while clicking.
  button.addEventListener("mousedown", (event) => event.preventDefault());
  return button;
}

function textButton(label: string, className = "mrte-text-btn"): HTMLButtonElement {
  const button = el("button", className);
  button.type = "button";
  button.textContent = label;
  return button;
}

function select(title: string, className = "mrte-select"): HTMLSelectElement {
  const element = el("select", className);
  element.title = title;
  element.setAttribute("aria-label", title);
  return element;
}

/** Font names as the browser normalises them (`"Courier New", monospace` ↔ `Courier New, monospace`). */
function fontKey(value: unknown): string {
  return typeof value === "string" ? value.replace(/["']/g, "").replace(/\s*,\s*/g, ",").toLowerCase() : "";
}

/** Builds the toolbar into `host` and keeps its state in sync with the editor via `update()`. */
export function buildToolbar(
  host: HTMLElement,
  root: HTMLElement,
  editor: Editor,
  config: ResolvedConfig,
  ui: EditorUi,
): Toolbar {
  const updaters: (() => void)[] = [];
  const cleanups: (() => void)[] = [];
  const disabled = () => !editor.isEditable || ui.sourceMode();
  let closePopover: (() => void) | undefined;

  /** Shows `content` below `anchor`; closes on a click outside, Escape or the next popover. */
  function popover(anchor: HTMLElement, content: HTMLElement, className = "") {
    closePopover?.();
    const box = el("div", `mrte-popover ${className}`.trim());
    box.append(content);
    root.append(box);
    const rootBox = root.getBoundingClientRect();
    const anchorBox = anchor.getBoundingClientRect();
    box.style.top = `${anchorBox.bottom - rootBox.top + 4}px`;
    box.style.left = `${Math.max(4, Math.min(anchorBox.left - rootBox.left, rootBox.width - box.offsetWidth - 4))}px`;

    const onOutside = (event: MouseEvent) => {
      if (!box.contains(event.target as Node) && !anchor.contains(event.target as Node)) close();
    };
    const onKeydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
        editor.commands.focus();
      }
    };
    const close = () => {
      box.remove();
      anchor.setAttribute("aria-expanded", "false");
      document.removeEventListener("mousedown", onOutside, true);
      box.removeEventListener("keydown", onKeydown);
      if (closePopover === close) closePopover = undefined;
    };
    document.addEventListener("mousedown", onOutside, true);
    box.addEventListener("keydown", onKeydown);
    closePopover = close;
    return close;
  }

  /** A toolbar button that opens a popover built on demand. */
  function popoverButton(title: string, icon: IconNode, build: (close: () => void) => HTMLElement, className?: string) {
    const button = iconButton(title, icon);
    button.addEventListener("click", () => {
      if (button.getAttribute("aria-expanded") === "true") {
        closePopover?.();
        return;
      }
      let close = () => {};
      const content = build(() => close());
      close = popover(button, content, className);
      button.setAttribute("aria-expanded", "true");
      content.querySelector<HTMLElement>("input, button")?.focus();
    });
    return button;
  }

  function linkPopover(close: () => void): HTMLElement {
    const form = el("div", "mrte-row");
    const input = el("input", "mrte-input");
    input.type = "url";
    input.placeholder = "https://…";
    input.setAttribute("aria-label", "Link-Adresse");
    input.value = editor.getAttributes("link").href ?? "";
    const apply = textButton("Übernehmen", "mrte-text-btn mrte-primary");
    const remove = textButton("Entfernen");
    remove.hidden = !editor.isActive("link");
    form.append(input, apply, remove);
    queueMicrotask(() => input.select());

    const setLink = (href: string) => {
      const chain = editor.chain().focus().extendMarkRange("link");
      if (!href) chain.unsetLink().run();
      else if (editor.state.selection.empty && !editor.isActive("link"))
        chain.insertContent({ type: "text", text: href, marks: [{ type: "link", attrs: { href } }] }).run();
      else chain.setLink({ href }).run();
      close();
    };
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        setLink(input.value.trim());
      }
    });
    apply.addEventListener("click", () => setLink(input.value.trim()));
    remove.addEventListener("click", () => setLink(""));
    return form;
  }

  function imagePopover(close: () => void): HTMLElement {
    const form = el("div", "mrte-row");
    const input = el("input", "mrte-input");
    input.type = "url";
    input.placeholder = "Bildadresse https://…";
    input.setAttribute("aria-label", "Bildadresse");
    const apply = textButton("Einfügen", "mrte-text-btn mrte-primary");
    const file = textButton("Datei …");
    const picker = el("input", "");
    picker.type = "file";
    picker.accept = "image/*";
    picker.multiple = true;
    picker.hidden = true;
    form.append(input, apply, file, picker);

    const insert = () => {
      const src = input.value.trim();
      if (src) editor.chain().focus().setImage({ src }).run();
      close();
    };
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        insert();
      }
    });
    apply.addEventListener("click", insert);
    file.addEventListener("click", () => picker.click());
    picker.addEventListener("change", () => {
      ui.insertImageFiles([...(picker.files ?? [])]);
      close();
    });
    return form;
  }

  function tablePopover(close: () => void): HTMLElement {
    if (!editor.isActive("table")) {
      // Grid picker: hover chooses the size, click inserts.
      const wrap = el("div", "mrte-grid-picker");
      const label = el("div", "mrte-grid-label");
      label.textContent = "Tabelle einfügen";
      const grid = el("div", "mrte-grid");
      const cells: HTMLButtonElement[] = [];
      const mark = (rows: number, cols: number) => {
        cells.forEach((cell) => {
          const r = Number(cell.dataset.row);
          const c = Number(cell.dataset.col);
          cell.classList.toggle("mrte-grid-on", r < rows && c < cols);
        });
        label.textContent = `${cols} × ${rows}`;
      };
      for (let row = 0; row < 8; row++) {
        for (let col = 0; col < 10; col++) {
          const cell = el("button", "mrte-grid-cell");
          cell.type = "button";
          cell.dataset.row = String(row);
          cell.dataset.col = String(col);
          cell.setAttribute("aria-label", `${col + 1} Spalten, ${row + 1} Zeilen`);
          cell.addEventListener("mouseenter", () => mark(row + 1, col + 1));
          cell.addEventListener("focus", () => mark(row + 1, col + 1));
          cell.addEventListener("click", () => {
            editor.chain().focus().insertTable({ rows: row + 1, cols: col + 1, withHeaderRow: true }).run();
            close();
          });
          cells.push(cell);
        }
      }
      grid.append(...cells);
      wrap.append(label, grid);
      return wrap;
    }

    const menu = el("div", "mrte-menu");
    const items: ([string, (c: ChainedCommands) => ChainedCommands] | "|")[] = [
      ["Zeile darüber einfügen", (c) => c.addRowBefore()],
      ["Zeile darunter einfügen", (c) => c.addRowAfter()],
      ["Spalte links einfügen", (c) => c.addColumnBefore()],
      ["Spalte rechts einfügen", (c) => c.addColumnAfter()],
      "|",
      ["Zeile löschen", (c) => c.deleteRow()],
      ["Spalte löschen", (c) => c.deleteColumn()],
      "|",
      ["Zellen verbinden", (c) => c.mergeCells()],
      ["Zelle teilen", (c) => c.splitCell()],
      ["Kopfzeile ein/aus", (c) => c.toggleHeaderRow()],
      "|",
      ["Tabelle löschen", (c) => c.deleteTable()],
    ];
    for (const item of items) {
      if (item === "|") {
        menu.append(el("div", "mrte-menu-sep"));
        continue;
      }
      const [label, run] = item;
      const button = textButton(label, "mrte-menu-item");
      button.disabled = !run(editor.can().chain()).run();
      button.addEventListener("click", () => {
        run(editor.chain().focus()).run();
        close();
      });
      menu.append(button);
    }
    return menu;
  }

  function colorPopover(kind: "textColor" | "highlight", close: () => void): HTMLElement {
    const wrap = el("div", "mrte-colors");
    const reset = textButton(kind === "textColor" ? "Automatisch" : "Keine Hervorhebung", "mrte-menu-item");
    reset.addEventListener("click", () => {
      const chain = editor.chain().focus();
      (kind === "textColor" ? chain.unsetColor() : chain.unsetHighlight()).run();
      close();
    });
    const grid = el("div", "mrte-swatches");
    const apply = (color: string) => {
      const chain = editor.chain().focus();
      (kind === "textColor" ? chain.setColor(color) : chain.setHighlight({ color })).run();
      close();
    };
    for (const color of config.colors) {
      const swatch = el("button", "mrte-swatch");
      swatch.type = "button";
      swatch.title = color;
      swatch.setAttribute("aria-label", color);
      swatch.style.background = color;
      swatch.addEventListener("click", () => apply(color));
      grid.append(swatch);
    }
    const custom = el("label", "mrte-menu-item mrte-custom-color");
    const picker = el("input", "");
    picker.type = "color";
    picker.addEventListener("change", () => apply(picker.value));
    custom.append(picker, document.createTextNode("Weitere Farbe …"));
    wrap.append(reset, grid, custom);
    return wrap;
  }

  function specialCharsPopover(): HTMLElement {
    const grid = el("div", "mrte-chars");
    for (const char of SPECIAL_CHARS) {
      const button = textButton(char, "mrte-char");
      button.title = `${char} (U+${char.codePointAt(0)!.toString(16).toUpperCase().padStart(4, "0")})`;
      // Stays open so several characters can be inserted in a row.
      button.addEventListener("click", () => editor.chain().focus().insertContent(char).run());
      grid.append(button);
    }
    const nbsp = textButton("geschütztes Leerzeichen", "mrte-menu-item mrte-chars-wide");
    nbsp.addEventListener("click", () => editor.chain().focus().insertContent(" ").run());
    grid.append(nbsp);
    return grid;
  }

  for (const item of config.toolbar) {
    if (item === "|") {
      host.append(el("span", "mrte-sep"));
      continue;
    }

    if (item === "paragraphFormat") {
      const formats = config.paragraphFormats;
      if (formats.length === 0) continue;
      const element = select("Absatzformat");
      const unknown = new Option("—", "");
      unknown.hidden = true;
      element.append(unknown, ...formats.map((format, index) => new Option(format.label, String(index))));
      element.addEventListener("change", () => {
        const format = formats[Number(element.value)];
        if (!format) return;
        const attrs = { class: format.class ?? null, textAlign: alignment(editor) === "left" ? null : alignment(editor) };
        const tag = format.tag ?? "p";
        const chain = editor.chain().focus();
        (tag === "p" ? chain.setNode("paragraph", attrs) : chain.setNode("heading", { ...attrs, level: Number(tag[1]) })).run();
      });
      updaters.push(() => {
        const block = editor.state.selection.$from.parent;
        const tag =
          block.type.name === "heading" ? `h${block.attrs.level}` : block.type.name === "paragraph" ? "p" : "";
        const index = findParagraphFormat(formats, tag, block.attrs.class);
        element.value = index >= 0 ? String(index) : "";
        element.disabled = disabled() || !tag;
      });
      host.append(element);
      continue;
    }

    if (item === "characterFormat") {
      const formats = config.characterFormats;
      if (formats.length === 0) continue;
      const element = select("Zeichenformat");
      element.append(
        new Option("Kein Zeichenformat", ""),
        ...formats.map((format, index) => new Option(format.label, String(index))),
      );
      element.addEventListener("change", () => {
        const format = formats[Number(element.value)];
        const chain = editor.chain().focus();
        (element.value === "" || !format ? chain.unsetCharacterFormat() : chain.setCharacterFormat(format.class)).run();
      });
      updaters.push(() => {
        const cls = editor.getAttributes("characterFormat").class;
        const index = formats.findIndex((format) => format.class === cls);
        element.value = index >= 0 ? String(index) : "";
        element.disabled = disabled();
      });
      host.append(element);
      continue;
    }

    if (item === "fontFamily" || item === "fontSize") {
      const isFamily = item === "fontFamily";
      const values = isFamily ? config.fontFamilies.map((font) => font.value) : config.fontSizes;
      const labels = isFamily ? config.fontFamilies.map((font) => font.label) : config.fontSizes.map((s) => s.replace(/pt$/, ""));
      if (values.length === 0) continue;
      const element = select(isFamily ? "Schriftart" : "Schriftgröße (pt)", isFamily ? "mrte-select" : "mrte-select mrte-select-narrow");
      const other = new Option("—", "-");
      other.hidden = true;
      element.append(
        new Option(isFamily ? "Standardschrift" : "Größe", ""),
        other,
        ...labels.map((label, index) => new Option(label, String(index))),
      );
      if (isFamily) [...element.options].slice(2).forEach((option, index) => (option.style.fontFamily = values[index]));
      element.addEventListener("change", () => {
        const value = values[Number(element.value)];
        const chain = editor.chain().focus();
        if (isFamily) (element.value === "" || !value ? chain.unsetFontFamily() : chain.setFontFamily(value)).run();
        else (element.value === "" || !value ? chain.unsetFontSize() : chain.setFontSize(value)).run();
      });
      updaters.push(() => {
        const current = editor.getAttributes("textStyle")[isFamily ? "fontFamily" : "fontSize"];
        const index = isFamily
          ? values.findIndex((value) => fontKey(value) === fontKey(current))
          : values.indexOf(current);
        element.value = index >= 0 ? String(index) : current ? "-" : "";
        other.textContent = isFamily ? String(current ?? "").split(",")[0].replace(/["']/g, "") : String(current ?? "").replace(/pt$/, "");
        element.disabled = disabled();
      });
      host.append(element);
      continue;
    }

    if (item === "textColor" || item === "highlight") {
      const isText = item === "textColor";
      const button = popoverButton(
        isText ? "Schriftfarbe" : "Hervorhebung",
        isText ? Baseline : Highlighter,
        (close) => colorPopover(item, close),
      );
      const bar = el("span", "mrte-color-bar");
      button.append(bar);
      updaters.push(() => {
        const color = isText ? editor.getAttributes("textStyle").color : editor.getAttributes("highlight").color;
        bar.style.background = color || (isText ? "currentColor" : "transparent");
        button.disabled = disabled();
      });
      host.append(button);
      continue;
    }

    if (item === "link") {
      const button = popoverButton("Link (Strg+K)", Link, linkPopover);
      const onKeydown = (event: KeyboardEvent) => {
        if (event.ctrlKey && event.key.toLowerCase() === "k" && !disabled()) {
          event.preventDefault();
          button.click();
        }
      };
      editor.view.dom.addEventListener("keydown", onKeydown);
      cleanups.push(() => editor.view.dom.removeEventListener("keydown", onKeydown));
      updaters.push(() => {
        button.setAttribute("aria-pressed", String(editor.isActive("link")));
        button.disabled = disabled();
      });
      host.append(button);
      continue;
    }

    if (item === "image" || item === "table" || item === "specialChars") {
      const button =
        item === "image"
          ? popoverButton("Bild einfügen", Image, imagePopover)
          : item === "table"
            ? popoverButton("Tabelle", Table, tablePopover)
            : popoverButton("Sonderzeichen", Omega, specialCharsPopover);
      updaters.push(() => {
        if (item === "table") button.setAttribute("aria-pressed", String(editor.isActive("table")));
        button.disabled = disabled();
      });
      host.append(button);
      continue;
    }

    if (item === "source" || item === "search" || item === "fullscreen" || item === "print") {
      const defs = {
        source: { title: "HTML-Quelltext", icon: CodeXml, run: ui.toggleSource, active: ui.sourceMode },
        search: { title: "Suchen und Ersetzen (Strg+F)", icon: Search, run: ui.toggleSearch, active: ui.searchOpen },
        fullscreen: { title: "Vollbild", icon: Maximize2, run: ui.toggleFullscreen, active: ui.fullscreen },
        print: { title: "Drucken (Strg+P)", icon: Printer, run: ui.print, active: undefined },
      };
      const def = defs[item];
      const button = iconButton(def.title, def.icon);
      button.addEventListener("click", () => def.run());
      updaters.push(() => {
        const active = def.active?.() ?? false;
        if (def.active) button.setAttribute("aria-pressed", String(active));
        if (item === "fullscreen") {
          button.replaceChildren(createElement(active ? Minimize2 : Maximize2, { "aria-hidden": "true" }));
          button.title = active ? "Vollbild beenden" : "Vollbild";
        }
        if (item === "search") button.disabled = ui.sourceMode();
      });
      host.append(button);
      continue;
    }

    const def = BUTTONS[item];
    if (!def) continue;
    const button = iconButton(def.title, def.icon);
    button.addEventListener("click", () => def.run(editor.chain().focus()).run());
    updaters.push(() => {
      if (def.active) button.setAttribute("aria-pressed", String(def.active(editor)));
      button.disabled = disabled() || !def.run(editor.can().chain()).run();
    });
    host.append(button);
  }

  const update = () => updaters.forEach((fn) => fn());
  update();
  return {
    update,
    destroy: () => {
      closePopover?.();
      cleanups.forEach((fn) => fn());
      host.replaceChildren();
    },
  };
}
