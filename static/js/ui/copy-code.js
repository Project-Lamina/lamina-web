export function initCopyButtons() {
  const blocks = document.querySelectorAll("pre:has(code), .lmn-install");
  if (!blocks.length) return;
  const status = document.createElement("div");
  status.className = "lmn-sr-only";
  status.setAttribute("role", "status");
  document.body.append(status);

  blocks.forEach((block) => {
    const code = block.querySelector("code");
    if (!code) return;
    const container = block.matches("pre")
      ? document.createElement("div")
      : block;
    if (container !== block) {
      container.className = "lmn-code-block";
      block.before(container);
      container.append(block);
      block.tabIndex = 0;
      block.setAttribute("aria-label", "Code example");
    }
    const button = document.createElement("button");
    button.type = "button";
    button.className = "lmn-copy";
    button.textContent = "Copy";
    button.setAttribute("aria-label", "Copy code");
    let reset;
    button.addEventListener("click", async () => {
      window.clearTimeout(reset);
      try {
        await navigator.clipboard.writeText(code.textContent);
        button.textContent = "Copied ✓";
        status.textContent = "Code copied to clipboard.";
      } catch {
        const range = document.createRange();
        range.selectNodeContents(code);
        const selection = window.getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        button.textContent = "Selected";
        status.textContent =
          "Clipboard unavailable. Code selected; use your copy shortcut.";
      }
      reset = window.setTimeout(() => {
        button.textContent = "Copy";
      }, 2200);
    });
    container.append(button);
  });
}
