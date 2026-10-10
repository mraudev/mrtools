import { Extension, type Editor } from "@tiptap/core";
import type { Node as PmNode } from "@tiptap/pm/model";
import { Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

export interface Match {
  from: number;
  to: number;
}

interface SearchState {
  query: string;
  caseSensitive: boolean;
  matches: Match[];
  /** Index of the current match, -1 if none. */
  current: number;
  decorations: DecorationSet;
}

interface SearchMeta {
  query?: string;
  caseSensitive?: boolean;
  current?: number;
}

const key = new PluginKey<SearchState>("mrteSearch");

/** All matches in the document. Searches within each text block; inline nodes (images, breaks) never match. */
export function findMatches(doc: PmNode, query: string, caseSensitive: boolean): Match[] {
  const matches: Match[] = [];
  if (!query) return matches;
  const needle = caseSensitive ? query : query.toLowerCase();
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    // One character per position: text as is, every inline leaf node as a placeholder.
    let text = "";
    node.forEach((child) => (text += child.isText ? child.text : "￼"));
    if (!caseSensitive) text = text.toLowerCase();
    for (let index = text.indexOf(needle); index >= 0; index = text.indexOf(needle, index + needle.length)) {
      matches.push({ from: pos + 1 + index, to: pos + 1 + index + needle.length });
    }
    return false;
  });
  return matches;
}

function build(doc: PmNode, query: string, caseSensitive: boolean, current: number): SearchState {
  const matches = findMatches(doc, query, caseSensitive);
  const index = matches.length === 0 ? -1 : Math.min(Math.max(current, 0), matches.length - 1);
  const decorations = DecorationSet.create(
    doc,
    matches.map((match, i) =>
      Decoration.inline(match.from, match.to, { class: i === index ? "mrte-match mrte-match-current" : "mrte-match" }),
    ),
  );
  return { query, caseSensitive, matches, current: index, decorations };
}

/** Highlights the matches of the search panel in the text. */
export const Search = Extension.create({
  name: "mrteSearch",
  addProseMirrorPlugins() {
    return [
      new Plugin<SearchState>({
        key,
        state: {
          init: (_, state) => build(state.doc, "", false, -1),
          apply: (tr, old) => {
            const meta = tr.getMeta(key) as SearchMeta | undefined;
            if (!meta && !tr.docChanged) return old;
            return build(
              tr.doc,
              meta?.query ?? old.query,
              meta?.caseSensitive ?? old.caseSensitive,
              meta?.current ?? old.current,
            );
          },
        },
        props: {
          decorations: (state) => key.getState(state)?.decorations,
        },
      }),
    ];
  },
});

export function searchState(editor: Editor): SearchState {
  return key.getState(editor.state)!;
}

function dispatch(editor: Editor, meta: SearchMeta, select = false) {
  const tr = editor.state.tr.setMeta(key, meta);
  editor.view.dispatch(tr);
  const state = searchState(editor);
  const match = state.matches[state.current];
  if (select && match) {
    editor.view.dispatch(
      editor.state.tr.setSelection(TextSelection.create(editor.state.doc, match.from, match.to)).scrollIntoView(),
    );
  }
}

/** Sets the search text; the current match is the first one at or after the cursor. */
export function setSearch(editor: Editor, query: string, caseSensitive: boolean) {
  const cursor = editor.state.selection.from;
  const matches = findMatches(editor.state.doc, query, caseSensitive);
  const next = matches.findIndex((match) => match.from >= cursor);
  dispatch(editor, { query, caseSensitive, current: next >= 0 ? next : 0 });
}

/** Goes to the next (`+1`) or previous (`-1`) match, wrapping around, and selects it. */
export function gotoMatch(editor: Editor, step: 1 | -1) {
  const { matches, current } = searchState(editor);
  if (matches.length === 0) return;
  dispatch(editor, { current: (current + step + matches.length) % matches.length }, true);
}

export function replaceCurrent(editor: Editor, replacement: string) {
  const { matches, current } = searchState(editor);
  const match = matches[current];
  if (!match) return;
  editor.view.dispatch(editor.state.tr.insertText(replacement, match.from, match.to));
  // The doc change recomputes the matches; `current` now points at the following one.
  dispatch(editor, {}, true);
}

/** Replaces every match; returns how many. */
export function replaceAll(editor: Editor, replacement: string): number {
  const { matches } = searchState(editor);
  if (matches.length === 0) return 0;
  const tr = editor.state.tr;
  for (const match of [...matches].reverse()) tr.insertText(replacement, match.from, match.to);
  editor.view.dispatch(tr);
  return matches.length;
}
