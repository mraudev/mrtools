import type { Settings } from "./types";

/** Accent colors: [color, readable foreground on that color]. */
export const ACCENTS: Record<string, [string, string]> = {
  amber: ["#f59e0b", "#18181b"],
  orange: ["#f97316", "#18181b"],
  red: ["#ef4444", "#ffffff"],
  rose: ["#f43f5e", "#ffffff"],
  pink: ["#ec4899", "#ffffff"],
  violet: ["#8b5cf6", "#ffffff"],
  indigo: ["#6366f1", "#ffffff"],
  blue: ["#3b82f6", "#ffffff"],
  sky: ["#0ea5e9", "#18181b"],
  teal: ["#14b8a6", "#18181b"],
  emerald: ["#10b981", "#18181b"],
  lime: ["#84cc16", "#18181b"],
};

const systemDark = window.matchMedia("(prefers-color-scheme: dark)");

export function applyTheme(settings: Pick<Settings, "theme" | "accent">) {
  const root = document.documentElement;
  const dark = settings.theme === "dark" || (settings.theme === "system" && systemDark.matches);
  root.classList.toggle("dark", dark);

  const [color, foreground] = ACCENTS[settings.accent] ?? ACCENTS.amber;
  root.style.setProperty("--accent", color);
  root.style.setProperty("--accent-foreground", foreground);
}

export function onSystemThemeChange(callback: () => void) {
  systemDark.addEventListener("change", callback);
}
