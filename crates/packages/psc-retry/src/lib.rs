//! Robust retry mechanisms with exponential backoff and circuit breaker pattern for external API calls.
//!
//! This crate provides utilities for implementing retry logic with exponential backoff and jitter,
//! as well as a circuit breaker pattern to prevent cascading failures when calling external services.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use thiserror::Error;
use tokio::time::{Instant, sleep};
use tracing::{debug, warn};

/// Errors that can occur during retry operations
#[derive(Error, Debug, PartialEq)]
pub enum RetryError<E> {
    /// The operation failed after all retry attempts were exhausted
    #[error("Retry attempts exhausted: {0}")]
    AttemptsExhausted(E),

    /// The circuit breaker is open, preventing further attempts
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
}

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Whether to use jitter in backoff calculations
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retry attempts
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the initial backoff duration
    pub fn with_initial_backoff(mut self, initial_backoff: Duration) -> Self {
        self.initial_backoff = initial_backoff;
        self
    }

    /// Set the maximum backoff duration
    pub fn with_max_backoff(mut self, max_backoff: Duration) -> Self {
        self.max_backoff = max_backoff;
        self
    }

    /// Set whether to use jitter in backoff calculations
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Calculate the backoff duration for a given attempt
    fn calculate_backoff(&self, attempt: usize) -> Duration {
        // Exponential backoff: initial_backoff * 2^attempt
        let exponential_backoff = self.initial_backoff.mul_f64(2f64.powi(attempt as i32));

        // Cap at max_backoff
        let backoff = std::cmp::min(exponential_backoff, self.max_backoff);

        // Add jitter if enabled
        if self.jitter {
            // Add random jitter of up to 25% of the backoff time
            let jitter_amount = backoff.mul_f32(0.25);
            let jitter = rand::random::<u64>() % (jitter_amount.as_millis() as u64 + 1);
            backoff + Duration::from_millis(jitter)
        } else {
            backoff
        }
    }
}

/// State of the circuit breaker
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    /// Circuit is closed, allowing requests
    Closed,
    /// Circuit is open, rejecting requests
    Open,
    /// Circuit is half-open, allowing limited requests to test if service is recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: usize,
    /// Timeout before attempting to close the circuit again
    pub timeout: Duration,
    /// Number of successful requests needed to close the circuit in half-open state
    pub success_threshold: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            success_threshold: 3,
        }
    }
}

/// Circuit breaker implementation
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    pub state: Arc<tokio::sync::RwLock<CircuitState>>,
    failure_count: Arc<AtomicUsize>,
    success_count: Arc<AtomicUsize>,
    last_failure_time: Arc<tokio::sync::RwLock<Option<Instant>>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(tokio::sync::RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            success_count: Arc::new(AtomicUsize::new(0)),
            last_failure_time: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Create a new circuit breaker with default configuration
    pub fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Check if the circuit breaker allows requests
    pub async fn can_execute(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = self.last_failure_time.read().await;
                if let Some(last_failure_time) = *last_failure {
                    if last_failure_time.elapsed() >= self.config.timeout {
                        // Move to half-open state
                        *self.state.write().await = CircuitState::HalfOpen;
                        self.success_count.store(0, Ordering::Relaxed);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    /// Record a successful request
    pub async fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);

        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => {
                // Already closed, nothing to do
            }
            CircuitState::HalfOpen => {
                // Increment success count
                let new_success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if new_success_count >= self.config.success_threshold {
                    // Close the circuit
                    *self.state.write().await = CircuitState::Closed;
                    self.success_count.store(0, Ordering::Relaxed);
                    debug!("Circuit breaker closed after successful requests");
                }
            }
            CircuitState::Open => {
                // Should not happen if can_execute is checked first
                warn!("Recorded success while circuit breaker is open");
            }
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let new_failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => {
                if new_failure_count >= self.config.failure_threshold {
                    // Open the circuit
                    *self.state.write().await = CircuitState::Open;
                    *self.last_failure_time.write().await = Some(Instant::now());
                    warn!(
                        "Circuit breaker opened after {} failures",
                        new_failure_count
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Failed in half-open state, go back to open
                *self.state.write().await = CircuitState::Open;
                *self.last_failure_time.write().await = Some(Instant::now());
                self.success_count.store(0, Ordering::Relaxed);
                warn!("Circuit breaker reopened after failure in half-open state");
            }
            CircuitState::Open => {
                // Already open, update last failure time
                *self.last_failure_time.write().await = Some(Instant::now());
            }
        }
    }
}

/// Execute an operation with retry logic and circuit breaker
///
/// # Arguments
/// * `policy` - The retry policy to use
/// * `circuit_breaker` - The circuit breaker to use (optional)
/// * `operation` - The operation to execute, which should return a Result
///
/// # Returns
/// * `Ok(T)` if the operation succeeds
/// * `Err(RetryError<E>)` if the operation fails after all retries or if the circuit breaker is open
pub async fn do_with_retry<T, E, F, Fut>(
    policy: &RetryPolicy,
    circuit_breaker: Option<&CircuitBreaker>,
    operation: F,
) -> Result<T, RetryError<E>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    // Check circuit breaker if provided
    if let Some(cb) = circuit_breaker {
        if !cb.can_execute().await {
            return Err(RetryError::CircuitBreakerOpen);
        }
    }

    let mut attempt = 0;
    let mut op = operation;
    loop {
        match op().await {
            Ok(result) => {
                // Record success in circuit breaker if provided
                if let Some(cb) = circuit_breaker {
                    cb.record_success().await;
                }
                return Ok(result);
            }
            Err(error) => {
                // Record failure in circuit breaker if provided
                if let Some(cb) = circuit_breaker {
                    cb.record_failure().await;

                    // Check if circuit breaker is now open
                    if !cb.can_execute().await {
                        return Err(RetryError::CircuitBreakerOpen);
                    }
                }

                attempt += 1;
                if attempt > policy.max_retries {
                    return Err(RetryError::AttemptsExhausted(error));
                }

                // Calculate backoff and sleep
                let backoff = policy.calculate_backoff(attempt);
                debug!("Attempt {} failed, retrying in {:?}", attempt, backoff);
                sleep(backoff).await;
            }
        }
    }
}
