import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";

// The browser context menu (reload, print, …) makes no sense in a desktop app.
if (import.meta.env.PROD) {
  document.addEventListener("contextmenu", (event) => {
    if (!(event.target as HTMLElement).closest("input, textarea")) event.preventDefault();
  });
}

createApp(App).mount("#app");
