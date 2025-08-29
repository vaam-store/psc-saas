use camino::Utf8PathBuf;
use config::{Config, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Log {
    pub level: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log: Log,
}

impl Settings {
    pub fn new() -> psc_error::Result<Self> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| psc_error::Error::Internal(e.to_string()))?;
        let mut path = Utf8PathBuf::from(cargo_manifest_dir);
        path.pop();
        path.pop();
        path.pop();

        let s = Config::builder()
            .add_source(File::with_name(path.join("config/default").as_str()))
            .add_source(
                File::with_name(path.join(format!("config/{}", run_mode)).as_str()).required(false),
            )
            .add_source(File::with_name(path.join("config/local").as_str()).required(false))
            .add_source(Environment::with_prefix("app"))
            .build()
            .map_err(|e| psc_error::Error::Internal(e.to_string()))?;

        s.try_deserialize()
            .map_err(|e| psc_error::Error::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let settings = Settings::new();
        assert!(settings.is_ok());
        let settings = settings.unwrap();
        assert_eq!(settings.log.level, "info");
    }

    #[test]
    fn test_env_override_log_level() {
        // Preserve existing env var if any
        let prev = std::env::var("APP_LOG_LEVEL").ok();

        std::env::set_var("APP_LOG_LEVEL", "debug");
        let settings = Settings::new().expect("failed to load settings with env override");
        assert_eq!(settings.log.level, "debug");

        // Restore previous value
        if let Some(v) = prev {
            std::env::set_var("APP_LOG_LEVEL", v);
        } else {
            std::env::remove_var("APP_LOG_LEVEL");
        }
    }
}
