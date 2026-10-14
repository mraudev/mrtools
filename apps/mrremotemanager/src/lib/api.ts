import { invoke } from "@tauri-apps/api/core";
import type { Active, Connection } from "./types";

/** Typed wrappers around the Rust commands in `src-tauri/src`. */
export const api = {
  load: () => invoke<Connection[] | null>("load_connections"),
  save: (connections: Connection[]) => invoke<void>("save_connections", { connections }),
  connect: (connection: Connection) => invoke<void>("connect", { connection }),
  active: () => invoke<Active[]>("active_connections"),
};
