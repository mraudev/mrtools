/** Mirrors the structs in `src-tauri/src`. */
export interface Installed {
  version: string | null;
  /** Path of the program, if it exists. */
  exe: string | null;
}

export interface App {
  /** Folder under `apps/`, also the prefix of the release tags. */
  folder: string;
  /** Product name, as shown in Windows' installed apps. */
  name: string;
  description: string | null;
  /** Logo as SVG source. */
  icon: string | null;
  installed: Installed | null;
}

export interface Release {
  version: string;
  url: string;
  publishedAt: string | null;
  /** Size of the installer in bytes, `null` if the release has none. */
  installerSize: number | null;
}
