export interface Socket {
  protocol: "TCP" | "UDP";
  ipv6: boolean;
  localAddress: string;
  localPort: number;
  /** Empty for listening sockets and UDP. */
  remoteAddress: string;
  remotePort: number;
  /** "Lauscht", "Verbunden", …; empty for UDP. */
  state: string;
  pid: number;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  /** Empty if access is denied. */
  exe: string;
  cmd: string;
  /** Working folder – for node.exe and the like usually the project. */
  cwd: string;
}

export interface Snapshot {
  sockets: Socket[];
  processes: ProcessInfo[];
}
