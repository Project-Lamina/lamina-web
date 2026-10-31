export function initContents() {
  const article = document.querySelector(".lmn-manual-article");
  if (!article) return;
  let links = article.querySelector(".lmn-roadmap-toc");
  const headings = [...article.querySelectorAll("h2[id]")];
  if (!links && headings.length < 3) return;

  const contents = document.createElement("details");
  contents.className = "lmn-page-toc";
  contents.open = true;
  const summary = document.createElement("summary");
  summary.textContent = "On this page";
  contents.append(summary);

  if (links) {
    links.before(contents);
  } else {
    links = document.createElement("nav");
    links.setAttribute("aria-label", "On this page");
    headings.forEach((heading) => {
      const link = document.createElement("a");
      link.href = `#${heading.id}`;
      link.textContent = heading.textContent;
      links.append(link);
    });
    headings[0].before(contents);
  }
  contents.append(links);
}
