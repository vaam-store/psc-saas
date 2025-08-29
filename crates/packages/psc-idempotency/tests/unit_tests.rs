use psc_idempotency::RedisIdempotencyStore;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestResult {
    value: String,
    count: u32,
}

#[test]
fn test_redis_idempotency_store_new() {
    // Test that we can create a RedisIdempotencyStore instance
    // This doesn't test the actual connection, just the creation
    let result = RedisIdempotencyStore::new("redis://127.0.0.1:6379");
    // We expect this to succeed as it only creates a client, doesn't connect
    assert!(result.is_ok());
}

#[test]
fn test_redis_idempotency_store_invalid_url() {
    // Test that an invalid URL results in an error
    let result = RedisIdempotencyStore::new("invalid-url");
    assert!(result.is_err());
}
