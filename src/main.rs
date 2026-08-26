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

    // Load configuration from file, env, or defaults
    let config = config::Config::from_file_or_env();
    debug!("Configuration loaded: {config:?}");

    // Initialize global config instance
    routes::get_config();

    // Initialize application
    app::init_app(&config).await?;

    // Run server
    if let Err(e) = server::run_server(config).await {
        error!("Server error: {e}");
        return Err(e);
    }

    Ok(())
}
