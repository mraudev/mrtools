/** Mirrors the structs in `src-tauri/src`. */
export interface Installed {
  version: string | null;
  /** Path of the program, if it exists. */
  exe: string | null;
}

export interface App {
  folder: string;
  path: string;
  name: string;
  description: string | null;
  /** Version in the source folder. */
  version: string | null;
  /** `owner/repo` on github.com. */
  github: string | null;
  /** Folder inside the repository (apps in a monorepo). */
  repoPath: string | null;
  /** Prefix of this app's release tags (apps in a monorepo), e.g. `mrprocs-v`. */
  tagPrefix: string | null;
  /** Logo as data URL. */
  icon: string | null;
  installed: Installed | null;
}

export interface Release {
  tag_name: string;
  html_url: string;
  published_at: string | null;
}

/** `undefined` = still loading, `null` = no release. */
export type ReleaseState = Release | null | undefined | { error: string };
