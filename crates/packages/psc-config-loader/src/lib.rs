#![deny(clippy::all)]
#![forbid(unsafe_code)]

//! A library for loading and resolving secrets in configuration files.

use anyhow::Result;
use futures::future::BoxFuture;
use futures::FutureExt;
use psc_secrets::{SecretError, SecretManager};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// A loader for configuration files that can resolve secrets from a secret manager.
pub struct ConfigLoader<S: SecretManager> {
    secret_manager: S,
}

impl<S: SecretManager> ConfigLoader<S> {
    /// Creates a new `ConfigLoader` with the given secret manager.
    pub fn new(secret_manager: S) -> Self {
        Self { secret_manager }
    }

    /// Loads a configuration from the given source and resolves any secrets within it.
    ///
    /// # Arguments
    ///
    /// * `source` - A string containing the configuration in a format that can be deserialized
    ///              into a `serde_json::Value`.
    ///
    /// # Returns
    ///
    /// A deserialized configuration of type `T` with all secrets resolved, or an error if
    /// loading or secret resolution fails.
    pub async fn load_and_resolve<T: DeserializeOwned>(&self, source: &str) -> Result<T> {
        let mut config_value: Value = serde_json::from_str(source)?;
        self.resolve_secrets(&mut config_value).await?;
        let config: T = serde_json::from_value(config_value)?;
        Ok(config)
    }

    /// Recursively traverses a `serde_json::Value` and resolves any secret paths.
    fn resolve_secrets<'a>(
        &'a self,
        value: &'a mut Value,
    ) -> BoxFuture<'a, Result<(), SecretError>> {
        async move {
            match value {
                Value::Object(map) => {
                    for (_key, val) in map.iter_mut() {
                        self.resolve_secrets(val).await?;
                    }
                }
                Value::Array(arr) => {
                    for val in arr.iter_mut() {
                        self.resolve_secrets(val).await?;
                    }
                }
                Value::String(s) => {
                    if let Some(secret_path) = s.strip_prefix("vault://") {
                        let parts: Vec<&str> = secret_path.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let path = parts[0];
                            let key = parts[1];
                            let secret_value = self.secret_manager.get_secret(path, key).await?;
                            *s = secret_value;
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        }
        .boxed()
    }
}
