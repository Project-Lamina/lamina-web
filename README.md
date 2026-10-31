# lamina-web

Lamina's website and documentation, served by Rust and Axum. Templates, CSS, and JavaScript are served directly; there is no frontend build step.

Run from this directory:

```sh
cargo run --locked
```

The default port is 5000. For a separate preview, use `HOST=127.0.0.1 PORT=5127 cargo run --locked`.

## Shared layout and components

`TemplateEngine::render_page` wraps every page in `templates/base.html`. The base owns the header, footer, metadata, skip link, and assets. Page templates contain only their main content. Add global navigation links in `templates/components/header.html`, and documentation links in `templates/components/docs_sidebar.html`.

The active main-navigation item is determined by `navigation_state` in `src/templates.rs`; documentation handlers supply the active sidebar item. Both states render on the server and work without JavaScript. All page handlers use `render_html_page`, including the styled 404 response for unknown documentation URLs.

CSS modules share the tokens in `static/css/base.css`:

| File | Owns |
| --- | --- |
| `base.css` | Colors, fonts, spacing tokens, resets, focus, reduced motion |
| `components.css` | Header, footer, buttons, cards, code surfaces, syntax colors, reference tables |
| `docs.css` | Documentation layouts, prose, sidebar, breadcrumbs, callouts, contents menu |
| `home.css` | Homepage hero, pipeline, console, target and execution sections |
| `error.css` | Error-page content; uses shared buttons and chrome |

`static/js/site.js` initializes the modules in `static/js/ui/`: navigation, disclosure behavior, code copying, and contents menus. Disclosure behavior uses one shared desktop breakpoint (901px), matching the CSS. Content and navigation remain available without JavaScript.

## Adding or editing pages

1. Add the page template and register it in `TemplateEngine::new`.
2. Add its route/handler, sitemap entry, and applicable navigation link.
3. Use one `h1` and a focusable `id="main-content"` target. Use `h2` for sections, with stable IDs; longer articles receive an automatic contents menu. Existing hand-authored contents menus use the same UI.
4. Reuse the component classes. Reference data uses native `table`, `thead`, `tbody`, and `th scope="col"`/`th scope="row"` elements. Put aliases together in one cell. Each body cell has a `data-label` matching its column heading, which labels the stacked mobile layout.
5. Keep paragraph-style definition rows in `.lmn-doc-table`, with one label and one description per row. Wrap labels containing multiple names in `.lmn-doc-term`. Use `.lmn-manual-note` for callouts. Add page-specific rules to the appropriate CSS module rather than appending global overrides.

Assets are cached for a day. When updating assets, bump their version query in `base.html`; for JavaScript modules, also version changed import URLs.

## Validation

```sh
cargo test --locked
```

The optional browser audit discovers every public page from the sitemap. It checks shared navigation/footer markup, headings, local links and anchors, asset loading, tables at five screen widths, keyboard navigation, clipboard copying, JavaScript-free fallbacks, and automated WCAG A/AA accessibility rules.

With the local server running:

```sh
npm ci --prefix tests/ui
npm exec --prefix tests/ui -- playwright install chromium
BASE_URL=http://127.0.0.1:5127 npm --prefix tests/ui test
```

Set `SCREENSHOT_DIR=/tmp/lamina-web-screenshots` to save desktop and mobile previews. Browser dependencies are development-only and do not affect deployment.
