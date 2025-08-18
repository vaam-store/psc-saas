#![deny(clippy::all)]
#![forbid(unsafe_code)]

//! Shared Provider trait and a MockProvider implementation for testing.
//!
//! This crate defines the unified Provider interface used by services to interact
//! with mobile-money providers (MTN, Orange, Camtel). The mock implementation
//! allows deterministic testing of success, error and latency scenarios.
//!
//! Method signatures use placeholder request/response types to avoid coupling with generated Protobufs.

use async_trait::async_trait;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;

/// Context alias for passing request-scoped metadata.
pub type Ctx = ();

/// Error type returned by provider implementations.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("network error: {0}")]
    Network(String),
    #[error("provider returned error: {0}")]
    Provider(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("timeout")]
    Timeout,
}

/// Example request/response placeholders — swap with Protobuf types.
#[derive(Debug, Clone)]
pub struct DepositRequest {
    pub msisdn: String,
    pub amount_minor: i64,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct DepositResponse {
    pub provider_reference: Option<String>,
    pub status: ProviderResultStatus,
}

#[derive(Debug, Clone)]
pub enum ProviderResultStatus {
    Pending,
    Success,
    Failed,
}

/// Provider trait that abstracts provider operations.
#[async_trait]
pub trait Provider: Send + Sync {
    async fn deposit(&self, ctx: &Ctx, req: DepositRequest) -> Result<DepositResponse, ProviderError>;
    async fn withdraw(&self, ctx: &Ctx, req: DepositRequest) -> Result<DepositResponse, ProviderError>;
    async fn refund(&self, ctx: &Ctx, req: DepositRequest) -> Result<DepositResponse, ProviderError>;
    async fn query(&self, ctx: &Ctx, reference: &str) -> Result<DepositResponse, ProviderError>;
    async fn verify_webhook(&self, ctx: &Ctx, payload: &[u8], signature_header: Option<&str>) -> Result<bool, ProviderError>;
}

mod mock {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use std::time::Instant;

    /// Behavior modes for the MockProvider.
    #[derive(Debug, Clone)]
    pub enum MockBehavior {
        AlwaysSucceed,
        AlwaysFail(String),
        FailOnceThenSucceed,
        Delay(Duration, Box<MockBehavior>),
    }

    /// Internal state for behaviors that need to record invocations.
    #[derive(Debug, Default)]
    struct MockState {
        pub fail_once_consumed: bool,
        pub last_invocation: Option<Instant>,
    }

    /// A configurable mock provider for tests and local development.
    #[derive(Clone)]
    pub struct MockProvider {
        behavior: MockBehavior,
        state: Arc<Mutex<MockState>>,
    }

    impl MockProvider {
        pub fn new(behavior: MockBehavior) -> Self {
            Self {
                behavior,
                state: Arc::new(Mutex::new(MockState::default())),
            }
        }

        async fn apply_behavior(&self) -> Result<(), ProviderError> {
            match &self.behavior {
                MockBehavior::AlwaysSucceed => Ok(()),
                MockBehavior::AlwaysFail(msg) => Err(ProviderError::Provider(msg.clone())),
                MockBehavior::FailOnceThenSucceed => {
                    let mut s = self.state.lock().await;
                    if !s.fail_once_consumed {
                        s.fail_once_consumed = true;
                        Err(ProviderError::Provider("transient-failure".into()))
                    } else {
                        Ok(())
                    }
                }
                MockBehavior::Delay(dur, inner) => {
                    sleep(*dur).await;
                    // After delay, evaluate the inner behavior non-recursively.
                    match &**inner {
                        MockBehavior::AlwaysSucceed => Ok(()),
                        MockBehavior::AlwaysFail(msg) => Err(ProviderError::Provider(msg.clone())),
                        MockBehavior::FailOnceThenSucceed => {
                            let mut s = self.state.lock().await;
                            if !s.fail_once_consumed {
                                s.fail_once_consumed = true;
                                Err(ProviderError::Provider("transient-failure".into()))
                            } else {
                                Ok(())
                            }
                        }
                        MockBehavior::Delay(inner_dur, inner_inner) => {
                            sleep(*inner_dur).await;
                            match &**inner_inner {
                                MockBehavior::AlwaysSucceed => Ok(()),
                                MockBehavior::AlwaysFail(msg) => Err(ProviderError::Provider(msg.clone())),
                                MockBehavior::FailOnceThenSucceed => {
                                    let mut s = self.state.lock().await;
                                    if !s.fail_once_consumed {
                                        s.fail_once_consumed = true;
                                        Err(ProviderError::Provider("transient-failure".into()))
                                    } else {
                                        Ok(())
                                    }
                                }
                                _ => Ok(()),
                            }
                        }
                    }
                }
            }
        }
    }

    #[async_trait]
    impl super::Provider for MockProvider {
        async fn deposit(&self, _ctx: &Ctx, req: DepositRequest) -> Result<DepositResponse, ProviderError> {
            let mut s = self.state.lock().await;
            s.last_invocation = Some(Instant::now());
            drop(s);
            self.apply_behavior().await?;
            Ok(DepositResponse {
                provider_reference: Some(format!("mock-ref-{}", req.msisdn)),
                status: ProviderResultStatus::Success,
            })
        }

        async fn withdraw(&self, _ctx: &Ctx, req: DepositRequest) -> Result<DepositResponse, ProviderError> {
            let mut s = self.state.lock().await;
            s.last_invocation = Some(Instant::now());
            drop(s);
            self.apply_behavior().await?;
            Ok(DepositResponse {
                provider_reference: Some(format!("mock-withdraw-{}", req.msisdn)),
                status: ProviderResultStatus::Success,
            })
        }

        async fn refund(&self, _ctx: &Ctx, req: DepositRequest) -> Result<DepositResponse, ProviderError> {
            let mut s = self.state.lock().await;
            s.last_invocation = Some(Instant::now());
            drop(s);
            self.apply_behavior().await?;
            Ok(DepositResponse {
                provider_reference: Some(format!("mock-refund-{}", req.msisdn)),
                status: ProviderResultStatus::Success,
            })
        }

        async fn query(&self, _ctx: &Ctx, reference: &str) -> Result<DepositResponse, ProviderError> {
            let mut s = self.state.lock().await;
            s.last_invocation = Some(Instant::now());
            drop(s);
            self.apply_behavior().await?;
            Ok(DepositResponse {
                provider_reference: Some(reference.to_string()),
                status: ProviderResultStatus::Success,
            })
        }

        async fn verify_webhook(&self, _ctx: &Ctx, _payload: &[u8], _signature_header: Option<&str>) -> Result<bool, ProviderError> {
            let mut s = self.state.lock().await;
            s.last_invocation = Some(Instant::now());
            drop(s);
            self.apply_behavior().await?;
            Ok(true)
        }
    }

}

pub use mock::{MockProvider, MockBehavior};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn mock_always_succeeds() {
        let provider = MockProvider::new(MockBehavior::AlwaysSucceed);
        let req = DepositRequest { msisdn: "+237670000001".into(), amount_minor: 1000, metadata: None };
        let res = provider.deposit(&(), req).await;
        assert!(res.is_ok());
        let res = res.unwrap();
        match res.status {
            ProviderResultStatus::Success => {}
            _ => panic!("expected success"),
        }
    }

    #[tokio::test]
    async fn mock_always_fails() {
        let provider = MockProvider::new(MockBehavior::AlwaysFail("OUT_OF_FUNDS".into()));
        let req = DepositRequest { msisdn: "+237670000002".into(), amount_minor: 1000, metadata: None };
        let res = provider.deposit(&(), req).await;
        assert!(res.is_err());
        match res.err().unwrap() {
            ProviderError::Provider(msg) => assert!(msg.contains("OUT_OF_FUNDS")),
            _ => panic!("unexpected error kind"),
        }
    }

    #[tokio::test]
    async fn mock_fail_once_then_succeed() {
        let provider = MockProvider::new(MockBehavior::FailOnceThenSucceed);
        let req = DepositRequest { msisdn: "+237670000003".into(), amount_minor: 1000, metadata: None };
        let first = provider.deposit(&(), req.clone()).await;
        assert!(first.is_err());
        let second = provider.deposit(&(), req).await;
        assert!(second.is_ok());
    }

    #[tokio::test]
    async fn mock_delay_behavior() {
        let provider = MockProvider::new(MockBehavior::Delay(Duration::from_millis(50), Box::new(MockBehavior::AlwaysSucceed)));
        let req = DepositRequest { msisdn: "+237670000004".into(), amount_minor: 1000, metadata: None };
        let start = std::time::Instant::now();
        let res = provider.deposit(&(), req).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(50));
        assert!(res.is_ok());
    }
}