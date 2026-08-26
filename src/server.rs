use crate::config::Config;
use crate::handlers;
use axum::{Router, routing::get};
use log::{debug, error, info};
use std::error;
use tokio::net::TcpListener;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::ServeDir;

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
        .nest_service("/static", static_service)
        .fallback(handlers::not_found)
        // A panic in one handler returns 500 for that request instead of killing the
        // connection and leaving the client with a reset and no explanation.
        .layer(CatchPanicLayer::new());

    info!("Router configured with {} routes", 8);
    router
}

/// Resolve when the process is asked to stop.
///
/// Without this, a deploy or a container stop kills the process mid-response. Waiting
/// on both signals lets in-flight requests finish before the listener closes. SIGTERM
/// is the one an orchestrator sends; Ctrl-C is the one a developer sends.
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
                // Nothing to wait on, so leave Ctrl-C as the only shutdown path.
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
            l
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
