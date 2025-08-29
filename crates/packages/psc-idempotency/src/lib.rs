//! Idempotency mechanism implementation using Redis as the backend store.
//!
//! This crate provides an implementation of an idempotency store that uses Redis
//! to store the results of operations, ensuring that repeated requests with the
//! same idempotency key return the same result.
//!
//! # Example
//!
//! ```no_run
//! use psc_idempotency::{IdempotencyStore, RedisIdempotencyStore};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug)]
//! struct PaymentResult {
//!     payment_id: String,
//!     amount: u64,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let store = RedisIdempotencyStore::new("redis://127.0.0.1:6379")?;
//!
//!     let result = PaymentResult {
//!         payment_id: "pay_123".to_string(),
//!         amount: 1000,
//!     };
//!
//!     // Try to set the result for an idempotency key
//!     let was_set = store.check_and_set("payment_123", &result, 3600).await?;
//!
//!     if was_set {
//!         println!("Result was stored for the first time");
//!     } else {
//!         println!("Result was already stored, retrieving existing result");
//!         let existing_result: Option<PaymentResult> = store.get_result("payment_123").await?;
//!         println!("Existing result: {:?}", existing_result);
//!     }
//!
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use psc_error::Error;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};

/// Trait for idempotency store implementations.
///
/// This trait defines the interface for storing and retrieving results
/// associated with idempotency keys.
#[async_trait]
pub trait IdempotencyStore {
    /// Store a result for an idempotency key if it doesn't already exist.
    ///
    /// Returns `true` if the result was stored, `false` if a result was
    /// already stored for the key.
    ///
    /// # Parameters
    ///
    /// * `key` - The idempotency key
    /// * `result` - The result to store
    /// * `ttl_seconds` - Time-to-live for the stored result in seconds
    async fn check_and_set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        result: &T,
        ttl_seconds: usize,
    ) -> Result<bool, Error>;

    /// Retrieve a result for an idempotency key.
    ///
    /// Returns `Some(result)` if a result was stored for the key,
    /// `None` if no result was found.
    ///
    /// # Parameters
    ///
    /// * `key` - The idempotency key
    async fn get_result<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error>;
}

/// Redis-based implementation of the idempotency store.
///
/// This implementation uses Redis to store results associated with
/// idempotency keys. Results are stored with a TTL (time-to-live)
/// to prevent indefinite storage.
pub struct RedisIdempotencyStore {
    client: redis::Client,
}

impl RedisIdempotencyStore {
    /// Create a new Redis idempotency store.
    ///
    /// # Parameters
    ///
    /// * `redis_url` - The URL of the Redis server
    ///
    /// # Returns
    ///
    /// A new `RedisIdempotencyStore` instance
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client cannot be created
    pub fn new(redis_url: &str) -> Result<Self, Error> {
        let client = redis::Client::open(redis_url).map_err(|e| Error::Internal(e.to_string()))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl IdempotencyStore for RedisIdempotencyStore {
    async fn check_and_set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        result: &T,
        ttl_seconds: usize,
    ) -> Result<bool, Error> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;

        let result_json =
            serde_json::to_string(result).map_err(|e| Error::Internal(e.to_string()))?;

        let was_set: bool = redis::cmd("SET")
            .arg(key)
            .arg(&result_json)
            .arg("NX")
            .arg("EX")
            .arg(ttl_seconds)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(was_set)
    }

    async fn get_result<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;

        let result_json: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;

        match result_json {
            Some(json) => {
                let result =
                    serde_json::from_str(&json).map_err(|e| Error::Internal(e.to_string()))?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }
}
