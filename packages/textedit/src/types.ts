/** Block tags a paragraph format can produce. */
export type BlockTag = "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6";

/**
 * A paragraph format, chosen in the „Absatzformat“ combobox. It is stored in the HTML as tag plus
 * class, e.g. `{ label: "Zitat", class: "zitat" }` → `<p class="zitat">`.
 */
export interface ParagraphFormat {
  label: string;
  /** Default `p`. */
  tag?: BlockTag;
  /** CSS class that marks the format in the HTML (letters, digits, `-`, `_`). */
  class?: string;
  /** CSS declarations for the editor display, e.g. `font-style: italic; margin-left: 2em`. */
  css?: string;
}

/**
 * A character format, chosen in the „Zeichenformat“ combobox. It is stored in the HTML as
 * `<span class="…">`.
 */
export interface CharacterFormat {
  label: string;
  class: string;
  css?: string;
}

export const TOOLBAR_ITEMS = [
  "undo",
  "redo",
  "paragraphFormat",
  "characterFormat",
  "fontFamily",
  "fontSize",
  "bold",
  "italic",
  "underline",
  "strike",
  "subscript",
  "superscript",
  "code",
  "textColor",
  "highlight",
  "alignLeft",
  "alignCenter",
  "alignRight",
  "alignJustify",
  "bulletList",
  "orderedList",
  "taskList",
  "indent",
  "outdent",
  "blockquote",
  "codeBlock",
  "horizontalRule",
  "link",
  "image",
  "table",
  "specialChars",
  "clearFormat",
  "search",
  "print",
  "fullscreen",
  "source",
] as const;

/** A toolbar entry; `"|"` is a separator. */
export type ToolbarItem = (typeof TOOLBAR_ITEMS)[number] | "|";

export interface EditorConfig {
  /** Toolbar entries in display order. Default: all. Empty list: no toolbar. */
  toolbar?: ToolbarItem[];
  /** Default: Standard and Überschrift 1–3. */
  paragraphFormats?: ParagraphFormat[];
  /** Default: none (the combobox is then hidden). */
  characterFormats?: CharacterFormat[];
  placeholder?: string;
  readOnly?: boolean;
  /** Default `auto` (follows the system). */
  theme?: "light" | "dark" | "auto";
  spellcheck?: boolean;
  /** Entries of the „Schriftart“ combobox: a CSS font family, or label plus value. */
  fontFamilies?: (string | { label: string; value: string })[];
  /** Entries of the „Schriftgröße“ combobox as CSS sizes, e.g. `"12pt"`. */
  fontSizes?: string[];
  /** Palette for text colour and highlight (CSS colours). */
  colors?: string[];
  /** Shows a status bar with words and characters below the text. Default `false`. */
  statusbar?: boolean;
}

export interface EditorOptions extends EditorConfig {
  /** Initial content as HTML. */
  content?: string;
  /** Called after every change made by the user (not after `setHTML`). */
  onChange?: () => void;
}
