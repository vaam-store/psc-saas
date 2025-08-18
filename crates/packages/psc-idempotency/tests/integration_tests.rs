use psc_idempotency::{IdempotencyStore, RedisIdempotencyStore};
use serde::{Deserialize, Serialize};
use tokio;
use uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestResult {
    value: String,
    count: u32,
}

#[tokio::test]
#[ignore] // This test requires a running Redis instance
async fn test_check_and_set_success() {
    let store =
        RedisIdempotencyStore::new("redis://127.0.0.1:6379").expect("Failed to create Redis store");
    let result = TestResult {
        value: "test".to_string(),
        count: 42,
    };

    // Use a unique key for each test run
    let key = format!("test_key_{}", uuid::Uuid::new_v4());

    let was_set = store
        .check_and_set(&key, &result, 60)
        .await
        .expect("Failed to check and set");
    assert!(was_set);
}

#[tokio::test]
#[ignore] // This test requires a running Redis instance
async fn test_check_and_set_duplicate() {
    let store =
        RedisIdempotencyStore::new("redis://127.0.0.1:6379").expect("Failed to create Redis store");
    let result1 = TestResult {
        value: "test1".to_string(),
        count: 42,
    };
    let result2 = TestResult {
        value: "test2".to_string(),
        count: 43,
    };

    // Use a unique key for each test run
    let key = format!("test_key_duplicate_{}", uuid::Uuid::new_v4());

    // First call should succeed
    let was_set1 = store
        .check_and_set(&key, &result1, 60)
        .await
        .expect("Failed to check and set first");
    assert!(was_set1);

    // Second call with same key should fail (not set)
    let was_set2 = store
        .check_and_set(&key, &result2, 60)
        .await
        .expect("Failed to check and set second");
    assert!(!was_set2);

    // Getting the result should return the first value
    let retrieved: Option<TestResult> = store.get_result(&key).await.expect("Failed to get result");
    assert_eq!(retrieved, Some(result1));
}

#[tokio::test]
#[ignore] // This test requires a running Redis instance
async fn test_get_result_not_found() {
    let store =
        RedisIdempotencyStore::new("redis://127.0.0.1:6379").expect("Failed to create Redis store");

    // Use a unique key for each test run
    let key = format!("non_existent_key_{}", uuid::Uuid::new_v4());

    let result: Option<TestResult> = store.get_result(&key).await.expect("Failed to get result");
    assert_eq!(result, None);
}

#[tokio::test]
#[ignore] // This test requires a running Redis instance
async fn test_ttl_expiration() {
    let store =
        RedisIdempotencyStore::new("redis://127.0.0.1:6379").expect("Failed to create Redis store");
    let result = TestResult {
        value: "test".to_string(),
        count: 42,
    };

    // Use a unique key for each test run
    let key = format!("test_key_ttl_{}", uuid::Uuid::new_v4());

    // Set with a very short TTL
    let was_set = store
        .check_and_set(&key, &result, 1)
        .await
        .expect("Failed to check and set");
    assert!(was_set);

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Getting the result should return None after expiration
    let retrieved: Option<TestResult> = store.get_result(&key).await.expect("Failed to get result");
    assert_eq!(retrieved, None);
}
