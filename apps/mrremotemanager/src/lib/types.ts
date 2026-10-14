export type Kind = "rdp" | "ssh" | "web";

/** A saved connection, stored as JSON by the backend (`connections.rs`). Never contains passwords. */
export interface Connection {
  id: string;
  name: string;
  kind: Kind;
  /** Host name or IP address; for "web" the address (scheme optional, https:// is added). */
  host: string;
  /** null: default port of the kind. */
  port: number | null;
  user: string;
  /** Free text; connections with the same group are listed together. */
  group: string;
  notes: string;
  /** RDP: full screen instead of a window. */
  fullscreen: boolean;
  /** RDP: use all monitors. */
  multimon: boolean;
  /** SSH: private key file (`-i`); empty: ssh's defaults. */
  keyFile: string;
  /** Unix time in ms of the last connect. */
  lastUsed: number | null;
}

/** A remote connection open right now (`active.rs`). */
export interface Active {
  kind: Kind;
  /** Host as the client was given it (name or IP); otherwise the remote IP. */
  host: string;
  port: number;
  user: string;
  keyFile: string;
  /** Remote IP address. */
  address: string;
  pid: number;
  /** Program, e.g. "mstsc.exe" or "putty.exe". */
  process: string;
}
