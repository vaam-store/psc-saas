use async_trait::async_trait;
use psc_config_loader::ConfigLoader;
use psc_secrets::{SecretManager, SecretError};
use serde::Deserialize;
use std::collections::HashMap;
use anyhow::Result;

#[derive(Deserialize, Debug, PartialEq)]
struct TestConfig {
    api_key: String,
    database: DatabaseConfig,
    features: Vec<String>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct DatabaseConfig {
    url: String,
    pool_size: u32,
}

struct MockSecretManager {
    secrets: HashMap<String, String>,
}

#[async_trait]
impl SecretManager for MockSecretManager {
    async fn get_secret(&self, path: &str, key: &str) -> Result<String, SecretError> {
        let secret_key = format!("{}:{}", path, key);
        self.secrets.get(&secret_key)
            .cloned()
            .ok_or_else(|| SecretError::SecretNotFound {
                path: path.to_string(),
                key: key.to_string(),
            })
    }
}

#[tokio::test]
async fn test_config_loader_resolve_secrets() {
    let mut secrets = HashMap::new();
    secrets.insert("my-app/database:url".to_string(), "postgres://user:secret@localhost:5432/mydb".to_string());
    secrets.insert("my-app/api:key".to_string(), "supersecretkey".to_string());

    let secret_manager = MockSecretManager { secrets };
    let config_loader = ConfigLoader::new(secret_manager);

    let config_source = r#"
    {
        "api_key": "vault://my-app/api:key",
        "database": {
            "url": "vault://my-app/database:url",
            "pool_size": 10
        },
        "features": [
            "feature1",
            "feature2"
        ]
    }
    "#;

    let config: TestConfig = config_loader.load_and_resolve(config_source).await.unwrap();

    assert_eq!(config.api_key, "supersecretkey");
    assert_eq!(config.database.url, "postgres://user:secret@localhost:5432/mydb");
    assert_eq!(config.database.pool_size, 10);
    assert_eq!(config.features, vec!["feature1", "feature2"]);
}

#[tokio::test]
async fn test_config_loader_no_secrets() {
    let secret_manager = MockSecretManager { secrets: HashMap::new() };
    let config_loader = ConfigLoader::new(secret_manager);

    let config_source = r#"
    {
        "api_key": "some_key",
        "database": {
            "url": "postgres://user:password@localhost:5432/mydb",
            "pool_size": 5
        },
        "features": []
    }
    "#;

    let config: TestConfig = config_loader.load_and_resolve(config_source).await.unwrap();

    assert_eq!(config.api_key, "some_key");
    assert_eq!(config.database.url, "postgres://user:password@localhost:5432/mydb");
    assert_eq!(config.database.pool_size, 5);
    assert_eq!(config.features, Vec::<String>::new());
}

#[tokio::test]
async fn test_config_loader_secret_not_found() {
    let secret_manager = MockSecretManager { secrets: HashMap::new() };
    let config_loader = ConfigLoader::new(secret_manager);

    let config_source = r#"
    {
        "api_key": "vault://non-existent/path:key",
        "database": {
            "url": "postgres://user:password@localhost:5432/mydb",
            "pool_size": 5
        },
        "features": []
    }
    "#;

    let result = config_loader.load_and_resolve::<TestConfig>(config_source).await;
    assert!(result.is_err());
}