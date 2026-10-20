import { invoke } from "@tauri-apps/api/core";
import type { Active, Candidate, Connection, ProbeStatus, Tools } from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  load: () => invoke<Connection[] | null>("load_connections"),
  save: (connections: Connection[]) => invoke<void>("save_connections", { connections }),
  exportTo: (path: string, connections: Connection[]) => invoke<void>("export_connections", { path, connections }),
  importFrom: (path: string) => invoke<unknown>("import_connections", { path }),

  connect: (connection: Connection) => invoke<void>("connect", { connection }),
  openSftp: (connection: Connection) => invoke<void>("open_sftp", { connection }),
  tools: () => invoke<Tools>("tools"),

  active: () => invoke<Active[]>("active_connections"),
  focusSession: (pid: number) => invoke<void>("focus_session", { pid }),
  endSession: (pid: number) => invoke<void>("end_session", { pid }),

  rdpPasswords: (hosts: string[]) => invoke<Record<string, boolean>>("rdp_passwords", { hosts }),
  setRdpPassword: (host: string, user: string, password: string) => invoke<void>("set_rdp_password", { host, user, password }),
  deleteRdpPassword: (host: string) => invoke<void>("delete_rdp_password", { host }),

  probe: (targets: { id: string; host: string; port: number }[]) => invoke<Record<string, ProbeStatus>>("probe", { targets }),

  importSources: () => invoke<Candidate[]>("import_sources"),
  importRdpFiles: (paths: string[]) => invoke<Candidate[]>("import_rdp_files", { paths }),
};
