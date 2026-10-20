export type Kind = "rdp" | "ssh" | "web" | "vnc" | "telnet" | "serial" | "hyperv";

/** A saved connection, stored as JSON by the backend (`connections.rs`). Never contains passwords. */
export interface Connection {
  id: string;
  name: string;
  kind: Kind;
  /**
   * Host name or IP address; for "web" the address (scheme optional, https:// is added),
   * for "serial" the line (COM3), for "hyperv" the Hyper-V server (empty: this PC).
   */
  host: string;
  /** null: default port of the kind. */
  port: number | null;
  user: string;
  /** Free text; connections with the same group are listed together. */
  group: string;
  notes: string;
  favorite: boolean;
  /** Unix time in ms of the last connect. */
  lastUsed: number | null;

  // Remotedesktop
  /** Full screen instead of a window. */
  fullscreen: boolean;
  /** Use all monitors. */
  multimon: boolean;
  /** Window size; null: mstsc's default. */
  width: number | null;
  height: number | null;
  clipboard: boolean;
  /** Make the local drives available in the session. */
  drives: boolean;
  /** RD gateway host; empty: none. */
  gateway: string;
  /** Administrative session (`mstsc /admin`). */
  admin: boolean;

  // SSH
  /** Private key file (`-i`); empty: ssh's defaults. */
  keyFile: string;
  /** Jump host (`-J`), e.g. "user@bastion:22". */
  jumpHost: string;
  /** One forwarding per line: "L 8080:localhost:80", "R …" or "D 1080". */
  forwards: string;

  // Serial line
  /** null: 9600. */
  baud: number | null;

  // Hyper-V
  vmName: string;
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

/** A connection found on this PC that can be imported (`import.rs`). */
export interface Candidate {
  kind: Kind;
  name: string;
  host: string;
  port: number | null;
  user: string;
  baud: number | null;
  /** Where it was found, e.g. "PuTTY". */
  source: string;
  detail: string;
}

/** Result of the reachability check (`probe.rs`). */
export interface ProbeStatus {
  reachable: boolean;
  /** IP addresses of the host. */
  ips: string[];
}

/** Which optional programs are installed. */
export interface Tools {
  vnc: boolean;
  putty: boolean;
  winscp: boolean;
}
