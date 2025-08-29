use psc_secrets::{SecretError, SecretManager, VaultConfig, VaultSecretManager};
use wiremock::matchers::{method, path, header};
use wiremock::{MockServer, Mock, ResponseTemplate};
use serde_json::json;
use url::Url;

async fn setup_mock_vault_server(secret_path: &str, secret_key: &str, secret_value: &str, token: &str) -> MockServer {
    let mock_server = MockServer::start().await;
    let vault_path = format!("/v1/secret/data/{}", secret_path);

    Mock::given(method("GET"))
        .and(path(&vault_path))
        .and(header("X-Vault-Token", token))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "request_id": "test-request-id",
            "lease_id": "",
            "renewable": false,
            "lease_duration": 0,
            "data": {
                "data": {
                    secret_key: secret_value,
                },
                "metadata": {
                    "created_time": "2023-01-01T00:00:00Z",
                    "version": 1,
                }
            },
            "wrap_info": null,
            "warnings": null,
            "auth": null
        })))
        .expect(1)
        .mount(&mock_server)
        .await;
    mock_server
}

#[tokio::test]
async fn test_vault_secret_manager_get_secret_success() {
    let secret_path = "my-app/config";
    let secret_key = "api_key";
    let secret_value = "supersecretkey";
    let vault_token = "my-root-token";

    let mock_server = setup_mock_vault_server(secret_path, secret_key, secret_value, vault_token).await;

    let config = VaultConfig {
        addr: Url::parse(&mock_server.uri()).unwrap(),
        token: Some(vault_token.to_string()),
        mount_path: "secret".to_string(),
    };
    let secret_manager = VaultSecretManager::new(config);

    let result = secret_manager.get_secret(secret_path, secret_key).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), secret_value);
}

#[tokio::test]
async fn test_vault_secret_manager_secret_not_found() {
    let secret_path = "my-app/config";
    let secret_key = "non_existent_key";
    let vault_token = "my-root-token";

    let mock_server = MockServer::start().await;
    let vault_path = format!("/v1/secret/data/{}", secret_path);

    Mock::given(method("GET"))
        .and(path(&vault_path))
        .and(header("X-Vault-Token", vault_token))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "request_id": "test-request-id",
            "data": {
                "data": {}, // Empty data, simulating key not found
                "metadata": {}
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = VaultConfig {
        addr: Url::parse(&mock_server.uri()).unwrap(),
        token: Some(vault_token.to_string()),
        mount_path: "secret".to_string(),
    };
    let secret_manager = VaultSecretManager::new(config);

    let result = secret_manager.get_secret(secret_path, secret_key).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SecretError::SecretNotFound { .. }));
}

#[tokio::test]
async fn test_vault_secret_manager_authentication_error() {
    let config = VaultConfig {
        addr: Url::parse("http://localhost:8200").unwrap(),
        token: None, // No token provided
        mount_path: "secret".to_string(),
    };
    let secret_manager = VaultSecretManager::new(config);

    let result = secret_manager.get_secret("some/path", "some_key").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SecretError::Authentication(_)));
}

#[tokio::test]
async fn test_vault_secret_manager_http_error() {
    let secret_path = "my-app/config";
    let secret_key = "api_key";
    let vault_token = "my-root-token";

    let mock_server = MockServer::start().await;
    let vault_path = format!("/v1/secret/data/{}", secret_path);

    Mock::given(method("GET"))
        .and(path(&vault_path))
        .and(header("X-Vault-Token", vault_token))
        .respond_with(ResponseTemplate::new(500)) // Simulate HTTP 500 error
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = VaultConfig {
        addr: Url::parse(&mock_server.uri()).unwrap(),
        token: Some(vault_token.to_string()),
        mount_path: "secret".to_string(),
    };
    let secret_manager = VaultSecretManager::new(config);

    let result = secret_manager.get_secret(secret_path, secret_key).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SecretError::Network(_)));
}