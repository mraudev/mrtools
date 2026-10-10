# mrtextedit

WYSIWYG-Texteditor mit konfigurierbarer Toolleiste und eigenen Absatz- und Zeichenformaten, die in
Comboboxen ausgewählt werden. Derselbe Editor läuft als Desktop-App, in Web-Seiten und in
VCL-Anwendungen (C++Builder/Delphi über `TEdgeBrowser`). Text geht als **HTML** hinein und heraus.

| Teil                                              | Wofür                                                        |
| ------------------------------------------------- | ------------------------------------------------------------ |
| [packages/textedit](../../packages/textedit)      | Editor-Kern (TypeScript, [Tiptap](https://tiptap.dev)/ProseMirror), framework-frei |
| `packages/textedit/dist/mrtextedit.js`            | eine Datei für Web-Seiten (Styles eingebaut)                 |
| `packages/textedit/dist/mrtextedit.html`          | eine Datei für `TEdgeBrowser` / WebView2 (Skript eingebaut)  |
| `apps/mrtextedit`                                 | Desktop-App (Tauri): Öffnen, Speichern, Formate einstellen   |
| [examples/web](examples/web/index.html)           | Beispiel Web-Seite                                           |
| [examples/vcl](examples/vcl/MainForm.cpp)         | Beispiel VCL-Formular (C++Builder)                           |

`dist/` entsteht mit `npm run build -w packages/textedit` (oder `npm run build` im Root).

## Konfiguration

Ein JSON-Objekt, überall gleich (App, Web, VCL). Alle Felder sind optional.

```json
{
  "toolbar": ["undo", "redo", "|", "paragraphFormat", "characterFormat", "|", "bold", "italic", "link"],
  "paragraphFormats": [
    { "label": "Standard" },
    { "label": "Überschrift", "tag": "h2" },
    { "label": "Zitat", "class": "zitat", "css": "font-style: italic; margin-left: 2em" }
  ],
  "characterFormats": [
    { "label": "Markiert", "class": "marker", "css": "background: #fde68a" }
  ],
  "placeholder": "Text eingeben …",
  "readOnly": false,
  "theme": "auto",
  "spellcheck": true,
  "statusbar": false,
  "fontFamilies": ["Arial", "Georgia, serif", { "label": "Hausschrift", "value": "Corporate Sans, sans-serif" }],
  "fontSizes": ["10pt", "12pt", "14pt", "18pt"],
  "colors": ["#000000", "#dc2626", "#2563eb", "#fef08a"]
}
```

- **toolbar** – Reihenfolge der Elemente, `"|"` ist ein Trenner. Ohne Angabe alle wichtigen, `[]` blendet die
  Leiste aus. Elemente:

  | Gruppe     | Elemente |
  | ---------- | -------- |
  | Verlauf    | `undo`, `redo` |
  | Comboboxen | `paragraphFormat`, `characterFormat`, `fontFamily`, `fontSize` |
  | Zeichen    | `bold`, `italic`, `underline`, `strike`, `subscript`, `superscript`, `code`, `textColor`, `highlight` |
  | Absatz     | `alignLeft`, `alignCenter`, `alignRight`, `alignJustify`, `bulletList`, `orderedList`, `taskList` (Checkliste), `indent`, `outdent` (Listeneinzug), `blockquote`, `codeBlock`, `horizontalRule` |
  | Einfügen   | `link` (<kbd>Strg</kbd>+<kbd>K</kbd>), `image` (Adresse oder Datei; auch Einfügen aus der Zwischenablage und Ziehen), `table` (Rasterauswahl; in einer Tabelle: Zeilen/Spalten einfügen und löschen, Zellen verbinden/teilen, Kopfzeile), `specialChars` |
  | Werkzeuge  | `clearFormat`, `search` (Suchen und Ersetzen, <kbd>Strg</kbd>+<kbd>F</kbd>/<kbd>H</kbd>), `print` (<kbd>Strg</kbd>+<kbd>P</kbd>, druckt nur den Text), `fullscreen`, `source` (HTML-Quelltext anzeigen und bearbeiten – Änderungen gelten sofort, mit `change`) |
- **paragraphFormats** – Einträge der Combobox „Absatzformat“. `tag` ist `p` (Standard) oder `h1`–`h6`,
  `class` kennzeichnet das Format im HTML: `{ "label": "Zitat", "class": "zitat" }` → `<p class="zitat">`.
  Ohne Angabe: Standard, Überschrift 1–3
- **characterFormats** – Einträge der Combobox „Zeichenformat“, im HTML `<span class="marker">`. Ohne Angabe
  keine (die Combobox fehlt dann)
- **css** – nur für die Anzeige im Editor. Das gespeicherte HTML enthält nur die Klassen; wer es woanders
  anzeigt, braucht dieselben Regeln (`formatsCss(config)` im Paket erzeugt sie, die App schreibt sie beim
  Speichern in den `<head>`)
- **theme** – `light`, `dark` oder `auto` (folgt dem System)
- **statusbar** – Leiste mit Wörtern und Zeichen unter dem Text
- **fontFamilies**, **fontSizes**, **colors** – Einträge für Schriftart, Schriftgröße und die Farbpalette von
  Schriftfarbe und Hervorhebung. Ohne Angabe gängige Windows-Schriften, 8–72 pt und 20 Farben. Im HTML landen sie
  als Inline-Style (`<span style="color: …; font-size: 14pt">`, `<mark data-color="…">`)

Klassennamen: Buchstaben, Ziffern, `-`, `_`. Ungültige Einträge werden ignoriert. Beim Einfügen aus Word & Co.
bleiben nur Klassen erhalten, die ein konfiguriertes Format sind.

## Im Web

```html
<script src="mrtextedit.js"></script>

<mr-textedit id="text" name="text" config='{"paragraphFormats":[{"label":"Standard"},{"label":"Zitat","class":"zitat"}]}'>
  <p>Anfangstext</p>
</mr-textedit>

<script>
  const editor = document.getElementById("text");
  editor.value = "<p>Neuer Text</p>";            // hineingeben (kein change-Event)
  editor.addEventListener("change", () => {      // nach jeder Änderung durch den Benutzer
    console.log(editor.value, editor.text);      // HTML und reiner Text
  });
  editor.config = { toolbar: ["bold", "italic"] }; // jederzeit änderbar
</script>
```

`<mr-textedit>` ist auch ein Formularfeld (`name`) und kennt das Attribut `readonly`. Ohne Custom Element:

```js
const editor = MrTextEdit.create(document.getElementById("host"), { ...config, content: "<p>…</p>", onChange });
editor.getHTML(); editor.setHTML(html); editor.insertHTML(html); editor.getText();
editor.setConfig(config); editor.setReadOnly(true); editor.onChange(fn); editor.focus(); editor.destroy();
editor.toggleSource(); editor.toggleSearch(); editor.toggleFullscreen(); editor.print();
```

`getText()` liefert eine Zeile pro Absatz, Tabellenzeilen als eine Zeile mit Tab zwischen den Zellen.
`insertHTML` fügt an der Cursorposition ein (z. B. Textbausteine) und zählt als Änderung.

In Vue/Tauri-Apps dieses Repositorys direkt aus dem Quelltext: `import { TextEditor } from "@mrtools/textedit"`.
Farben lassen sich über CSS-Variablen anpassen (`--mrte-accent`, `--mrte-bg`, `--mrte-fg`, … in
`packages/textedit/src/textedit.css`).

## In VCL (TEdgeBrowser)

`mrtextedit.html` neben die .exe legen (dazu `WebView2Loader.dll` aus dem Redist-Ordner von RAD Studio) und im
`TEdgeBrowser` öffnen. Der Austausch läuft über die WebView2-Nachrichten – kein eigener Komponenten- oder
DLL-Code nötig:

| Richtung        | Wie                                                              | Inhalt                                          |
| --------------- | ---------------------------------------------------------------- | ----------------------------------------------- |
| Seite → Host    | `OnWebMessageReceived`, `TryGetWebMessageAsString`               | `{"type":"ready"}` – erst jetzt Befehle senden   |
|                 |                                                                  | `{"type":"change","html":"…","text":"…"}` – Benutzer hat geändert |
|                 |                                                                  | `{"type":"content","html":"…","text":"…"}` – nach `setHtml` |
| Host → Seite    | `ExecuteScript`                                                  | `mrte.setHtml("…")`, `mrte.insertHtml("…")`, `mrte.setConfig({…})`, `mrte.setReadOnly(true)`, `mrte.focus()` |
|                 | oder `PostWebMessageAsJson`                                      | `{"type":"setHtml","html":"…"}`, `{"type":"insertHtml","html":"…"}`, `{"type":"setConfig","config":{…}}`, … |

Alles Weitere erreicht man über `mrte.editor` (siehe „Im Web“), z. B. `mrte.editor.print()`.

Den aktuellen Text als HTML hält man am einfachsten aus den `change`/`content`-Nachrichten in einer Variablen –
dann ist er jederzeit synchron abrufbar. Strings für `ExecuteScript` mit `TJSONString::ToJSON()` maskieren.
Vollständiges Beispiel: [examples/vcl/MainForm.cpp](examples/vcl/MainForm.cpp).

## Desktop-App

- Neu, Öffnen (`.html`, `.htm`, `.txt`), Speichern, Speichern unter (<kbd>Strg</kbd>+<kbd>N</kbd>/<kbd>O</kbd>/<kbd>S</kbd>,
  <kbd>Strg</kbd>+<kbd>Umschalt</kbd>+<kbd>S</kbd>); als `.html` wird ein vollständiges HTML-Dokument mit den
  Format-Regeln gespeichert (geöffnet wird der Inhalt von `<body>`), als `.txt` nur der Text
- **Zuletzt geöffnet** (Pfeil neben „Öffnen“), **Drucken** (<kbd>Strg</kbd>+<kbd>P</kbd>)
- **Zoom** mit <kbd>Strg</kbd>+<kbd>+</kbd>/<kbd>−</kbd>/<kbd>0</kbd>, <kbd>Strg</kbd>+Mausrad oder in der Statusleiste
- Rückfrage bei ungespeicherten Änderungen (Neu, Öffnen, Schließen)
- **Toolleiste und Formate** als JSON (siehe oben), gespeichert in
  `%APPDATA%\de.mraudev.mrtextedit\config.json`
- Datei auf der Befehlszeile: `mrtextedit.exe brief.html`
- Wörter und Zeichen in der Statusleiste, dunkles/helles Design, automatische Updates

## Grenzen

- Inhalt ist das, was der Editor kennt (alles aus der Toolleiste oben). Anderes HTML (z. B. `<div>`-Layouts,
  Formulare, beliebige CSS-Eigenschaften) wird beim Hineingeben vereinfacht
- Ein Absatz/Textstück hat genau ein Absatz- bzw. Zeichenformat
- Bilder aus Dateien, der Zwischenablage oder per Ziehen werden als `data:`-URL ins HTML eingebettet – große
  Fotos machen das HTML entsprechend groß

## Technik

Editor: Tiptap 3 (ProseMirror) mit eigenen Erweiterungen für Absatzformat (`class` an `p`/`h1`–`h6`) und
Zeichenformat (`<span class>`), Symbole von Lucide. App: [Tauri 2](https://v2.tauri.app) mit Vue 3, TypeScript,
Tailwind CSS 4 und reka-ui – Aufbau und Aussehen wie [mrstart](../mrstart).

Entwicklung, Tests und Release: siehe [README im Repository-Root](../../README.md)
(`npm run tauri dev -w apps/mrtextedit`).
