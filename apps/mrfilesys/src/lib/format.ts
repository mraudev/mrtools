const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/** Binary units like Windows Explorer: 1 KB = 1024 bytes. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
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

export function formatCount(count: number): string {
  return count.toLocaleString("de-DE");
}

export function formatPercent(part: number, whole: number): string {
  if (whole <= 0) return "–";
  const value = (part / whole) * 100;
  return `${value.toLocaleString("de-DE", { minimumFractionDigits: 1, maximumFractionDigits: 1 })} %`;
}

export function formatDate(ms: number): string {
  if (!ms) return "–";
  return new Date(ms).toLocaleString("de-DE", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
