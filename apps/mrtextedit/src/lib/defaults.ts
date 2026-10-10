import { TOOLBAR_ITEMS, type EditorConfig } from "@mrtools/textedit";

/** Configuration until the user saves their own (toolbar and formats dialog). */
export const DEFAULT_CONFIG: EditorConfig = {
  toolbar: [
    "undo", "redo", "|",
    "paragraphFormat", "characterFormat", "fontFamily", "fontSize", "|",
    "bold", "italic", "underline", "strike", "subscript", "superscript", "textColor", "highlight", "|",
    "alignLeft", "alignCenter", "alignRight", "alignJustify", "|",
    "bulletList", "orderedList", "taskList", "outdent", "indent", "|",
    "link", "image", "table", "blockquote", "codeBlock", "horizontalRule", "specialChars", "|",
    "clearFormat", "search", "source",
  ],
  paragraphFormats: [
    { label: "Standard" },
    { label: "Überschrift 1", tag: "h1" },
    { label: "Überschrift 2", tag: "h2" },
    { label: "Überschrift 3", tag: "h3" },
    { label: "Zitat", class: "zitat", css: "font-style: italic; margin-left: 2em; opacity: .8" },
    { label: "Hinweis", class: "hinweis", css: "padding: .5em .75em; border-left: 4px solid #f59e0b; background: rgb(245 158 11 / .12)" },
    { label: "Kleingedrucktes", class: "klein", css: "font-size: .85em; opacity: .75" },
  ],
  characterFormats: [
    { label: "Markiert", class: "marker", css: "background: rgb(250 204 21 / .45)" },
    { label: "Produktname", class: "produkt", css: "font-variant: small-caps; font-weight: 600" },
    { label: "Tastenkürzel", class: "taste", css: "padding: 0 .3em; border: 1px solid currentColor; border-radius: 4px; font-size: .85em" },
  ],
  placeholder: "Text eingeben …",
};

export const TOOLBAR_HELP = `Toolleiste: ${TOOLBAR_ITEMS.join(", ")} und "|" als Trenner.`;
