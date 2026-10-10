import { Extension, Mark } from "@tiptap/core";
import {
  TOOLBAR_ITEMS,
  type BlockTag,
  type EditorConfig,
  type ParagraphFormat,
  type ToolbarItem,
} from "./types";

export const DEFAULT_TOOLBAR: ToolbarItem[] = [
  "undo", "redo", "|",
  "paragraphFormat", "characterFormat", "fontFamily", "fontSize", "|",
  "bold", "italic", "underline", "strike", "textColor", "highlight", "|",
  "alignLeft", "alignCenter", "alignRight", "alignJustify", "|",
  "bulletList", "orderedList", "taskList", "outdent", "indent", "|",
  "link", "image", "table", "specialChars", "|",
  "clearFormat", "search", "print", "source",
];

export const DEFAULT_PARAGRAPH_FORMATS: ParagraphFormat[] = [
  { label: "Standard", tag: "p" },
  { label: "Überschrift 1", tag: "h1" },
  { label: "Überschrift 2", tag: "h2" },
  { label: "Überschrift 3", tag: "h3" },
];

export const DEFAULT_FONT_FAMILIES = [
  "Arial", "Calibri", "Cambria", "Courier New, monospace", "Georgia, serif", "Segoe UI",
  "Tahoma", "Times New Roman, serif", "Verdana",
];

export const DEFAULT_FONT_SIZES = ["8pt", "9pt", "10pt", "11pt", "12pt", "14pt", "16pt", "18pt", "20pt", "24pt", "28pt", "36pt", "48pt", "72pt"];

export const DEFAULT_COLORS = [
  "#000000", "#3f3f46", "#71717a", "#a1a1aa", "#d4d4d8", "#ffffff", "#dc2626", "#ea580c", "#ca8a04", "#16a34a",
  "#0d9488", "#2563eb", "#7c3aed", "#db2777", "#fecaca", "#fed7aa", "#fef08a", "#bbf7d0", "#bfdbfe", "#e9d5ff",
];

const BLOCK_TAGS: BlockTag[] = ["p", "h1", "h2", "h3", "h4", "h5", "h6"];
const CLASS_NAME = /^[A-Za-z_][\w-]*$/;

export interface FontFamily {
  label: string;
  value: string;
}

export type ResolvedConfig = Required<Omit<EditorConfig, "fontFamilies">> & { fontFamilies: FontFamily[] };

/** Fills in defaults and drops entries the editor cannot represent (bad tags/class names). */
export function resolveConfig(config: EditorConfig = {}): ResolvedConfig {
  const paragraphFormats = (config.paragraphFormats ?? DEFAULT_PARAGRAPH_FORMATS)
    .map((format) => ({ ...format, tag: format.tag ?? "p" }))
    .filter(
      (format) =>
        typeof format.label === "string" &&
        BLOCK_TAGS.includes(format.tag) &&
        (format.class === undefined || format.class === "" || CLASS_NAME.test(format.class)),
    )
    .map((format) => (format.class ? format : { ...format, class: undefined }));
  const characterFormats = (config.characterFormats ?? []).filter(
    (format) => typeof format.label === "string" && CLASS_NAME.test(format.class ?? ""),
  );
  const toolbar = (config.toolbar ?? DEFAULT_TOOLBAR).filter(
    (item) => item === "|" || (TOOLBAR_ITEMS as readonly string[]).includes(item),
  );
  return {
    toolbar,
    paragraphFormats,
    characterFormats,
    placeholder: config.placeholder ?? "",
    readOnly: config.readOnly ?? false,
    theme: config.theme ?? "auto",
    spellcheck: config.spellcheck ?? true,
    fontFamilies: (config.fontFamilies ?? DEFAULT_FONT_FAMILIES)
      .map((font) => (typeof font === "string" ? { label: font.split(",")[0].trim(), value: font } : font))
      .filter((font) => typeof font?.label === "string" && typeof font.value === "string"),
    fontSizes: (config.fontSizes ?? DEFAULT_FONT_SIZES).filter((size) => typeof size === "string"),
    colors: (config.colors ?? DEFAULT_COLORS).filter((color) => typeof color === "string"),
    statusbar: config.statusbar ?? false,
  };
}

/** Index of the paragraph format for a block with this tag and class, or -1. */
export function findParagraphFormat(formats: ParagraphFormat[], tag: string, cls: string | null | undefined): number {
  return formats.findIndex((format) => (format.tag ?? "p") === tag && (format.class ?? "") === (cls ?? ""));
}

/** Format CSS can only add declarations, never close the rule or the style element. */
function declarations(css: string | undefined): string {
  return (css ?? "").replace(/[{}<>]/g, "");
}

/**
 * CSS for the formats. `scope` is put in front of every selector (e.g. the editor's content
 * element); without a scope the rules work in a standalone HTML document.
 */
export function formatsCss(
  config: Pick<EditorConfig, "paragraphFormats" | "characterFormats">,
  scope = "",
): string {
  const { paragraphFormats, characterFormats } = resolveConfig(config);
  const prefix = scope ? `${scope} ` : "";
  const rules: string[] = [];
  for (const format of paragraphFormats) {
    if (!format.css) continue;
    const selector = format.class ? `${format.tag}.${format.class}` : `${format.tag}:not([class])`;
    rules.push(`${prefix}${selector} { ${declarations(format.css)} }`);
  }
  for (const format of characterFormats) {
    if (format.css) rules.push(`${prefix}span.${format.class} { ${declarations(format.css)} }`);
  }
  return rules.join("\n");
}

/** Keeps the `class` attribute of paragraphs and headings – it carries the paragraph format. */
export const BlockClass = Extension.create({
  name: "blockClass",
  addGlobalAttributes() {
    return [
      {
        types: ["paragraph", "heading"],
        attributes: {
          class: {
            default: null,
            parseHTML: (element) => element.getAttribute("class") || null,
            renderHTML: (attributes) => (attributes.class ? { class: attributes.class } : {}),
          },
        },
      },
    ];
  },
});

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    characterFormat: {
      setCharacterFormat: (cls: string) => ReturnType;
      unsetCharacterFormat: () => ReturnType;
    };
  }
}

/** A character format: `<span class="…">`. One per text range (they exclude each other). */
export const CharacterFormatMark = Mark.create({
  name: "characterFormat",
  addAttributes() {
    return {
      class: {
        default: null,
        parseHTML: (element) => element.getAttribute("class"),
        renderHTML: (attributes) => ({ class: attributes.class }),
      },
    };
  },
  parseHTML() {
    return [{ tag: "span[class]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["span", HTMLAttributes, 0];
  },
  addCommands() {
    return {
      setCharacterFormat:
        (cls) =>
        ({ commands }) =>
          commands.setMark(this.name, { class: cls }),
      unsetCharacterFormat:
        () =>
        ({ commands }) =>
          commands.unsetMark(this.name, { extendEmptyMarkRange: true }),
    };
  },
});
