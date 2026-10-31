/* Run against the local server. The sitemap supplies the public page inventory. */
const { chromium } = require("playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");
const axePath = require.resolve("axe-core/axe.min.js");
const origin = process.env.BASE_URL || "http://127.0.0.1:5127";

async function audit() {
  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({
      permissions: ["clipboard-read", "clipboard-write"],
    });
    const page = await context.newPage();
    const errors = [];
    const failures = [];
    const documents = new Map();
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("response", (response) => {
      if (response.url().startsWith(origin + "/static/") && !response.ok())
        errors.push(`Asset ${response.status()}: ${response.url()}`);
    });
    const sitemap = await (
      await context.request.get(origin + "/sitemap.xml")
    ).text();
    const routes = [...sitemap.matchAll(/<loc>(.*?)<\/loc>/g)].map(
      (match) => new URL(match[1]).pathname,
    );
    assert(routes.length > 10, "Sitemap must supply the page inventory");
    const missing = [
      "/missing-page",
      "/docs/missing-page",
      "/docs/c-bindings/missing-page",
    ];
    let referenceHeader;
    let referenceFooter;
    let rowChecks = 0;
    let accessibilityChecks = 0;

    for (const width of [320, 390, 768, 1024, 1440]) {
      await page.setViewportSize({ width, height: 950 });
      for (const route of [...routes, ...missing]) {
        const response = await page.goto(origin + route);
        assert.equal(
          response.status(),
          missing.includes(route) ? 404 : 200,
          route,
        );
        // Modules are deferred; wait for navigation enhancement to finish.
        await page
          .locator(".lmn-nav-disclosure")
          .waitFor({ state: "attached" });
        const result = await page.evaluate(() => {
          const failures = [];
          const count = (selector) =>
            document.querySelectorAll(selector).length;
          for (const selector of [
            "main",
            "h1",
            ".lmn-header",
            ".footer",
            "#main-content",
          ]) {
            if (count(selector) !== 1)
              failures.push(`${selector}: expected exactly one`);
          }
          const ids = [...document.querySelectorAll("[id]")].map((el) => el.id);
          if (new Set(ids).size !== ids.length) failures.push("Duplicate IDs");
          if (document.documentElement.scrollWidth > innerWidth + 1)
            failures.push("Page overflow");
          const headings = [
            ...document.querySelectorAll("main h1, main h2, main h3, main h4"),
          ];
          headings.forEach((heading, i) => {
            if (i && +heading.tagName[1] > +headings[i - 1].tagName[1] + 1)
              failures.push(`Skipped heading level: ${heading.textContent}`);
          });
          const normalize = (selector) => {
            const node = document.querySelector(selector).cloneNode(true);
            node
              .querySelectorAll("[aria-current]")
              .forEach((el) => el.removeAttribute("aria-current"));
            node
              .querySelectorAll("details")
              .forEach((el) => el.removeAttribute("open"));
            return node.outerHTML;
          };
          let rows = 0;
          document.querySelectorAll(".lmn-reference-table").forEach((table) => {
            if (table.tagName !== "TABLE")
              failures.push("Reference data must use a table");
            const headers = [...table.querySelectorAll("thead th")].map((th) =>
              th.textContent.trim(),
            );
            const columns = headers.length;
            table.querySelectorAll("tbody tr").forEach((row) => {
              rows++;
              if (row.cells.length !== columns)
                failures.push("Incorrect table cell count");
              [...row.cells].forEach((cell, i) => {
                if (cell.dataset.label !== headers[i])
                  failures.push("Mobile label differs from column heading");
                if (cell.scrollWidth > cell.clientWidth + 1)
                  failures.push("Cell overflow");
                if (
                  innerWidth > 760 &&
                  Math.abs(
                    cell.getBoundingClientRect().top -
                      row.getBoundingClientRect().top,
                  ) > 1
                )
                  failures.push("Misaligned table cells");
                if (
                  innerWidth <= 760 &&
                  i &&
                  cell.getBoundingClientRect().top <
                    row.cells[i - 1].getBoundingClientRect().bottom - 1
                )
                  failures.push("Overlapping mobile cells");
              });
            });
          });
          document.querySelectorAll(".lmn-doc-table > p").forEach((row) => {
            rows++;
            if (row.children.length !== 2)
              failures.push(
                "Definition row must have a label and a description",
              );
            [...row.children].forEach((cell) => {
              if (cell.scrollWidth > cell.clientWidth + 1)
                failures.push("Definition cell overflow");
            });
          });
          return {
            failures,
            rows,
            header: normalize(".lmn-header"),
            footer: normalize(".footer"),
            ids,
            links: [...document.querySelectorAll("a[href]")].map((a) =>
              a.getAttribute("href"),
            ),
          };
        });
        result.failures.forEach((failure) =>
          failures.push(`${width} ${route}: ${failure}`),
        );
        referenceHeader ??= result.header;
        referenceFooter ??= result.footer;
        assert.equal(
          result.header,
          referenceHeader,
          `Header differs on ${route}`,
        );
        assert.equal(
          result.footer,
          referenceFooter,
          `Footer differs on ${route}`,
        );
        rowChecks += result.rows;
        if (width === 1440) documents.set(route, result);
        if (width === 390 || width === 1440) {
          await page.addScriptTag({ path: axePath });
          const accessibility = await page.evaluate(async () => {
            const results = await axe.run(document, {
              runOnly: {
                type: "tag",
                values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"],
              },
            });
            return results.violations.map(
              (v) =>
                `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`,
            );
          });
          accessibility.forEach((failure) =>
            failures.push(`${width} ${route}: ${failure}`),
          );
          accessibilityChecks++;
        }
      }
      console.log(`Checked every page at ${width}px.`);
    }

    // Check local links and cross-page fragment targets, including content below the fold.
    for (const [route, doc] of documents) {
      for (const href of doc.links) {
        const url = new URL(href, origin + route);
        if (url.origin !== origin) continue;
        const target = documents.get(url.pathname);
        if (!target) {
          failures.push(`${route}: unserved internal link ${href}`);
          continue;
        }
        if (
          url.hash &&
          !target.ids.includes(decodeURIComponent(url.hash.slice(1)))
        )
          failures.push(`${route}: missing anchor ${href}`);
      }
    }

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(origin + "/docs/getting-started");
    const menu = page.locator(".lmn-nav-disclosure");
    await menu.locator("summary").click();
    assert(await page.locator(".lmn-nav").isVisible());
    await page.keyboard.press("Escape");
    assert(!(await page.locator(".lmn-nav").isVisible()));
    const sidebar = page.locator(".lmn-sidebar-disclosure");
    await sidebar.locator("summary").click();
    assert(await page.locator(".lmn-manual-sidebar").isVisible());
    await page.keyboard.press("Escape");
    assert(!(await page.locator(".lmn-manual-sidebar").isVisible()));
    await page.locator(".lmn-copy").first().click();
    assert(
      (await page.evaluate(() => navigator.clipboard.readText())).includes(
        "cargo install lamina",
      ),
    );
    await page.setViewportSize({ width: 1440, height: 950 });
    await page.locator(".lmn-nav").waitFor({ state: "visible" });
    await page.locator(".lmn-manual-sidebar").waitFor({ state: "visible" });

    const fallbackContext = await browser.newContext({
      javaScriptEnabled: false,
      viewport: { width: 390, height: 844 },
    });
    const fallback = await fallbackContext.newPage();
    for (const route of [
      "/",
      "/docs/getting-started",
      "/roadmap",
      "/missing-page",
    ]) {
      await fallback.goto(origin + route);
      assert(await fallback.locator(".lmn-nav").isVisible());
      await fallback.keyboard.press("Tab");
      await fallback.keyboard.press("Enter");
      assert.equal(
        await fallback.locator(":focus").getAttribute("id"),
        "main-content",
        route,
      );
    }
    await fallback.goto(origin + "/docs/ir");
    assert.equal(
      await fallback
        .locator('.lmn-manual-sidebar [aria-current="page"]')
        .count(),
      1,
    );

    if (process.env.SCREENSHOT_DIR) {
      await fs.mkdir(process.env.SCREENSHOT_DIR, { recursive: true });
      for (const width of [390, 1440]) {
        await page.setViewportSize({ width, height: 950 });
        for (const route of [
          "/",
          "/docs",
          "/docs/getting-started",
          "/roadmap",
          "/gallery",
          "/missing-page",
        ]) {
          await page.goto(origin + route);
          await page.screenshot({
            path: path.join(
              process.env.SCREENSHOT_DIR,
              `${route.replaceAll("/", "_") || "home"}-${width}.png`,
            ),
          });
        }
      }
    }
    assert.deepEqual(errors, [], "Browser and asset errors");
    assert.deepEqual(failures, [], failures.join("\n"));
    console.log(
      `PASS: ${(routes.length + missing.length) * 5} page layouts, ${rowChecks} table rows, ${accessibilityChecks} accessibility scans, links, shared chrome, keyboard, clipboard, resize, and no-JS fallback.`,
    );
  } finally {
    await browser.close();
  }
}
audit().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
