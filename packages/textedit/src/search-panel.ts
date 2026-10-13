import type { Editor } from "@tiptap/core";
import { CaseSensitive, ChevronDown, ChevronUp, createElement, type IconNode, X } from "lucide";
import { gotoMatch, replaceAll, replaceCurrent, searchState, setSearch } from "./search";

export interface SearchPanel {
  element: HTMLElement;
  open(replace: boolean): void;
  close(): void;
  isOpen(): boolean;
  /** Refreshes the match count (after edits). */
  update(): void;
}

function button(title: string, icon: IconNode): HTMLButtonElement {
  const element = document.createElement("button");
  element.type = "button";
  element.className = "mrte-btn";
  element.title = title;
  element.setAttribute("aria-label", title);
  element.append(createElement(icon, { "aria-hidden": "true" }));
  return element;
}

function textButton(label: string): HTMLButtonElement {
  const element = document.createElement("button");
  element.type = "button";
  element.className = "mrte-text-btn";
  element.textContent = label;
  return element;
}

function input(placeholder: string): HTMLInputElement {
  const element = document.createElement("input");
  element.className = "mrte-input";
  element.placeholder = placeholder;
  element.setAttribute("aria-label", placeholder);
  element.spellcheck = false;
  return element;
}

/** The bar below the toolbar: search (Enter/Umschalt+Enter), match count, replace one or all. */
export function createSearchPanel(editor: Editor, onToggle: () => void): SearchPanel {
  const element = document.createElement("div");
  element.className = "mrte-search";
  element.hidden = true;

  const query = input("Suchen");
  const caseButton = button("Groß-/Kleinschreibung beachten", CaseSensitive);
  caseButton.setAttribute("aria-pressed", "false");
  const count = document.createElement("span");
  count.className = "mrte-search-count";
  const prev = button("Vorheriger Treffer (Umschalt+Enter)", ChevronUp);
  const next = button("Nächster Treffer (Enter)", ChevronDown);
  const replacement = input("Ersetzen durch");
  const replaceOne = textButton("Ersetzen");
  const replaceEvery = textButton("Alle ersetzen");
  const replaceRow = document.createElement("span");
  replaceRow.className = "mrte-row";
  replaceRow.append(replacement, replaceOne, replaceEvery);
  const close = button("Schließen (Esc)", X);
  element.append(query, caseButton, count, prev, next, replaceRow, close);

  const caseSensitive = () => caseButton.getAttribute("aria-pressed") === "true";
  const search = () => setSearch(editor, query.value, caseSensitive());

  const update = () => {
    const { matches, current } = searchState(editor);
    count.textContent = query.value ? (matches.length ? `${current + 1} von ${matches.length}` : "Keine Treffer") : "";
    replaceRow.hidden = !editor.isEditable;
    for (const control of [prev, next, replaceOne, replaceEvery]) control.disabled = matches.length === 0;
  };

  query.addEventListener("input", () => {
    search();
    update();
  });
  caseButton.addEventListener("click", () => {
    caseButton.setAttribute("aria-pressed", String(!caseSensitive()));
    search();
    update();
  });
  prev.addEventListener("click", () => gotoMatch(editor, -1));
  next.addEventListener("click", () => gotoMatch(editor, 1));
  replaceOne.addEventListener("click", () => replaceCurrent(editor, replacement.value));
  replaceEvery.addEventListener("click", () => {
    const replaced = replaceAll(editor, replacement.value);
    count.textContent = `${replaced} ersetzt`;
  });
  close.addEventListener("click", onToggle);

  element.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onToggle();
    } else if (event.key === "Enter" && event.target === query) {
      event.preventDefault();
      gotoMatch(editor, event.shiftKey ? -1 : 1);
    } else if (event.key === "Enter" && event.target === replacement) {
      event.preventDefault();
      replaceCurrent(editor, replacement.value);
    }
  });

  return {
    element,
    open(replace) {
      element.hidden = false;
      const { from, to, empty } = editor.state.selection;
      const selected = empty ? "" : editor.state.doc.textBetween(from, to, " ");
      if (selected && !selected.includes("\n") && selected.length < 200) query.value = selected;
      search();
      update();
      const target = replace && editor.isEditable ? replacement : query;
      target.focus();
      target.select();
    },
    close() {
      element.hidden = true;
      setSearch(editor, "", false);
      editor.commands.focus();
    },
    isOpen: () => !element.hidden,
    update,
  };
}
