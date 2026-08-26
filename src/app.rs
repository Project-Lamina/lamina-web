use crate::config::Config;
use log::{debug, info};

/// Initialize the application
pub async fn init_app(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Lamina website initialization");

    info!("Starting Lamina website server...");
    debug!(
        "Configuration: host={:?}, port={}, static_dir={}",
        config.host, config.port, config.static_dir
    );

    // Load templates now so a missing or malformed one is a startup failure with a
    // clear message, not a panic inside the first request that needs it.
    crate::routes::init_template_engine()
        .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
    info!("Templates loaded");

    info!("Application initialization completed successfully");
    Ok(())
}
