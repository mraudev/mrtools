import { ref, watchEffect } from "vue";

export type Theme = "light" | "dark";

const KEY = "theme";
const systemDark = window.matchMedia("(prefers-color-scheme: dark)");

function stored(): Theme | null {
  try {
    const value = localStorage.getItem(KEY);
    return value === "light" || value === "dark" ? value : null;
  } catch {
    return null;
  }
}

/** Follows the system until the user picks a theme in the status bar. */
export const theme = ref<Theme>(stored() ?? (systemDark.matches ? "dark" : "light"));

systemDark.addEventListener("change", (event) => {
  if (!stored()) theme.value = event.matches ? "dark" : "light";
});

export function toggleTheme() {
  theme.value = theme.value === "dark" ? "light" : "dark";
  try {
    localStorage.setItem(KEY, theme.value);
  } catch {
    // Not persisted – it still applies for this session.
  }
}

watchEffect(() => document.documentElement.classList.toggle("dark", theme.value === "dark"));
