mod app;
mod components;
mod config;
mod handlers;
mod logger;
mod releases;
mod routes;
mod server;
mod templates;

use log::{debug, error, info};
use logger::Logger;
use std::error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    // Initialize logger FIRST, before any other code
    match Logger::init() {
        Ok(_) => {
            info!("Logger initialized successfully");
        }
        Err(e) => {
            eprintln!("Failed to initialize logger: {e}");
            return Err(format!("Failed to initialize logger: {e}").into());
        }
    }

    // Parse once. The handlers read the same instance through routes::get_config, so
    // parsing here as well would duplicate every warning and leave two configs that
    // only agree by coincidence.
    let config = routes::get_config();
    debug!("Configuration loaded: {config:?}");

    // Initialize application
    app::init_app(config).await?;

    // Run server
    if let Err(e) = server::run_server(config.clone()).await {
        error!("Server error: {e}");
        return Err(e);
    }

    Ok(())
}
