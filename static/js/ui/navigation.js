import { createDisclosure, desktopNavigation } from "./disclosure.js";

export function initNavigation() {
  const nav = document.querySelector(".lmn-header .lmn-nav");
  if (nav) {
    const menu = createDisclosure(nav, "Menu", "lmn-nav-disclosure");
    nav.addEventListener("click", (event) => {
      if (event.target.closest("a") && !desktopNavigation.matches)
        menu.open = false;
    });
    document.addEventListener("click", (event) => {
      if (!menu.contains(event.target) && !desktopNavigation.matches)
        menu.open = false;
    });
  }
  const sidebar = document.querySelector(".lmn-manual-sidebar");
  if (sidebar) {
    const active = sidebar.querySelector('[aria-current="page"]');
    const label = active
      ? `Documentation · ${active.textContent}`
      : "Browse documentation";
    createDisclosure(sidebar, label, "lmn-sidebar-disclosure");
  }
}
