use crate::config::Config;
use crate::templates::TemplateEngine;
use log::debug;
use std::error::Error;
use std::sync::OnceLock;

/// Global template engine instance
static TEMPLATE_ENGINE: OnceLock<TemplateEngine> = OnceLock::new();

/// Global config instance
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Load every template into the global instance. Called once at startup so a missing
/// template stops the process there instead of panicking inside the first request.
pub fn init_template_engine(config: &Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    debug!("Initializing global template engine");
    let engine = TemplateEngine::new(&config.template_dir)?;
    let _ = TEMPLATE_ENGINE.set(engine);
    Ok(())
}

/// Present for every handler: startup calls `init_template_engine` and exits on error.
pub fn get_template_engine() -> &'static TemplateEngine {
    TEMPLATE_ENGINE
        .get()
        .expect("template engine is initialized during startup")
}

/// Get the global config instance
pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| {
        debug!("Initializing global config");
        Config::from_file_or_env()
    })
}
