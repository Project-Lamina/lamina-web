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

    info!("Application initialization completed successfully");
    Ok(())
}
