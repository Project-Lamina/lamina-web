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

    verify_static_dir(&config.static_dir)?;
    info!("Static assets served from {}", config.static_dir);

    crate::routes::init_template_engine(config)
        .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
    info!("Templates loaded from {}", config.template_dir);

    crate::releases::spawn_refresh_task();

    info!("Application initialization completed successfully");
    Ok(())
}

/// Fail startup if the static directory is missing, unreadable, or not a directory.
/// Without this the site serves unstyled pages while the process looks healthy.
fn verify_static_dir(static_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(static_dir);
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());

    match std::fs::metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(format!(
            "Static path {static_dir} is not a directory (working directory: {cwd}; \
             set static_dir or STATIC_DIR)"
        )
        .into()),
        Err(e) => Err(format!(
            "Cannot read static directory {static_dir}: {e} (working directory: {cwd}; \
             set static_dir or STATIC_DIR)"
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_static_directory_is_rejected() {
        assert!(verify_static_dir("definitely-not-a-directory-here").is_err());
    }

    #[test]
    fn a_file_is_not_accepted_as_the_static_directory() {
        assert!(verify_static_dir("Cargo.toml").is_err());
    }

    #[test]
    fn the_repository_static_directory_is_accepted() {
        assert!(verify_static_dir("static").is_ok());
    }
}
