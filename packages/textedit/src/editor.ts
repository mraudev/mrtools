import { Editor } from "@tiptap/core";
import type { Node as PmNode } from "@tiptap/pm/model";
import { EditorState } from "@tiptap/pm/state";
import StarterKit from "@tiptap/starter-kit";
import Highlight from "@tiptap/extension-highlight";
import Image from "@tiptap/extension-image";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import Subscript from "@tiptap/extension-subscript";
import Superscript from "@tiptap/extension-superscript";
import { TableKit } from "@tiptap/extension-table";
import TextAlign from "@tiptap/extension-text-align";
import { TextStyleKit } from "@tiptap/extension-text-style";
import { Placeholder } from "@tiptap/extensions";
import { BlockClass, CharacterFormatMark, formatsCss, resolveConfig, type ResolvedConfig } from "./formats";
import { Search } from "./search";
import { createSearchPanel, type SearchPanel } from "./search-panel";
import { buildToolbar, type Toolbar } from "./toolbar";
import type { EditorConfig, EditorOptions } from "./types";
import baseCss from "./textedit.css?inline";

let nextId = 0;

/** The base styles go into the document once, however many editors there are. */
function injectBaseStyles() {
  if (document.getElementById("mrte-styles")) return;
  const style = document.createElement("style");
  style.id = "mrte-styles";
  style.textContent = baseCss;
  document.head.append(style);
}

function readAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

const imageFiles = (files: FileList | null | undefined) => [...(files ?? [])].filter((f) => f.type.startsWith("image/"));

/**
 * A WYSIWYG editor with toolbar inside `host`. Content goes in and out as HTML
 * (`setHTML`/`getHTML`); toolbar and formats can be changed at any time with `setConfig`.
 */
export class TextEditor {
  /** The root element (`.mrte`) inside the host. */
  readonly element: HTMLElement;
  readonly tiptap: Editor;
  #config: ResolvedConfig;
  #toolbar?: Toolbar;
  readonly #toolbarHost: HTMLElement;
  readonly #style: HTMLStyleElement;
  readonly #body: HTMLElement;
  readonly #source: HTMLTextAreaElement;
  readonly #search: SearchPanel;
  readonly #statusbar: HTMLElement;
  readonly #listeners = new Set<() => void>();

  constructor(host: HTMLElement, options: EditorOptions = {}) {
    injectBaseStyles();
    this.#config = resolveConfig(options);

    this.element = document.createElement("div");
    this.element.className = "mrte";
    this.element.dataset.mrte = String(++nextId);
    this.#style = document.createElement("style");
    this.#toolbarHost = document.createElement("div");
    this.#toolbarHost.className = "mrte-toolbar";
    this.#toolbarHost.setAttribute("role", "toolbar");
    this.#body = document.createElement("div");
    this.#body.className = "mrte-body";
    this.#source = document.createElement("textarea");
    this.#source.className = "mrte-source";
    this.#source.hidden = true;
    this.#source.spellcheck = false;
    this.#source.setAttribute("aria-label", "HTML-Quelltext");
    // Every edit in the source goes straight into the document, so getHTML and change events stay current.
    this.#source.addEventListener("input", () => this.tiptap.commands.setContent(this.#source.value, { emitUpdate: true }));
    this.#statusbar = document.createElement("div");
    this.#statusbar.className = "mrte-statusbar";
    // Events of the toolbar's own inputs must not look like events of the editor to the host page.
    for (const type of ["change", "input"]) this.element.addEventListener(type, (event) => event.stopPropagation());
    this.element.addEventListener("keydown", (event) => this.#onKeydown(event));
    host.append(this.element);

    if (options.onChange) this.#listeners.add(options.onChange);
    this.tiptap = new Editor({
      element: this.#body,
      extensions: [
        // No trailing node: it would add an empty paragraph to the HTML after a final heading.
        StarterKit.configure({ link: { openOnClick: false, autolink: true }, trailingNode: false }),
        Subscript,
        Superscript,
        TextAlign.configure({ types: ["heading", "paragraph"] }),
        TextStyleKit.configure({ backgroundColor: false, lineHeight: false }),
        Highlight.configure({ multicolor: true }),
        TableKit.configure({ table: { resizable: true } }),
        Image.configure({
          allowBase64: true,
          resize: {
            enabled: true,
            directions: ["top-left", "top-right", "bottom-left", "bottom-right"],
            alwaysPreserveAspectRatio: true,
            minWidth: 24,
            minHeight: 24,
          },
        }),
        TaskList,
        TaskItem.configure({ nested: true }),
        Placeholder.configure({ placeholder: () => this.#config.placeholder }),
        BlockClass,
        CharacterFormatMark,
        Search,
      ],
      content: options.content ?? "",
      editable: !this.#config.readOnly,
      editorProps: {
        attributes: () => ({ class: "mrte-content", spellcheck: String(this.#config.spellcheck) }),
        transformPastedHTML: (html) => this.#cleanPastedHtml(html),
        handlePaste: (_view, event) => {
          // Only a pure image (screenshot); Word & Co. put a picture next to their HTML as well.
          const files = imageFiles(event.clipboardData?.files);
          if (files.length === 0 || event.clipboardData?.types.includes("text/html")) return false;
          this.insertImageFiles(files);
          return true;
        },
        handleDrop: (view, event) => {
          const files = imageFiles(event.dataTransfer?.files);
          if (files.length === 0) return false;
          const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos;
          this.insertImageFiles(files, pos);
          return true;
        },
      },
      onUpdate: () => {
        this.#updateStatus();
        this.#listeners.forEach((fn) => fn());
      },
      onTransaction: () => {
        this.#toolbar?.update();
        // Also runs while the editor is being built, before the panel exists.
        this.#search?.update();
      },
    });
    this.#search = createSearchPanel(this.tiptap, () => this.toggleSearch());
    this.element.append(this.#style, this.#toolbarHost, this.#search.element, this.#body, this.#source, this.#statusbar);
    // A click below the last paragraph should still put the cursor into the text.
    this.#body.addEventListener("mousedown", (event) => {
      if (event.target === this.#body) {
        event.preventDefault();
        this.tiptap.commands.focus("end");
      }
    });
    this.#applyConfig();
    this.#updateStatus();
  }

  /** The content as HTML; an empty document gives `""`. */
  getHTML(): string {
    return this.isEmpty ? "" : this.tiptap.getHTML();
  }

  /** Replaces the content (without change event) and clears the undo history. */
  setHTML(html: string) {
    this.tiptap.commands.setContent(html || "", { emitUpdate: false });
    const state = this.tiptap.state;
    this.tiptap.view.updateState(EditorState.create({ doc: state.doc, plugins: state.plugins }));
    if (this.sourceMode) this.#source.value = formatSource(this.getHTML());
    this.#updateStatus();
    this.#toolbar?.update();
  }

  /** Inserts HTML at the cursor (replacing the selection), e.g. a text block from the host. Counts as a change. */
  insertHTML(html: string) {
    this.tiptap.chain().focus().insertContent(html).run();
  }

  /** The content as plain text: one line per paragraph, table rows as one line with tab-separated cells. */
  getText(): string {
    return plainText(this.tiptap.state.doc);
  }

  /** Only a single empty paragraph – Tiptap's own `isEmpty` would also count an empty table or a lone image. */
  get isEmpty(): boolean {
    const doc = this.tiptap.state.doc;
    const first = doc.firstChild;
    return doc.childCount === 1 && first?.type.name === "paragraph" && first.content.size === 0;
  }

  get config(): ResolvedConfig {
    return structuredClone(this.#config);
  }

  /** Replaces the whole configuration (toolbar, formats, …); the content stays. */
  setConfig(config: EditorConfig) {
    this.#config = resolveConfig(config);
    this.#applyConfig();
  }

  setReadOnly(readOnly: boolean) {
    this.setConfig({ ...this.#config, readOnly });
  }

  /** Calls `listener` after every change by the user; returns a function that unsubscribes. */
  onChange(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  focus() {
    if (this.sourceMode) this.#source.focus();
    else this.tiptap.commands.focus();
  }

  /** True while the HTML source is shown instead of the formatted text. */
  get sourceMode(): boolean {
    return !this.#source.hidden;
  }

  /** Switches between formatted text and HTML source (toolbar item `source`). */
  toggleSource(show = !this.sourceMode) {
    if (show === this.sourceMode) return;
    if (show) {
      if (this.#search.isOpen()) this.#search.close();
      this.#source.value = formatSource(this.getHTML());
    }
    this.#source.hidden = !show;
    this.#body.hidden = show;
    if (show) this.#source.focus();
    else this.tiptap.commands.focus();
    this.#toolbar?.update();
  }

  /** Opens the search bar (with the replace field focused if `replace`), or closes it. */
  toggleSearch(show = !this.#search.isOpen(), replace = false) {
    if (show && this.sourceMode) return;
    if (show) this.#search.open(replace);
    else if (this.#search.isOpen()) this.#search.close();
    this.#toolbar?.update();
  }

  get fullscreen(): boolean {
    return this.element.classList.contains("mrte-fullscreen");
  }

  toggleFullscreen(on = !this.fullscreen) {
    this.element.classList.toggle("mrte-fullscreen", on);
    this.#toolbar?.update();
    this.focus();
  }

  /** Prints only the text (with the format CSS) through a hidden frame. */
  print() {
    const frame = document.createElement("iframe");
    frame.style.cssText = "position:fixed;width:0;height:0;border:0;visibility:hidden";
    document.body.append(frame);
    const doc = frame.contentDocument!;
    doc.open();
    doc.write(`<!doctype html><html><head><meta charset="utf-8"><title></title><style>
${baseCss}
.mrte { height: auto; display: block; background: #fff; color: #000; }
.mrte-content { padding: 0; min-height: 0; }
${formatsCss(this.#config, ".mrte-content")}
</style></head><body><div class="mrte" data-theme="light"><div class="mrte-content"></div></div></body></html>`);
    doc.close();
    doc.title = document.title;
    doc.querySelector(".mrte-content")!.innerHTML = this.getHTML();
    const done = () => frame.remove();
    frame.contentWindow!.addEventListener("afterprint", done);
    // Images must be loaded, otherwise they are missing on paper.
    Promise.all(
      [...doc.images].map((img) => (img.complete ? null : new Promise((r) => (img.onload = img.onerror = r)))),
    ).then(() => {
      frame.contentWindow!.focus();
      frame.contentWindow!.print();
      setTimeout(done, 60_000);
    });
  }

  /** Inserts image files as data URLs at `pos` (default: the cursor). */
  async insertImageFiles(files: File[], pos?: number) {
    if (!this.tiptap.isEditable) return;
    for (const file of files.filter((f) => f.type.startsWith("image/"))) {
      const src = await readAsDataUrl(file);
      const content = { type: "image", attrs: { src, alt: file.name.replace(/\.[^.]+$/, "") } };
      if (pos === undefined) this.tiptap.chain().focus().insertContent(content).run();
      else this.tiptap.chain().focus().insertContentAt(pos, content).run();
      pos = undefined;
    }
  }

  destroy() {
    this.#toolbar?.destroy();
    this.tiptap.destroy();
    this.element.remove();
    this.#listeners.clear();
  }

  #onKeydown(event: KeyboardEvent) {
    const toolbar = this.#config.toolbar;
    const key = event.key.toLowerCase();
    if (event.ctrlKey && !event.altKey && (key === "f" || key === "h") && toolbar.includes("search")) {
      event.preventDefault();
      this.toggleSearch(true, key === "h");
    } else if (event.ctrlKey && key === "p" && toolbar.includes("print")) {
      event.preventDefault();
      this.print();
    } else if (event.key === "Escape" && !event.defaultPrevented && this.fullscreen) {
      this.toggleFullscreen(false);
    }
  }

  /** Pasted HTML keeps only the classes of configured formats (no `MsoNormal` & co. from Word). */
  #cleanPastedHtml(html: string): string {
    const known = new Set([
      ...this.#config.paragraphFormats.map((format) => format.class).filter(Boolean),
      ...this.#config.characterFormats.map((format) => format.class),
    ]);
    const doc = new DOMParser().parseFromString(html, "text/html");
    for (const element of doc.body.querySelectorAll("[class]")) {
      const kept = [...element.classList].filter((cls) => known.has(cls));
      if (kept.length) element.setAttribute("class", kept.join(" "));
      else element.removeAttribute("class");
    }
    return doc.body.innerHTML;
  }

  #updateStatus() {
    if (this.#statusbar.hidden) return;
    const text = this.getText();
    const words = text.split(/\s+/).filter(Boolean).length;
    this.#statusbar.textContent = `${words} ${words === 1 ? "Wort" : "Wörter"} · ${text.replace(/\n/g, "").length} Zeichen`;
  }

  #applyConfig() {
    const config = this.#config;
    this.element.dataset.theme = config.theme;
    this.#style.textContent = formatsCss(config, `.mrte[data-mrte="${this.element.dataset.mrte}"] .mrte-content`);
    this.tiptap.setEditable(!config.readOnly, false);
    this.#source.readOnly = config.readOnly;
    this.#statusbar.hidden = !config.statusbar;
    // Without the button there would be no way back from these views.
    if (!config.toolbar.includes("source")) this.toggleSource(false);
    if (!config.toolbar.includes("fullscreen") && this.fullscreen) this.toggleFullscreen(false);
    this.#toolbar?.destroy();
    this.#toolbarHost.hidden = config.toolbar.length === 0;
    this.#toolbar = buildToolbar(this.#toolbarHost, this.element, this.tiptap, config, {
      sourceMode: () => this.sourceMode,
      toggleSource: () => this.toggleSource(),
      searchOpen: () => this.#search.isOpen(),
      toggleSearch: () => this.toggleSearch(),
      fullscreen: () => this.fullscreen,
      toggleFullscreen: () => this.toggleFullscreen(),
      print: () => this.print(),
      insertImageFiles: (files) => this.insertImageFiles(files),
    });
    this.#updateStatus();
    this.#search.update();
    // Re-evaluates the editor attributes (spellcheck) and the placeholder.
    this.tiptap.view.dispatch(this.tiptap.state.tr);
  }
}

const leafText = (leaf: PmNode) => (leaf.type.name === "hardBreak" ? "\n" : "");

export function plainText(doc: PmNode): string {
  const lines: string[] = [];
  doc.descendants((node) => {
    if (node.type.name === "tableRow") {
      const cells: string[] = [];
      node.forEach((cell) => cells.push(cell.textBetween(0, cell.content.size, " ", leafText)));
      lines.push(cells.join("\t"));
      return false;
    }
    if (node.isTextblock) {
      lines.push(node.textBetween(0, node.content.size, "\n", leafText));
      return false;
    }
    return true;
  });
  return lines.join("\n");
}

/** One block per line, list items indented – easier to read than the single line from getHTML. */
export function formatSource(html: string): string {
  return html
    .replace(/(<\/(?:p|h[1-6]|blockquote|pre|table)>|<hr>|<img [^>]*>)(?!<\/li>)/g, "$1\n")
    .replace(/(<\/(?:li|tr)>)/g, "$1\n")
    .replace(/(<(?:ul|ol|blockquote|table|tbody)(?: [^>]*)?>)/g, "$1\n")
    .replace(/(<\/(?:ul|ol|tbody)>)/g, "$1\n")
    .replace(/^<(li|tr)/gm, "  <$1")
    .trimEnd();
}
