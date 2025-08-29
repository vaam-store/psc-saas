use psc_retry::*;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_retry_policy_default() {
    let policy = RetryPolicy::default();
    assert_eq!(policy.max_retries, 3);
    assert_eq!(policy.initial_backoff, Duration::from_millis(100));
    assert_eq!(policy.max_backoff, Duration::from_secs(10));
    assert_eq!(policy.jitter, true);
}

#[tokio::test]
async fn test_retry_policy_builder() {
    let policy = RetryPolicy::new()
        .with_max_retries(5)
        .with_initial_backoff(Duration::from_millis(200))
        .with_max_backoff(Duration::from_secs(5))
        .with_jitter(false);

    assert_eq!(policy.max_retries, 5);
    assert_eq!(policy.initial_backoff, Duration::from_millis(200));
    assert_eq!(policy.max_backoff, Duration::from_secs(5));
    assert_eq!(policy.jitter, false);
}

#[tokio::test]
async fn test_successful_operation_no_retries() {
    let policy = RetryPolicy::new();

    let result = do_with_retry(&policy, None, || async {
        Ok::<String, String>("success".to_string())
    })
    .await;

    assert_eq!(result, Ok("success".to_string()));
}

#[tokio::test]
async fn test_retry_until_success() {
    let policy = RetryPolicy::new().with_max_retries(3);
    let mut call_count = 0;

    let result = do_with_retry(&policy, None, || {
        let count = call_count;
        call_count += 1;
        async move {
            if count < 2 {
                Err::<String, String>("temporary error".to_string())
            } else {
                Ok::<String, String>("success".to_string())
            }
        }
    })
    .await;

    assert_eq!(result, Ok("success".to_string()));
    assert_eq!(call_count, 3);
}

#[tokio::test]
async fn test_retry_exhausted() {
    let policy = RetryPolicy::new().with_max_retries(2);
    let mut call_count = 0;

    let result = do_with_retry(&policy, None, || {
        call_count += 1;
        async move { Err::<String, String>("permanent error".to_string()) }
    })
    .await;

    assert_eq!(
        result,
        Err(RetryError::AttemptsExhausted("permanent error".to_string()))
    );
    assert_eq!(call_count, 3); // Initial attempt + 2 retries
}

#[tokio::test]
async fn test_circuit_breaker_default() {
    let cb = CircuitBreaker::default();
    assert!(
        timeout(Duration::from_millis(100), cb.can_execute())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_circuit_breaker_open_and_close() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        timeout: Duration::from_millis(100),
        success_threshold: 2,
    };
    let cb = CircuitBreaker::new(config);

    // Record failures to open circuit
    cb.record_failure().await;
    assert!(
        timeout(Duration::from_millis(100), cb.can_execute())
            .await
            .unwrap()
    );
    cb.record_failure().await;
    assert!(
        !timeout(Duration::from_millis(100), cb.can_execute())
            .await
            .unwrap()
    );

    // Wait for timeout to move to half-open
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        timeout(Duration::from_millis(100), cb.can_execute())
            .await
            .unwrap()
    );

    // Record successes to close circuit
    cb.record_success().await;
    assert!(
        timeout(Duration::from_millis(100), cb.can_execute())
            .await
            .unwrap()
    );
    cb.record_success().await;
    assert!(
        timeout(Duration::from_millis(100), cb.can_execute())
            .await
            .unwrap()
    );

    // Circuit should now be closed
    assert_eq!(*cb.state.read().await, CircuitState::Closed);
}
