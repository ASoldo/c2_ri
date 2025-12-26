import { boot } from "/static/ui/main.js";

document.addEventListener("DOMContentLoaded", () => {
  boot().catch((error) => console.error("C2 UI bootstrap failed", error));
});
