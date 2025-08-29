#![deny(clippy::all)]
#![forbid(unsafe_code)]

//! A shared client for securely retrieving secrets from HashiCorp Vault or a cloud Key Management Service (KMS).

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

/// Error types for secret management operations.
#[derive(thiserror::Error, Debug)]
pub enum SecretError {
    #[error("Vault API error: {0}")]
    VaultApi(String),
    #[error("Secret not found at path '{path}' with key '{key}'")]
    SecretNotFound { path: String, key: String },
    #[error("Invalid secret data: {0}")]
    InvalidSecretData(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Authentication error: {0}")]
    Authentication(String),
}

/// Trait for abstracting secret management operations.
#[async_trait]
pub trait SecretManager: Send + Sync {
    /// Retrieves a secret from the specified path and key.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the secret (e.g., "secret/data/my-app/config").
    /// * `key` - The specific key within the secret to retrieve.
    ///
    /// # Returns
    ///
    /// The secret value as a String, or a `SecretError` if retrieval fails.
    async fn get_secret(&self, path: &str, key: &str) -> Result<String, SecretError>;
}

/// Configuration for the Vault client.
#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub addr: Url,
    pub token: Option<String>, // For token-based auth, e.g., during development
    pub mount_path: String,    // e.g., "secret" for KV v2
}

/// HashiCorp Vault implementation of `SecretManager`.
#[derive(Debug, Clone)]
pub struct VaultSecretManager {
    client: reqwest::Client,
    config: VaultConfig,
}

impl VaultSecretManager {
    pub fn new(config: VaultConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Authenticates with Vault using a token.
    async fn authenticate_token(&self) -> Result<(), SecretError> {
        if self.config.token.is_none() {
            return Err(SecretError::Authentication(
                "No Vault token provided".to_string(),
            ));
        }
        // In a real application, you'd validate the token or perform a login.
        // For simplicity, we assume the provided token is valid for direct use.
        Ok(())
    }

    /// Builds the full URL for a Vault secret.
    fn build_secret_url(&self, path: &str) -> Result<Url, SecretError> {
        let full_path = format!("{}/data/{}", self.config.mount_path, path);
        self.config
            .addr
            .join(&full_path)
            .map_err(SecretError::UrlParse)
    }
}

#[async_trait]
impl SecretManager for VaultSecretManager {
    async fn get_secret(&self, path: &str, key: &str) -> Result<String, SecretError> {
        self.authenticate_token().await?;

        let url = self.build_secret_url(path)?;

        let mut request = self.client.get(url);
        if let Some(token) = &self.config.token {
            request = request.header("X-Vault-Token", token);
        }

        let response = request.send().await?.error_for_status()?;
        let json_response: serde_json::Value = response.json().await?;

        #[derive(Deserialize)]
        struct VaultData {
            data: HashMap<String, serde_json::Value>,
        }

        #[derive(Deserialize)]
        struct VaultResponse {
            data: VaultData,
        }

        let vault_response: VaultResponse = serde_json::from_value(json_response).map_err(|e| {
            SecretError::InvalidSecretData(format!("Failed to parse Vault response: {}", e))
        })?;

        vault_response
            .data
            .data
            .get(key)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| SecretError::SecretNotFound {
                path: path.to_string(),
                key: key.to_string(),
            })
    }
}
