use crate::config::Config;
use crate::handlers;
use axum::http::{HeaderValue, header};
use axum::serve::ListenerExt;
use axum::{Router, routing::get};
use log::{debug, error, info};
use std::error;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;

/// Build the application router
pub fn build_app(config: &Config) -> Router {
    info!("Building application router");
    let static_service = ServeDir::new(&config.static_dir);
    debug!(
        "Static file service configured for directory: {}",
        config.static_dir
    );

    let router = Router::new()
        .route("/", get(handlers::homepage))
        .route("/healthz", get(handlers::health))
        .route("/roadmap", get(handlers::roadmap))
        .route("/roadmap/artery", get(handlers::roadmap_artery))
        .route("/docs", get(handlers::docs))
        .route("/docs/{page}", get(handlers::docs_page))
        .route(
            "/docs/c-bindings/{page}",
            get(handlers::docs_c_bindings_page),
        )
        .route("/sitemap.xml", get(handlers::sitemap))
        .route("/robots.txt", get(handlers::robots_txt))
        .nest_service(
            "/static",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=86400"),
                ))
                .service(static_service),
        )
        .fallback(handlers::not_found)
        .layer(
            ServiceBuilder::new()
                .layer(TimeoutLayer::new(Duration::from_secs(15)))
                // 500 instead of a dropped connection
                .layer(CatchPanicLayer::new())
                .layer(CompressionLayer::new()),
        );

    info!("Router configured with {} routes", 9);
    router
}

/// Resolve when the process is asked to stop, so in-flight requests finish before the
/// listener closes. SIGTERM comes from an orchestrator, Ctrl-C from a developer.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!("Failed to install Ctrl-C handler: {e}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                error!("Failed to install SIGTERM handler: {e}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl-C, shutting down"),
        _ = terminate => info!("Received SIGTERM, shutting down"),
    }
}

/// Run the server
pub async fn run_server(config: Config) -> Result<(), Box<dyn error::Error>> {
    info!("Starting server initialization");

    let app = build_app(&config);
    info!("Application routes configured");

    let addr = config.socket_addr();
    info!("Server will listen on {addr}");
    debug!("Full address: {addr:?}");

    let listener = match TcpListener::bind(addr).await {
        Ok(l) => {
            info!("TCP listener bound successfully to {addr}");
            // a compressed body goes out as several chunks, and without this Nagle
            // holds the last one for the delayed ACK: ~40 ms per keep-alive request
            l.tap_io(|stream| {
                if let Err(e) = stream.set_nodelay(true) {
                    error!("Failed to set TCP_NODELAY: {e}");
                }
            })
        }
        Err(e) => {
            error!("Failed to bind TCP listener to {addr}: {e}");
            return Err(e.into());
        }
    };

    info!("Starting HTTP server...");
    info!("Server is ready to accept connections on {addr}");

    match axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        Ok(_) => {
            info!("Server shutdown gracefully");
            Ok(())
        }
        Err(e) => {
            error!("Server error: {e}");
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::init_template_engine;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    const SERVED_PAGES: &[&str] = &[
        "/",
        "/roadmap",
        "/roadmap/artery",
        "/docs",
        "/docs/getting-started",
        "/docs/ir",
        "/docs/output",
        "/docs/cli",
        "/docs/rust",
        "/docs/irbuilder",
        "/docs/targets",
        "/docs/abi",
        "/docs/ras",
        "/docs/weld",
        "/docs/c-bindings",
        "/docs/c-bindings/install",
        "/docs/c-bindings/usage",
    ];

    fn real_config() -> Config {
        Config {
            template_dir: format!("{}/templates", env!("CARGO_MANIFEST_DIR")),
            ..Config::default()
        }
    }

    async fn get(app: &Router, path: &str) -> (StatusCode, String) {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    fn unresolved_placeholder(html: &str) -> Option<String> {
        let mut rest = html;
        while let Some(start) = rest.find("{{") {
            let after = &rest[start + 2..];
            let name_len = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .count();
            if name_len > 0 && after[name_len..].starts_with("}}") {
                return Some(format!("{{{{{}}}}}", &after[..name_len]));
            }
            rest = after;
        }
        None
    }

    #[tokio::test]
    async fn every_served_page_renders_without_unresolved_placeholders() {
        let config = real_config();
        init_template_engine(&config).expect("real templates load");
        let app = build_app(&config);

        let cases = SERVED_PAGES
            .iter()
            .map(|path| (*path, StatusCode::OK))
            .chain([("/no-such-page", StatusCode::NOT_FOUND)]);

        let mut failures = Vec::new();
        for (path, expected) in cases {
            let (status, html) = get(&app, path).await;
            if status != expected {
                failures.push(format!("{path}: status {status}, expected {expected}"));
            }
            if [
                "Failed to render",
                "could not be rendered",
                "<h1>Error</h1>",
            ]
            .iter()
            .any(|marker| html.contains(marker))
            {
                failures.push(format!("{path}: rendered an error page"));
            }
            if let Some(marker) = unresolved_placeholder(&html) {
                failures.push(format!("{path}: unresolved placeholder {marker}"));
            }
        }

        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[tokio::test]
    async fn every_sidebar_link_is_served() {
        let config = real_config();
        init_template_engine(&config).expect("real templates load");
        let app = build_app(&config);

        let (status, html) = get(&app, "/docs/getting-started").await;
        assert_eq!(status, StatusCode::OK);
        let start = html
            .find("<aside class=\"lmn-manual-sidebar\"")
            .expect("docs page includes the sidebar");
        let end = start + html[start..].find("</aside>").expect("sidebar is closed");
        let links: Vec<&str> = html[start..end]
            .split("href=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .filter(|href| href.starts_with('/'))
            .collect();
        assert!(!links.is_empty(), "sidebar lists internal links");

        let mut failures = Vec::new();
        for link in links {
            let (status, _) = get(&app, link).await;
            if status != StatusCode::OK {
                failures.push(format!("sidebar link {link}: status {status}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
