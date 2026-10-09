import type { Entry, SortKey } from "./types";

export type Kind = "folder" | "image" | "video" | "audio" | "archive" | "code" | "text" | "table" | "pdf" | "program" | "file";

const KINDS: Record<string, Kind> = {};
const add = (kind: Kind, exts: string) => exts.split(" ").forEach((e) => (KINDS[e] = kind));
add("image", "png jpg jpeg gif webp bmp svg ico avif tif tiff heic psd");
add("video", "mp4 webm mkv mov avi wmv m4v");
add("audio", "mp3 wav flac ogg m4a aac wma opus");
add("archive", "zip 7z rar tar gz bz2 xz zst cab iso");
add("code", "js mjs cjs ts tsx jsx vue svelte rs py go java kt c h cpp hpp cs php rb swift html htm css scss less json jsonc yaml yml toml xml sql sh ps1 psm1 bat cmd lua dart");
add("text", "txt md markdown log ini cfg conf env csv tsv rtf gitignore editorconfig lock");
add("table", "xlsx xls ods");
add("pdf", "pdf");
add("program", "exe msi dll sys com appx msix lnk url");

/** Lower-case extension without dot, "" if none (`.gitignore` counts as extension "gitignore"). */
export function extension(name: string): string {
  const i = name.lastIndexOf(".");
  return i < 0 || i === name.length - 1 ? "" : name.slice(i + 1).toLowerCase();
}

export function kindOf(entry: Pick<Entry, "name" | "isDir">): Kind {
  if (entry.isDir) return "folder";
  return KINDS[extension(entry.name)] ?? "file";
}

/** Type column like Explorer: "Dateiordner", "PNG-Datei", "Datei". */
export function typeLabel(entry: Pick<Entry, "name" | "isDir">): string {
  if (entry.isDir) return "Dateiordner";
  const ext = extension(entry.name);
  return ext ? `${ext.toUpperCase()}-Datei` : "Datei";
}

/** What the detail panel can show inline. */
export function previewOf(entry: Pick<Entry, "name" | "isDir">): "image" | "video" | "audio" | "text" | null {
  if (entry.isDir) return null;
  const ext = extension(entry.name);
  if (["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "avif"].includes(ext)) return "image";
  if (["mp4", "webm", "m4v", "mov"].includes(ext)) return "video";
  if (["mp3", "wav", "flac", "ogg", "m4a", "aac", "opus"].includes(ext)) return "audio";
  const kind = kindOf(entry);
  // Unknown files are tried as text too – the backend says if they are binary.
  return kind === "code" || kind === "text" || kind === "file" ? "text" : null;
}

const collator = new Intl.Collator("de", { numeric: true, sensitivity: "base" });

/** Folders first, then by `key`; ties by name. Natural order for names ("2" < "10"). */
export function sortEntries(entries: Entry[], key: SortKey, ascending: boolean): Entry[] {
  const dir = ascending ? 1 : -1;
  const compare = (a: Entry, b: Entry): number => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    let result = 0;
    if (key === "size") result = a.size - b.size;
    else if (key === "modified") result = a.modified - b.modified;
    else if (key === "type") result = collator.compare(typeLabel(a), typeLabel(b));
    return result * dir || collator.compare(a.name, b.name) * (key === "name" ? dir : 1);
  };
  return [...entries].sort(compare);
}
