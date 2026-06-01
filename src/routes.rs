use crate::config::Config;
use crate::templates::TemplateEngine;
use log::debug;
use std::sync::OnceLock;

/// Global template engine instance
static TEMPLATE_ENGINE: OnceLock<TemplateEngine> = OnceLock::new();

/// Global config instance
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Get the global template engine instance
pub fn get_template_engine() -> &'static TemplateEngine {
    TEMPLATE_ENGINE.get_or_init(|| {
        debug!("Initializing global template engine");
        TemplateEngine::new().expect("Failed to initialize template engine")
    })
}

/// Get the global config instance
pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| {
        debug!("Initializing global config");
        Config::from_file_or_env()
    })
}
