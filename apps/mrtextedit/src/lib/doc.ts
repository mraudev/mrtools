import { reactive, shallowRef, watchEffect } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { formatsCss, type EditorConfig, type TextEditor } from "@mrtools/textedit";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "./api";
import { DEFAULT_CONFIG } from "./defaults";
import { fileName, fromFile, toDocument } from "./document";

export const doc = reactive({
  path: null as string | null,
  dirty: false,
  words: 0,
  chars: 0,
  /** Open while asking about unsaved changes. */
  unsaved: null as null | ((answer: "save" | "discard" | "cancel") => void),
  configOpen: false,
  /** Zoom of the text in percent (Strg +/−/0, Strg+Mausrad). */
  zoom: readStored("zoom", 100),
  /** Most recent first. */
  recent: readStored<string[]>("recent", []),
});

function readStored<T>(key: string, fallback: T): T {
  try {
    const value = localStorage.getItem(key);
    return value === null ? fallback : (JSON.parse(value) as T);
  } catch {
    return fallback;
  }
}

function store(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Not persisted – it still applies for this session.
  }
}

function remember(path: string) {
  doc.recent = [path, ...doc.recent.filter((p) => p.toLowerCase() !== path.toLowerCase())].slice(0, 10);
  store("recent", doc.recent);
}

export function forgetRecent() {
  doc.recent = [];
  store("recent", []);
}

const ZOOM_STEPS = [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300];

/** `+1`/`-1` steps through the zoom levels, `0` resets to 100 %. */
export function zoom(step: -1 | 0 | 1) {
  const index = ZOOM_STEPS.findIndex((z) => z >= doc.zoom);
  doc.zoom = step === 0 ? 100 : ZOOM_STEPS[Math.min(Math.max(index + step, 0), ZOOM_STEPS.length - 1)];
  store("zoom", doc.zoom);
}

export const config = shallowRef<EditorConfig>(DEFAULT_CONFIG);

let editor: TextEditor | undefined;

const FILTERS = [
  { name: "HTML-Dokument", extensions: ["html", "htm"] },
  { name: "Textdatei", extensions: ["txt"] },
];

export function attachEditor(instance: TextEditor) {
  editor = instance;
  editor.onChange(() => {
    doc.dirty = true;
    countWords();
  });
}

function countWords() {
  const text = editor?.getText() ?? "";
  doc.words = text.split(/\s+/).filter(Boolean).length;
  doc.chars = text.replace(/\n/g, "").length;
}

function load(html: string, path: string | null) {
  editor?.setHTML(html);
  doc.path = path;
  doc.dirty = false;
  countWords();
}

/** Asks about unsaved changes; `true` means the current text may be replaced. */
async function confirmDiscard(): Promise<boolean> {
  if (!doc.dirty) return true;
  const answer = await new Promise<"save" | "discard" | "cancel">((resolve) => (doc.unsaved = resolve));
  doc.unsaved = null;
  if (answer === "save") return saveDoc();
  return answer === "discard";
}

export async function newDoc() {
  if (await confirmDiscard()) load("", null);
  editor?.focus();
}

export async function openDoc(path?: string) {
  if (!(await confirmDiscard())) return;
  const chosen = path ?? (await open({ title: "Dokument öffnen", filters: FILTERS }));
  if (typeof chosen !== "string") return;
  try {
    load(fromFile(await api.readText(chosen), chosen), chosen);
    remember(chosen);
    editor?.focus();
  } catch (error) {
    // A recent file that no longer exists drops out of the list.
    doc.recent = doc.recent.filter((p) => p !== chosen);
    store("recent", doc.recent);
    toastError("Öffnen fehlgeschlagen", error);
  }
}

export async function saveDoc(): Promise<boolean> {
  if (!doc.path) return saveDocAs();
  return write(doc.path);
}

export async function saveDocAs(): Promise<boolean> {
  const path = await save({
    title: "Speichern unter",
    defaultPath: doc.path ? fileName(doc.path) : "Dokument.html",
    filters: doc.path && isText(doc.path) ? [FILTERS[1], FILTERS[0]] : FILTERS,
  });
  return path ? write(path) : false;
}

async function write(path: string): Promise<boolean> {
  if (!editor) return false;
  try {
    const title = fileName(path).replace(/\.html?$/i, "");
    // .txt keeps only the text; Windows line breaks so every editor shows the lines.
    const content = isText(path)
      ? editor.getText().replace(/\n/g, "\r\n")
      : toDocument(editor.getHTML(), title, formatsCss(config.value));
    await api.writeText(path, content);
    doc.path = path;
    doc.dirty = false;
    remember(path);
    toast("success", "Gespeichert", fileName(path));
    return true;
  } catch (error) {
    toastError("Speichern fehlgeschlagen", error);
    return false;
  }
}

const isText = (path: string) => /\.txt$/i.test(path);

export function printDoc() {
  editor?.print();
}

export async function applyConfig(next: EditorConfig, persist: boolean) {
  if (persist) await api.saveConfig(JSON.stringify(next, null, 2));
  config.value = next;
  // The app has its own status bar.
  editor?.setConfig({ ...next, statusbar: false });
}

/** Loads the saved configuration and the file from the command line; asks before closing with changes. */
export async function initDoc() {
  try {
    const saved = await api.loadConfig();
    if (saved) await applyConfig(JSON.parse(saved), false);
  } catch (error) {
    toastError("Einstellungen nicht lesbar – Standard wird verwendet", error);
  }
  const startup = await api.startupFile();
  if (startup) await openDoc(startup);

  const appWindow = getCurrentWindow();
  await appWindow.onCloseRequested(async (event) => {
    if (!doc.dirty) return;
    event.preventDefault();
    if (await confirmDiscard()) await appWindow.destroy();
  });
  watchEffect(() => {
    const name = doc.path ? fileName(doc.path) : "Unbenannt";
    appWindow.setTitle(`${doc.dirty ? "• " : ""}${name} – mrtextedit`);
  });
}
