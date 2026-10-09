const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/** Binary units like Windows: 1 KB = 1024 bytes. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  const digits = value < 10 ? 2 : value < 100 ? 1 : 0;
  const text = value.toLocaleString("de-DE", { minimumFractionDigits: digits, maximumFractionDigits: digits });
  return `${text} ${UNITS[unit]}`;
}

export function formatRate(bytesPerSecond: number): string {
  return bytesPerSecond > 0 ? `${formatBytes(bytesPerSecond)}/s` : "0";
}

export function formatCount(count: number): string {
  return count.toLocaleString("de-DE");
}

export function formatCpu(percent: number): string {
  if (percent <= 0) return "0";
  if (percent < 0.1) return "< 0,1";
  return percent.toLocaleString("de-DE", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
}

export function formatPercent(part: number, whole: number): string {
  if (whole <= 0) return "–";
  const value = (part / whole) * 100;
  return `${value.toLocaleString("de-DE", { minimumFractionDigits: 1, maximumFractionDigits: 1 })} %`;
}

/** Seconds since 1970 → "08.10.2026, 14:03:12". */
export function formatDateTime(seconds: number): string {
  if (!seconds) return "–";
  return new Date(seconds * 1000).toLocaleString("de-DE", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** Elapsed ms → "3 T 04:05:06", "04:05:06" or "05:06". */
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const days = Math.floor(total / 86400);
  const h = Math.floor((total % 86400) / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  const clock = h || days ? `${pad(h)}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
  return days ? `${days} T ${clock}` : clock;
}
