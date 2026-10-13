import { reactive, watch } from "vue";

export type Tab = "lights" | "heating";

function loadTab(): Tab {
  try {
    return localStorage.getItem("tab") === "heating" ? "heating" : "lights";
  } catch {
    return "lights";
  }
}

/** Which part of the app is shown. */
export const ui = reactive({ tab: loadTab() });

watch(
  () => ui.tab,
  (tab) => {
    try {
      localStorage.setItem("tab", tab);
    } catch {
      // Not persisted – still applies for this session.
    }
  },
);
