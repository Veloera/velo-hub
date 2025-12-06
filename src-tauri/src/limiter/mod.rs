mod semaphore;
mod sliding_window;
mod token_bucket;

pub use semaphore::ConcurrencyGuard;

use crate::error::RateLimitError;
use crate::models::RateLimitConfig;
use semaphore::SemaphoreLimiter;
use sliding_window::SlidingWindowLimiter;
use std::sync::Arc;
use token_bucket::TokenBucketLimiter;

/// Main rate limiter that combines three limiting strategies:
/// 1. Token Bucket for RPM (Requests Per Minute)
/// 2. Sliding Window for token quotas (5h, week, month)
/// 3. Semaphore for concurrent session control
#[derive(Debug, Clone)]
pub struct RateLimiter {
    rpm_limiter: Arc<TokenBucketLimiter>,
    token_limiter: Arc<SlidingWindowLimiter>,
    concurrency_limiter: Arc<SemaphoreLimiter>,
    fail_open: bool,
}

impl RateLimiter {
    /// Create a new RateLimiter instance
    ///
    /// # Arguments
    /// * `fail_open` - If true, allows requests to proceed when rate limiting fails
    pub fn new(fail_open: bool) -> Self {
        Self {
            rpm_limiter: Arc::new(TokenBucketLimiter::new()),
            token_limiter: Arc::new(SlidingWindowLimiter::new()),
            concurrency_limiter: Arc::new(SemaphoreLimiter::new()),
            fail_open,
        }
    }

    /// Check if a request can proceed under all configured rate limits
    /// This does NOT consume any quotas - call consume() after successful request
    ///
    /// # Arguments
    /// * `provider_id` - The provider identifier
    /// * `estimated_tokens` - Estimated token count for the request
    /// * `config` - Rate limit configuration for the provider
    ///
    /// # Returns
    /// * `Ok(())` - Request is allowed
    /// * `Err(RateLimitError)` - Request should be rejected with specific error
    pub async fn check_limit(
        &self,
        provider_id: &str,
        estimated_tokens: u64,
        config: &RateLimitConfig,
    ) -> Result<(), RateLimitError> {
        // Check RPM limit
        if let Some(rpm) = config.rpm {
            match self.rpm_limiter.check_and_consume(provider_id, rpm).await {
                Ok(()) => {
                    tracing::debug!(
                        provider_id = %provider_id,
                        rpm_limit = rpm,
                        "RPM limit check passed"
                    );
                }
                Err(retry_after_secs) => {
                    if self.fail_open {
                        tracing::warn!(
                            provider_id = %provider_id,
                            rpm_limit = rpm,
                            retry_after_secs = retry_after_secs,
                            "RPM limit exceeded but fail_open is enabled"
                        );
                    } else {
                        tracing::warn!(
                            provider_id = %provider_id,
                            rpm_limit = rpm,
                            retry_after_secs = retry_after_secs,
                            "RPM limit exceeded, rejecting request"
                        );
                        return Err(RateLimitError::RpmExceeded {
                            provider_id: provider_id.to_string(),
                            retry_after_secs,
                        });
                    }
                }
            }
        }

        // Check token quota limits
        match self
            .token_limiter
            .check_limit(
                provider_id,
                estimated_tokens,
                config.tokens_per_5h,
                config.tokens_per_week,
                config.tokens_per_month,
            )
            .await
        {
            Ok(()) => {
                tracing::debug!(
                    provider_id = %provider_id,
                    estimated_tokens = estimated_tokens,
                    "Token quota check passed"
                );
            }
            Err(window) => {
                if self.fail_open {
                    tracing::warn!(
                        provider_id = %provider_id,
                        estimated_tokens = estimated_tokens,
                        window = %window,
                        "Token quota exceeded but fail_open is enabled"
                    );
                } else {
                    tracing::warn!(
                        provider_id = %provider_id,
                        estimated_tokens = estimated_tokens,
                        window = %window,
                        "Token quota exceeded, rejecting request"
                    );
                    return Err(RateLimitError::TokenQuotaExceeded {
                        provider_id: provider_id.to_string(),
                        window,
                    });
                }
            }
        }

        Ok(())
    }

    /// Try to acquire a concurrency slot for the provider
    /// Returns a guard that must be held for the duration of the session
    /// The slot is automatically released when the guard is dropped
    ///
    /// # Arguments
    /// * `provider_id` - The provider identifier
    /// * `config` - Rate limit configuration for the provider
    ///
    /// # Returns
    /// * `Ok(guard)` - Concurrency slot acquired
    /// * `Err(RateLimitError)` - Max concurrent sessions reached
    pub async fn try_acquire_concurrency(
        &self,
        provider_id: &str,
        config: &RateLimitConfig,
    ) -> Result<ConcurrencyGuard, RateLimitError> {
        let max_concurrent = config.max_concurrent_sessions.unwrap_or(0);

        match self
            .concurrency_limiter
            .try_acquire(provider_id, max_concurrent)
            .await
        {
            Ok(guard) => Ok(guard),
            Err(()) => {
                if self.fail_open {
                    tracing::warn!(
                        "Concurrency limit reached for provider {}, but fail_open is enabled",
                        provider_id
                    );
                    // In fail_open mode, we still try to acquire (will wait)
                    Ok(self
                        .concurrency_limiter
                        .acquire(provider_id, max_concurrent)
                        .await)
                } else {
                    Err(RateLimitError::ConcurrencyExceeded(provider_id.to_string()))
                }
            }
        }
    }

    /// Record actual token consumption after a successful request
    /// This updates the sliding window counters
    ///
    /// # Arguments
    /// * `provider_id` - The provider identifier
    /// * `actual_tokens` - Actual token count consumed
    pub async fn consume(
        &self,
        provider_id: &str,
        actual_tokens: u64,
    ) -> Result<(), RateLimitError> {
        self.token_limiter.consume(provider_id, actual_tokens).await;
        Ok(())
    }

    /// Get current rate limit status for a provider
    /// Returns (available_rpm_tokens, consumed_5h, consumed_week, consumed_month, available_concurrency)
    pub async fn get_status(
        &self,
        provider_id: &str,
    ) -> (Option<f64>, u64, u64, u64, Option<usize>) {
        let rpm_tokens = self.rpm_limiter.get_tokens(provider_id).await;

        let (consumed_5h, consumed_week, consumed_month) = self
            .token_limiter
            .get_consumption(provider_id)
            .await
            .unwrap_or((0, 0, 0));

        let available_concurrency = self.concurrency_limiter.available_permits(provider_id);

        (
            rpm_tokens,
            consumed_5h,
            consumed_week,
            consumed_month,
            available_concurrency,
        )
    }

    /// Reset all rate limits for a provider (useful for testing or manual reset)
    pub async fn reset(&self, provider_id: &str) {
        self.rpm_limiter.reset(provider_id).await;
        self.token_limiter.reset(provider_id).await;
        self.concurrency_limiter.reset(provider_id);
    }

    /// Cleanup old entries (should be called periodically, e.g., every hour)
    pub async fn cleanup(&self) {
        self.token_limiter.cleanup_old_entries().await;
    }

    /// Check if fail-open mode is enabled
    pub fn is_fail_open(&self) -> bool {
        self.fail_open
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> RateLimitConfig {
        RateLimitConfig {
            rpm: Some(60),
            tokens_per_5h: Some(100000),
            tokens_per_week: Some(1000000),
            tokens_per_month: Some(5000000),
            max_concurrent_sessions: Some(5),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limits() {
        let limiter = RateLimiter::new(false);
        let provider_id = "test-provider";
        let config = create_test_config();

        // Should allow request within all limits
        let result = limiter.check_limit(provider_id, 1000, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_rpm_exceeded() {
        let limiter = RateLimiter::new(false);
        let provider_id = "test-provider";
        let mut config = create_test_config();
        config.rpm = Some(2); // Very low limit

        // Consume all RPM
        let _ = limiter.check_limit(provider_id, 100, &config).await;
        let _ = limiter.check_limit(provider_id, 100, &config).await;

        // Should be rate limited
        let result = limiter.check_limit(provider_id, 100, &config).await;
        assert!(result.is_err());
        match result {
            Err(RateLimitError::RpmExceeded { .. }) => {}
            _ => panic!("Expected RpmExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_token_quota_exceeded() {
        let limiter = RateLimiter::new(false);
        let provider_id = "test-provider";
        let mut config = create_test_config();
        config.tokens_per_5h = Some(10000);

        // Consume tokens
        limiter.consume(provider_id, 9000).await.unwrap();

        // Should be blocked
        let result = limiter.check_limit(provider_id, 2000, &config).await;
        assert!(result.is_err());
        match result {
            Err(RateLimitError::TokenQuotaExceeded { window, .. }) => {
                assert_eq!(window, "5h");
            }
            _ => panic!("Expected TokenQuotaExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrency_control() {
        let limiter = RateLimiter::new(false);
        let provider_id = "test-provider";
        let mut config = create_test_config();
        config.max_concurrent_sessions = Some(2);

        // Acquire concurrency slots
        let guard1 = limiter.try_acquire_concurrency(provider_id, &config).await;
        assert!(guard1.is_ok());

        let guard2 = limiter.try_acquire_concurrency(provider_id, &config).await;
        assert!(guard2.is_ok());

        // Should be blocked
        let result = limiter.try_acquire_concurrency(provider_id, &config).await;
        assert!(result.is_err());
        match result {
            Err(RateLimitError::ConcurrencyExceeded(_)) => {}
            _ => panic!("Expected ConcurrencyExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_fail_open_mode() {
        let limiter = RateLimiter::new(true); // Enable fail-open
        let provider_id = "test-provider";
        let mut config = create_test_config();
        config.rpm = Some(1);

        // Consume RPM
        let _ = limiter.check_limit(provider_id, 100, &config).await;

        // Should still allow in fail-open mode
        let result = limiter.check_limit(provider_id, 100, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_consume_updates_quotas() {
        let limiter = RateLimiter::new(false);
        let provider_id = "test-provider";

        // Consume tokens
        limiter.consume(provider_id, 5000).await.unwrap();
        limiter.consume(provider_id, 3000).await.unwrap();

        // Check status
        let (_, consumed_5h, consumed_week, consumed_month, _) =
            limiter.get_status(provider_id).await;

        assert_eq!(consumed_5h, 8000);
        assert_eq!(consumed_week, 8000);
        assert_eq!(consumed_month, 8000);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(false);
        let provider_id = "test-provider";
        let mut config = create_test_config();
        config.rpm = Some(1);

        // Consume RPM
        let _ = limiter.check_limit(provider_id, 100, &config).await;

        // Should be blocked
        let result = limiter.check_limit(provider_id, 100, &config).await;
        assert!(result.is_err());

        // Reset
        limiter.reset(provider_id).await;

        // Should allow after reset
        let result = limiter.check_limit(provider_id, 100, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_no_limits_configured() {
        let limiter = RateLimiter::new(false);
        let provider_id = "test-provider";
        let config = RateLimitConfig::default(); // No limits

        // Should always allow
        for _ in 0..100 {
            let result = limiter.check_limit(provider_id, 10000, &config).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_get_status() {
        let limiter = RateLimiter::new(false);
        let provider_id = "test-provider";
        let config = create_test_config();

        // Initial status
        let (rpm_tokens, consumed_5h, consumed_week, consumed_month, concurrency) =
            limiter.get_status(provider_id).await;

        // RPM tokens should be available (or None if not initialized)
        // Consumed should be 0
        assert_eq!(consumed_5h, 0);
        assert_eq!(consumed_week, 0);
        assert_eq!(consumed_month, 0);

        // After consuming
        limiter.consume(provider_id, 1000).await.unwrap();
        let (_, consumed_5h, _, _, _) = limiter.get_status(provider_id).await;
        assert_eq!(consumed_5h, 1000);
    }
}
