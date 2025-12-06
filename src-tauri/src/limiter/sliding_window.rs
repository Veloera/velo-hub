use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Sliding Window rate limiter for token quota control
/// Tracks token consumption across multiple time windows (5h, week, month)
#[derive(Debug)]
pub struct SlidingWindowLimiter {
    windows: Arc<DashMap<String, Arc<RwLock<ProviderWindows>>>>,
}

#[derive(Debug)]
struct ProviderWindows {
    window_5h: TimeWindow,
    window_week: TimeWindow,
    window_month: TimeWindow,
}

#[derive(Debug)]
struct TimeWindow {
    duration_secs: u64,
    entries: Vec<TokenEntry>,
}

#[derive(Debug, Clone)]
struct TokenEntry {
    timestamp: u64,
    tokens: u64,
}

impl SlidingWindowLimiter {
    pub fn new() -> Self {
        Self {
            windows: Arc::new(DashMap::new()),
        }
    }

    /// Check if token consumption is within limits for all configured windows
    /// Returns Ok(()) if allowed, Err with window name if any limit exceeded
    pub async fn check_limit(
        &self,
        provider_id: &str,
        tokens: u64,
        limit_5h: Option<u64>,
        limit_week: Option<u64>,
        limit_month: Option<u64>,
    ) -> Result<(), String> {
        if limit_5h.is_none() && limit_week.is_none() && limit_month.is_none() {
            return Ok(()); // No limits configured
        }

        let windows = self.get_or_create_windows(provider_id).await;
        let windows_guard = windows.read().await;

        let now = Self::current_timestamp();

        // Check 5-hour window
        if let Some(limit) = limit_5h {
            let consumed = windows_guard.window_5h.get_consumed_tokens(now);
            if consumed + tokens > limit {
                return Err("5h".to_string());
            }
        }

        // Check weekly window
        if let Some(limit) = limit_week {
            let consumed = windows_guard.window_week.get_consumed_tokens(now);
            if consumed + tokens > limit {
                return Err("week".to_string());
            }
        }

        // Check monthly window
        if let Some(limit) = limit_month {
            let consumed = windows_guard.window_month.get_consumed_tokens(now);
            if consumed + tokens > limit {
                return Err("month".to_string());
            }
        }

        Ok(())
    }

    /// Record token consumption after a successful request
    pub async fn consume(&self, provider_id: &str, tokens: u64) {
        let windows = self.get_or_create_windows(provider_id).await;
        let mut windows_guard = windows.write().await;

        let now = Self::current_timestamp();
        let entry = TokenEntry {
            timestamp: now,
            tokens,
        };

        windows_guard.window_5h.add_entry(entry.clone(), now);
        windows_guard.window_week.add_entry(entry.clone(), now);
        windows_guard.window_month.add_entry(entry, now);
    }

    /// Get current token consumption for a provider across all windows
    pub async fn get_consumption(&self, provider_id: &str) -> Option<(u64, u64, u64)> {
        if let Some(windows) = self.windows.get(provider_id) {
            let windows_guard = windows.read().await;
            let now = Self::current_timestamp();

            Some((
                windows_guard.window_5h.get_consumed_tokens(now),
                windows_guard.window_week.get_consumed_tokens(now),
                windows_guard.window_month.get_consumed_tokens(now),
            ))
        } else {
            None
        }
    }

    /// Reset all windows for a provider (useful for testing)
    pub async fn reset(&self, provider_id: &str) {
        self.windows.remove(provider_id);
    }

    /// Cleanup old entries across all providers (should be called periodically)
    pub async fn cleanup_old_entries(&self) {
        let now = Self::current_timestamp();

        for entry in self.windows.iter() {
            let mut windows_guard = entry.value().write().await;
            windows_guard.window_5h.cleanup(now);
            windows_guard.window_week.cleanup(now);
            windows_guard.window_month.cleanup(now);
        }
    }

    async fn get_or_create_windows(&self, provider_id: &str) -> Arc<RwLock<ProviderWindows>> {
        if let Some(windows) = self.windows.get(provider_id) {
            return windows.clone();
        }

        let windows = Arc::new(RwLock::new(ProviderWindows::new()));
        self.windows
            .insert(provider_id.to_string(), windows.clone());
        windows
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

impl ProviderWindows {
    fn new() -> Self {
        Self {
            window_5h: TimeWindow::new(5 * 60 * 60),          // 5 hours
            window_week: TimeWindow::new(7 * 24 * 60 * 60),   // 7 days
            window_month: TimeWindow::new(30 * 24 * 60 * 60), // 30 days
        }
    }
}

impl TimeWindow {
    fn new(duration_secs: u64) -> Self {
        Self {
            duration_secs,
            entries: Vec::new(),
        }
    }

    fn add_entry(&mut self, entry: TokenEntry, now: u64) {
        self.entries.push(entry);
        self.cleanup(now);
    }

    fn get_consumed_tokens(&self, now: u64) -> u64 {
        let cutoff = now.saturating_sub(self.duration_secs);

        self.entries
            .iter()
            .filter(|e| e.timestamp >= cutoff)
            .map(|e| e.tokens)
            .sum()
    }

    fn cleanup(&mut self, now: u64) {
        let cutoff = now.saturating_sub(self.duration_secs);
        self.entries.retain(|e| e.timestamp >= cutoff);
    }
}

impl Default for SlidingWindowLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_sliding_window_allows_within_limit() {
        let limiter = SlidingWindowLimiter::new();
        let provider_id = "test-provider";

        // Check with 5h limit
        let result = limiter
            .check_limit(provider_id, 1000, Some(10000), None, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sliding_window_blocks_over_limit() {
        let limiter = SlidingWindowLimiter::new();
        let provider_id = "test-provider";

        // Consume tokens
        limiter.consume(provider_id, 8000).await;

        // Try to consume more than limit
        let result = limiter
            .check_limit(provider_id, 3000, Some(10000), None, None)
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "5h");
    }

    #[tokio::test]
    async fn test_sliding_window_multiple_windows() {
        let limiter = SlidingWindowLimiter::new();
        let provider_id = "test-provider";

        // Consume tokens
        limiter.consume(provider_id, 5000).await;

        // Check all windows
        let result = limiter
            .check_limit(
                provider_id,
                3000,
                Some(10000), // 5h: 8000/10000 - OK
                Some(7000),  // week: 8000/7000 - FAIL
                Some(20000), // month: 8000/20000 - OK
            )
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "week");
    }

    #[tokio::test]
    async fn test_sliding_window_consumption_tracking() {
        let limiter = SlidingWindowLimiter::new();
        let provider_id = "test-provider";

        // Consume tokens
        limiter.consume(provider_id, 1000).await;
        limiter.consume(provider_id, 2000).await;
        limiter.consume(provider_id, 3000).await;

        // Check consumption
        let consumption = limiter.get_consumption(provider_id).await;
        assert!(consumption.is_some());
        let (consumed_5h, consumed_week, consumed_month) = consumption.unwrap();
        assert_eq!(consumed_5h, 6000);
        assert_eq!(consumed_week, 6000);
        assert_eq!(consumed_month, 6000);
    }

    #[tokio::test]
    async fn test_sliding_window_no_limits() {
        let limiter = SlidingWindowLimiter::new();
        let provider_id = "test-provider";

        // Should allow any amount when no limits configured
        let result = limiter
            .check_limit(provider_id, 1000000, None, None, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sliding_window_reset() {
        let limiter = SlidingWindowLimiter::new();
        let provider_id = "test-provider";

        // Consume tokens
        limiter.consume(provider_id, 9000).await;

        // Should be near limit
        let result = limiter
            .check_limit(provider_id, 2000, Some(10000), None, None)
            .await;
        assert!(result.is_err());

        // Reset
        limiter.reset(provider_id).await;

        // Should allow after reset
        let result = limiter
            .check_limit(provider_id, 2000, Some(10000), None, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sliding_window_cleanup() {
        let limiter = SlidingWindowLimiter::new();
        let provider_id = "test-provider";

        // Add entries
        limiter.consume(provider_id, 1000).await;
        limiter.consume(provider_id, 2000).await;

        // Cleanup (in real scenario, old entries would be removed)
        limiter.cleanup_old_entries().await;

        // Consumption should still be tracked (entries are recent)
        let consumption = limiter.get_consumption(provider_id).await;
        assert!(consumption.is_some());
        let (consumed_5h, _, _) = consumption.unwrap();
        assert_eq!(consumed_5h, 3000);
    }
}
