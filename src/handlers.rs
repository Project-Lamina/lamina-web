use crate::components::sitemap::generate_sitemap_xml;
use crate::config::Config;
use crate::releases::homepage_release_summary;
use crate::templates::TemplateEngine;
use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{Html, Response},
};
use log::{debug, error, info};
use std::collections::HashMap;

/// Homepage handler
pub async fn homepage() -> Html<String> {
    info!("Serving homepage request");
    debug!("Homepage route accessed");

    info!("Homepage served successfully");

    let template_engine = get_template_engine();
    let mut variables = HashMap::new();
    variables.insert(
        "homepage_release_summary".to_string(),
        homepage_release_summary(),
    );

    match template_engine.render("lamina_homepage.html", &variables) {
        Ok(content) => {
            match template_engine.render_base_with_meta_and_css(
                "Lamina - Typed IR compiler backend",
                &content,
                "Lamina compiles a compact, typed IR into machine code. Use it for native builds, nightly JIT compilation, or as the backend for a language frontend.",
                None,
            ) {
                Ok(html) => Html(html),
                Err(e) => {
                    error!("Failed to render base template: {e}");
                    Html(format!("<h1>Error</h1><p>Failed to render page: {e}</p>"))
                }
            }
        }
        Err(e) => {
            error!("Failed to render homepage template: {e}");
            Html(format!("<h1>Error</h1><p>Failed to render page: {e}</p>"))
        }
    }
}

/// Renders nothing on purpose: says the process is serving without touching templates,
/// disk, or the network.
pub async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

/// Roadmap handler
pub async fn roadmap() -> Html<String> {
    info!("Serving roadmap request");
    render_simple_page(
        "roadmap.html",
        "Lamina Roadmap",
        "Lamina toolchain roadmap: current target support, milestones from 0.1 to 1.0, and the testing that backs them.",
    )
}

/// Render a variable-free template inside the base layout.
fn render_simple_page(template: &str, title: &str, description: &str) -> Html<String> {
    let template_engine = get_template_engine();
    let variables = HashMap::new();

    let content = match template_engine.render(template, &variables) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to render {template}: {e}");
            return Html(render_error_page(&e.to_string()));
        }
    };

    match template_engine.render_base_with_meta_and_css(title, &content, description, None) {
        Ok(html) => Html(html),
        Err(e) => {
            error!("Failed to render base template for {template}: {e}");
            Html(render_error_page(&e.to_string()))
        }
    }
}

/// Used when even the base template will not render. Escaped: can carry a path.
fn render_error_page(message: &str) -> String {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <title>Lamina</title></head><body>\
         <h1>This page could not be rendered</h1><p>{escaped}</p>\
         <p><a href=\"/\">Return to the homepage</a></p></body></html>"
    )
}

/// Artery target page, split out of the roadmap because it stands on its own.
pub async fn roadmap_artery() -> Html<String> {
    info!("Serving roadmap artery request");
    render_simple_page(
        "roadmap_artery.html",
        "Lamina on the Artery Kernel",
        "Plan for supporting the Artery kernel as a freestanding Lamina target: the constraints it imposes on code generation, the binary format, system calls, and the work items.",
    )
}

/// Official Lamina documentation handler
pub async fn docs() -> Html<String> {
    info!("Serving Lamina documentation");
    debug!("Documentation route accessed");

    render_docs_template(
        "docs_index.html",
        "Lamina Documentation",
        "Documentation for the Lamina compiler and its IR, ras, weld, camina, and the lamina-c C bindings.",
        "home",
    )
}

/// Focused documentation page handler
pub async fn docs_page(Path(page): Path<String>) -> Result<Html<String>, StatusCode> {
    info!("Serving Lamina documentation page: {page}");

    let (template, title, description, active) = match page.as_str() {
        "getting-started" => (
            "docs_getting_started.html",
            "Getting Started",
            "Install Lamina, compile a small typed IR module, and choose AOT, nightly JIT, MIR, or assembly output.",
            "getting_started",
        ),
        "output" => (
            "docs_output.html",
            "Writing Output",
            "Lamina output and I/O: the byte I/O contract, writebyte and write, and calling printf through an extern declaration.",
            "output",
        ),
        "ir" => (
            "docs_ir.html",
            "IR Language",
            "Lamina IR as a typed, block-based text format with mutable slots rather than SSA: types, declarations, annotations, and the rules for reads and assignments.",
            "ir",
        ),
        "ir-instructions" => (
            "docs_ir_instructions.html",
            "IR Instructions",
            "Every Lamina IR instruction form: arithmetic, conversions, memory, atomics, calls, control flow, and I/O, with examples.",
            "ir_instructions",
        ),
        "runtime" => (
            "docs_runtime.html",
            "Runtime and libc",
            "Lamina needs no C library. The compiler runtime routines it may call for 128-bit arithmetic, and how to redirect them with LAMINA_BUILTINS.",
            "runtime",
        ),
        "lumir" => (
            "docs_lumir.html",
            "LUMIR",
            "LUMIR, the lower intermediate form that Lamina optimizes and lowers to machine code, and how it differs from the IR text.",
            "lumir",
        ),
        "camina" => (
            "docs_camina.html",
            "camina C Compiler",
            "camina, an early C compiler that lowers a subset of C to Lamina IR: what compiles, what does not, and the stdio gap.",
            "camina",
        ),
        "cli" => (
            "docs_cli.html",
            "Compiler Driver",
            "Reference for Lamina compiler-driver options, output modes, toolchain selection, targets, debugging, and register allocation.",
            "cli",
        ),
        "rust" => (
            "docs_rust.html",
            "Rust API Guide",
            "Use Lamina from Rust with assembly helpers, target compilation, nightly runtime compilation, the inline macro, and docs.rs.",
            "rust",
        ),
        "irbuilder" => (
            "docs_irbuilder.html",
            "IRBuilder Reference",
            "Construct Lamina modules in Rust with IRBuilder functions for blocks, values, memory, control flow, and metadata.",
            "irbuilder",
        ),
        "targets" => (
            "docs_targets.html",
            "Targets and Backends",
            "Tested and experimental Lamina target identifiers for x86_64, AArch64, RISC-V, and WebAssembly.",
            "targets",
        ),
        "abi" => (
            "docs_abi.html",
            "Calling Conventions",
            "Which calling convention each Lamina backend implements, how aggregates and floats are classified, and what is still refused.",
            "abi",
        ),
        "ras" => (
            "docs_ras.html",
            "ras Assembler",
            "ras, the assembler: install it, use it through the Lamina driver or in place of GNU as, and the gaps it has.",
            "ras",
        ),
        "weld" => (
            "docs_weld.html",
            "weld Linker",
            "Install weld and use it in place of ld, lld, or mold, with the flags it accepts and the gaps it still has.",
            "weld",
        ),
        "c-bindings" => (
            "docs_c_bindings.html",
            "C Bindings Development SDK",
            "Use the Lamina C bindings development SDK: owned IRBuilder handles, AOT assembly compilation, release status, and the nightly JIT extension.",
            "c_bindings",
        ),
        _ => return Err(StatusCode::NOT_FOUND),
    };

    Ok(render_docs_template(template, title, description, active))
}

/// Focused pages for the C bindings development SDK.
pub async fn docs_c_bindings_page(Path(page): Path<String>) -> Result<Html<String>, StatusCode> {
    info!("Serving Lamina C bindings documentation page: {page}");

    let (template, title, description, active) = match page.as_str() {
        "install" => (
            "docs_c_bindings_install.html",
            "Install the C Bindings",
            "Download the Lamina C bindings development SDK or build it from source. Review the headers, Linux release archives, and nightly JIT bundle.",
            "c_bindings_install",
        ),
        "usage" => (
            "docs_c_bindings_usage.html",
            "Use Lamina from C",
            "Use the Lamina C bindings development SDK: owned IRBuilder handles, module serialization, AOT assembly output, buffers, and nightly JIT boundaries.",
            "c_bindings_usage",
        ),
        _ => return Err(StatusCode::NOT_FOUND),
    };

    Ok(render_docs_template(template, title, description, active))
}

fn render_docs_template(
    template: &str,
    title: &str,
    description: &str,
    active: &str,
) -> Html<String> {
    let template_engine = get_template_engine();
    let mut variables = HashMap::new();
    for page in [
        "home",
        "getting_started",
        "cli",
        "ir",
        "ir_instructions",
        "runtime",
        "lumir",
        "camina",
        "output",
        "targets",
        "abi",
        "ras",
        "weld",
        "rust",
        "irbuilder",
        "c_bindings",
        "c_bindings_install",
        "c_bindings_usage",
    ] {
        variables.insert(format!("docs_active_{page}"), String::new());
    }
    variables.insert(format!("docs_active_{active}"), "is-active".to_string());

    match template_engine.render(template, &variables) {
        Ok(content) => {
            match template_engine.render_base_with_meta_and_css(title, &content, description, None)
            {
                Ok(html) => Html(html),
                Err(e) => {
                    error!("Failed to render documentation base template: {e}");
                    Html(format!("<h1>Error</h1><p>Failed to render page: {e}</p>"))
                }
            }
        }
        Err(e) => {
            error!("Failed to render documentation template: {e}");
            Html(format!("<h1>Error</h1><p>Failed to render page: {e}</p>"))
        }
    }
}

/// Sitemap XML handler
pub async fn sitemap() -> Result<Response<String>, StatusCode> {
    info!("Serving sitemap.xml request");
    debug!("Sitemap route accessed");

    let base_url = get_config().base_url.clone();

    match generate_sitemap_xml(&base_url) {
        Ok(xml) => {
            info!("Sitemap generated successfully");
            debug!("Sitemap XML length: {} chars", xml.len());

            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
                .body(xml)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(response)
        }
        Err(e) => {
            error!("Failed to generate sitemap: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Robots.txt handler
pub async fn robots_txt() -> Result<Response<String>, StatusCode> {
    info!("Serving robots.txt request");
    debug!("Robots.txt route accessed");

    let base_url = get_config().base_url.clone();

    let robots_content = format!(
        "User-agent: *\n\
         Allow: /\n\
         \n\
         Sitemap: {}/sitemap.xml\n",
        base_url
    );

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(robots_content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

/// 404 Not Found handler
pub async fn not_found() -> (StatusCode, Html<String>) {
    info!("Serving 404 page");
    debug!("404 route accessed");

    let template_engine = get_template_engine();
    let variables = HashMap::new();

    let html = match template_engine.render("404.html", &variables) {
        Ok(content) => {
            match template_engine.render_base_with_meta_and_css(
                "Page Not Found",
                &content,
                "The requested Lamina page does not exist. Open the compiler documentation or return to the homepage.",
                None,
            ) {
                Ok(html) => Html(html),
                Err(e) => {
                    error!("Failed to render 404 base template: {e}");
                    Html(format!("<h1>404 - Page Not Found</h1><p>Failed to render page: {e}</p>"))
                }
            }
        }
        Err(e) => {
            error!("Failed to render 404 template: {e}");
            Html(format!("<h1>404 - Page Not Found</h1><p>Failed to render page: {e}</p>"))
        }
    };

    (StatusCode::NOT_FOUND, html)
}

fn get_template_engine() -> &'static TemplateEngine {
    crate::routes::get_template_engine()
}

fn get_config() -> &'static Config {
    crate::routes::get_config()
}
