use crate::components::sitemap::generate_sitemap_xml;
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
        crate::releases::homepage_release_summary().await,
    );

    match template_engine.render("lamina_homepage.html", &variables) {
        Ok(content) => {
            match template_engine.render_base_with_meta_and_css(
                "Lamina - Typed SSA compiler backend",
                &content,
                "Lamina compiles a compact, typed SSA IR into machine code. Use it for native builds, nightly JIT compilation, or as the backend for a language frontend.",
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

/// Official Lamina documentation handler
pub async fn docs() -> Html<String> {
    info!("Serving Lamina documentation");
    debug!("Documentation route accessed");

    render_docs_template(
        "docs_index.html",
        "Lamina Documentation",
        "Documentation for Lamina IR, the compiler driver, Rust embedding, IRBuilder, target backends, and the C bindings development SDK.",
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
            "Install Lamina, compile a small typed SSA IR module, and choose AOT, nightly JIT, MIR, or assembly output.",
            "getting_started",
        ),
        "ir" => (
            "docs_ir.html",
            "IR Language Reference",
            "Reference for Lamina typed SSA IR syntax, types, values, instructions, memory ownership, and portable output semantics.",
            "ir",
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
        "targets",
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

    // Get base URL from config (loaded from file/env/defaults)
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

    // Get base URL from config (loaded from file/env/defaults)
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

fn get_config() -> &'static crate::config::Config {
    crate::routes::get_config()
}
