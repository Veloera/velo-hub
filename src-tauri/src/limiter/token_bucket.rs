use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Token Bucket rate limiter for RPM (Requests Per Minute) control
/// Uses the token bucket algorithm to allow burst traffic while maintaining average rate
#[derive(Debug)]
pub struct TokenBucketLimiter {
    buckets: Arc<DashMap<String, Arc<Mutex<TokenBucket>>>>,
}

#[derive(Debug)]
struct TokenBucket {
    capacity: u32,
    tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucketLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
        }
    }

    /// Check if a request can proceed under the RPM limit
    /// Returns Ok(()) if allowed, Err with retry_after_secs if rate limited
    pub async fn check_and_consume(&self, provider_id: &str, rpm: u32) -> Result<(), u64> {
        if rpm == 0 {
            return Ok(()); // No limit configured
        }

        let bucket = self.get_or_create_bucket(provider_id, rpm).await;
        let mut bucket_guard = bucket.lock().await;

        // Refill tokens based on elapsed time
        bucket_guard.refill();

        if bucket_guard.tokens >= 1.0 {
            bucket_guard.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate retry after time
            let tokens_needed = 1.0 - bucket_guard.tokens;
            let retry_after_secs = (tokens_needed / bucket_guard.refill_rate).ceil() as u64;
            Err(retry_after_secs)
        }
    }

    /// Get current token count for a provider (for monitoring)
    pub async fn get_tokens(&self, provider_id: &str) -> Option<f64> {
        if let Some(bucket) = self.buckets.get(provider_id) {
            let mut bucket_guard = bucket.lock().await;
            bucket_guard.refill();
            Some(bucket_guard.tokens)
        } else {
            None
        }
    }

    /// Reset the bucket for a provider (useful for testing or manual reset)
    pub async fn reset(&self, provider_id: &str) {
        self.buckets.remove(provider_id);
    }

    async fn get_or_create_bucket(&self, provider_id: &str, rpm: u32) -> Arc<Mutex<TokenBucket>> {
        if let Some(bucket) = self.buckets.get(provider_id) {
            return bucket.clone();
        }

        let bucket = Arc::new(Mutex::new(TokenBucket::new(rpm)));
        self.buckets.insert(provider_id.to_string(), bucket.clone());
        bucket
    }
}

impl TokenBucket {
    fn new(rpm: u32) -> Self {
        let capacity = rpm;
        let refill_rate = rpm as f64 / 60.0; // tokens per second

        Self {
            capacity,
            tokens: capacity as f64, // Start with full bucket
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let elapsed_secs = elapsed.as_secs_f64();

        // Add tokens based on elapsed time
        let tokens_to_add = elapsed_secs * self.refill_rate;
        self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);
        self.last_refill = now;
    }
}

impl Default for TokenBucketLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_token_bucket_allows_requests_within_limit() {
        let limiter = TokenBucketLimiter::new();
        let provider_id = "test-provider";
        let rpm = 60; // 1 request per second

        // Should allow first request
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_ok());
    }

    #[tokio::test]
    async fn test_token_bucket_blocks_excessive_requests() {
        let limiter = TokenBucketLimiter::new();
        let provider_id = "test-provider";
        let rpm = 2; // Very low limit for testing

        // Consume all tokens
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_ok());
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_ok());

        // Should be rate limited
        let result = limiter.check_and_consume(provider_id, rpm).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_bucket_refills_over_time() {
        let limiter = TokenBucketLimiter::new();
        let provider_id = "test-provider";
        let rpm = 60; // 1 request per second

        // Consume all initial tokens
        for _ in 0..60 {
            let _ = limiter.check_and_consume(provider_id, rpm).await;
        }

        // Should be rate limited
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_err());

        // Wait for refill (1 second = 1 token)
        sleep(Duration::from_millis(1100)).await;

        // Should allow request after refill
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_ok());
    }

    #[tokio::test]
    async fn test_token_bucket_no_limit() {
        let limiter = TokenBucketLimiter::new();
        let provider_id = "test-provider";
        let rpm = 0; // No limit

        // Should always allow requests
        for _ in 0..100 {
            assert!(limiter.check_and_consume(provider_id, rpm).await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_token_bucket_reset() {
        let limiter = TokenBucketLimiter::new();
        let provider_id = "test-provider";
        let rpm = 2;

        // Consume all tokens
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_ok());
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_ok());
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_err());

        // Reset
        limiter.reset(provider_id).await;

        // Should allow requests again
        assert!(limiter.check_and_consume(provider_id, rpm).await.is_ok());
    }
}
