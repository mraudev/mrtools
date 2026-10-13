import { TextEditor } from "./editor";
import { defineElement } from "./element";
import type { EditorOptions } from "./types";

export { TextEditor } from "./editor";
export { MrTextEditElement, defineElement } from "./element";
export {
  DEFAULT_COLORS,
  DEFAULT_FONT_FAMILIES,
  DEFAULT_FONT_SIZES,
  DEFAULT_PARAGRAPH_FORMATS,
  DEFAULT_TOOLBAR,
  formatsCss,
  resolveConfig,
} from "./formats";
export * from "./types";

/** Creates an editor inside `host`. */
export function create(host: HTMLElement, options?: EditorOptions): TextEditor {
  return new TextEditor(host, options);
}

if (typeof customElements !== "undefined") defineElement();
