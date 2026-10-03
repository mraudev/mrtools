const relative = new Intl.RelativeTimeFormat("de", { numeric: "auto" });

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 31_536_000],
  ["month", 2_592_000],
  ["week", 604_800],
  ["day", 86_400],
  ["hour", 3_600],
  ["minute", 60],
];

/** "vor 3 Tagen", "gestern", "gerade eben" … */
export function ago(date: string | number): string {
  const time = typeof date === "number" ? date : Date.parse(date);
  if (Number.isNaN(time)) return "";
  const seconds = (time - Date.now()) / 1000;
  for (const [unit, size] of UNITS) {
    if (Math.abs(seconds) >= size) return relative.format(Math.round(seconds / size), unit);
  }
  return "gerade eben";
}

export const WEEK_MS = 7 * 86_400_000;

export function olderThan(iso: string, ms: number): boolean {
  const time = Date.parse(iso);
  return !Number.isNaN(time) && Date.now() - time > ms;
}

export const dateTime = (date: string | number) =>
  new Date(date).toLocaleString("de", { dateStyle: "medium", timeStyle: "short" });
