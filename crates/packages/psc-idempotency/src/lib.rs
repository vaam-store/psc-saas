use async_trait::async_trait;
use psc_error::Error;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};

#[async_trait]
pub trait IdempotencyStore {
    async fn check_and_set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        result: &T,
        ttl_seconds: usize,
    ) -> Result<bool, Error>;

    async fn get_result<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error>;
}

pub struct RedisIdempotencyStore {
    client: redis::Client,
}

impl RedisIdempotencyStore {
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
