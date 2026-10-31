// Keep desktop/mobile behavior in one place for both navigation disclosures.
export const desktopNavigation = window.matchMedia("(min-width: 901px)");

export function createDisclosure(element, label, className) {
  const details = document.createElement("details");
  details.className = className;
  const summary = document.createElement("summary");
  summary.textContent = label;
  element.before(details);
  details.append(summary, element);
  const sync = () => {
    details.open = desktopNavigation.matches;
  };
  sync();
  desktopNavigation.addEventListener("change", sync);
  details.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && details.open && !desktopNavigation.matches) {
      details.open = false;
      summary.focus();
      event.stopPropagation();
    }
  });
  return details;
}
