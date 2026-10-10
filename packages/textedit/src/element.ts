import { TextEditor } from "./editor";
import type { EditorConfig } from "./types";

/**
 * `<mr-textedit>` – the editor as a custom element for any web page or framework.
 *
 * - content: `value` property (HTML), or HTML inside the element at start
 * - configuration: `config` property or `config` attribute (JSON)
 * - `readonly` attribute, `change` event, form field with `name` attribute
 */
export class MrTextEditElement extends HTMLElement {
  static formAssociated = true;
  static observedAttributes = ["config", "readonly"];

  #editor?: TextEditor;
  #config: EditorConfig = {};
  #value = "";
  readonly #internals = this.attachInternals();

  get editor(): TextEditor | undefined {
    return this.#editor;
  }

  get value(): string {
    return this.#editor ? this.#editor.getHTML() : this.#value || this.innerHTML.trim();
  }

  set value(html: string) {
    this.#value = html ?? "";
    this.#editor?.setHTML(this.#value);
    this.#internals.setFormValue(this.value);
  }

  get text(): string {
    return this.#editor?.getText() ?? "";
  }

  get config(): EditorConfig {
    return this.#config;
  }

  set config(config: EditorConfig) {
    this.#config = config ?? {};
    this.#editor?.setConfig(this.#effectiveConfig());
  }

  connectedCallback() {
    if (this.#editor) return;
    // Defined before the page is parsed: the children (initial content) are not there yet.
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", () => this.isConnected && this.connectedCallback(), { once: true });
      return;
    }
    if (!this.#value && this.innerHTML.trim()) this.#value = this.innerHTML;
    this.replaceChildren();
    if (!this.style.display) this.style.display = "block";
    this.#editor = new TextEditor(this, {
      ...this.#effectiveConfig(),
      content: this.#value,
      onChange: () => {
        this.#internals.setFormValue(this.value);
        this.dispatchEvent(new Event("change", { bubbles: true }));
      },
    });
    this.#internals.setFormValue(this.value);
  }

  disconnectedCallback() {
    // Removed for good, or only moved? Moving reconnects in the same task.
    queueMicrotask(() => {
      if (this.isConnected || !this.#editor) return;
      this.#value = this.#editor.getHTML();
      this.#editor.destroy();
      this.#editor = undefined;
    });
  }

  attributeChangedCallback(name: string) {
    if (name === "config") {
      try {
        this.config = JSON.parse(this.getAttribute("config") || "{}");
      } catch (error) {
        console.error("mr-textedit: config ist kein gültiges JSON", error);
      }
    } else {
      this.#editor?.setConfig(this.#effectiveConfig());
    }
  }

  /** Inserts HTML at the cursor (replacing the selection). */
  insertHTML(html: string) {
    this.#editor?.insertHTML(html);
  }

  focus() {
    this.#editor?.focus();
  }

  #effectiveConfig(): EditorConfig {
    return this.hasAttribute("readonly") ? { ...this.#config, readOnly: true } : this.#config;
  }
}

export function defineElement(name = "mr-textedit") {
  if (!customElements.get(name)) customElements.define(name, class extends MrTextEditElement {});
}
