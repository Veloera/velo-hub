use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};

/// Semaphore-based concurrency limiter for controlling concurrent sessions per provider
/// Uses tokio Semaphore to limit the number of concurrent active sessions
#[derive(Debug)]
pub struct SemaphoreLimiter {
    semaphores: Arc<DashMap<String, Arc<Semaphore>>>,
}

/// RAII guard that releases the semaphore permit when dropped
pub struct ConcurrencyGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
    provider_id: String,
}

impl SemaphoreLimiter {
    pub fn new() -> Self {
        Self {
            semaphores: Arc::new(DashMap::new()),
        }
    }

    /// Try to acquire a concurrency slot for the provider
    /// Returns Ok(guard) if successful, Err if limit reached
    /// The guard must be held for the duration of the session
    pub async fn try_acquire(
        &self,
        provider_id: &str,
        max_concurrent: u32,
    ) -> Result<ConcurrencyGuard, ()> {
        if max_concurrent == 0 {
            // No limit configured, create a dummy guard
            // We use a very large semaphore that will never block in practice
            // Note: Tokio semaphore has a max of 2^31 - 1 permits
            let semaphore = self.get_or_create_semaphore(provider_id, 1_000_000);
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            return Ok(ConcurrencyGuard {
                _permit: permit,
                provider_id: provider_id.to_string(),
            });
        }

        let semaphore = self.get_or_create_semaphore(provider_id, max_concurrent as usize);

        match semaphore.clone().try_acquire_owned() {
            Ok(permit) => Ok(ConcurrencyGuard {
                _permit: permit,
                provider_id: provider_id.to_string(),
            }),
            Err(_) => Err(()),
        }
    }

    /// Acquire a concurrency slot, waiting if necessary
    /// This will block until a slot becomes available
    pub async fn acquire(&self, provider_id: &str, max_concurrent: u32) -> ConcurrencyGuard {
        let max = if max_concurrent == 0 {
            1_000_000 // Very large limit for "no limit" case
        } else {
            max_concurrent as usize
        };

        let semaphore = self.get_or_create_semaphore(provider_id, max);
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        ConcurrencyGuard {
            _permit: permit,
            provider_id: provider_id.to_string(),
        }
    }

    /// Get the number of available permits for a provider
    pub fn available_permits(&self, provider_id: &str) -> Option<usize> {
        self.semaphores
            .get(provider_id)
            .map(|sem| sem.available_permits())
    }

    /// Reset the semaphore for a provider (useful for reconfiguration)
    pub fn reset(&self, provider_id: &str) {
        self.semaphores.remove(provider_id);
    }

    fn get_or_create_semaphore(&self, provider_id: &str, max_concurrent: usize) -> Arc<Semaphore> {
        if let Some(semaphore) = self.semaphores.get(provider_id) {
            return semaphore.clone();
        }

        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        self.semaphores
            .insert(provider_id.to_string(), semaphore.clone());
        semaphore
    }
}

impl Default for SemaphoreLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl ConcurrencyGuard {
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_semaphore_allows_within_limit() {
        let limiter = SemaphoreLimiter::new();
        let provider_id = "test-provider";
        let max_concurrent = 3;

        // Should allow up to max_concurrent
        let guard1 = limiter.try_acquire(provider_id, max_concurrent).await;
        assert!(guard1.is_ok());

        let guard2 = limiter.try_acquire(provider_id, max_concurrent).await;
        assert!(guard2.is_ok());

        let guard3 = limiter.try_acquire(provider_id, max_concurrent).await;
        assert!(guard3.is_ok());

        // Should block the 4th
        let guard4 = limiter.try_acquire(provider_id, max_concurrent).await;
        assert!(guard4.is_err());
    }

    #[tokio::test]
    async fn test_semaphore_releases_on_drop() {
        let limiter = SemaphoreLimiter::new();
        let provider_id = "test-provider";
        let max_concurrent = 2;

        // Acquire all permits
        let guard1 = limiter
            .try_acquire(provider_id, max_concurrent)
            .await
            .unwrap();
        let guard2 = limiter
            .try_acquire(provider_id, max_concurrent)
            .await
            .unwrap();

        // Should be blocked
        assert!(limiter
            .try_acquire(provider_id, max_concurrent)
            .await
            .is_err());

        // Drop one guard
        drop(guard1);

        // Should now be available
        let guard3 = limiter.try_acquire(provider_id, max_concurrent).await;
        assert!(guard3.is_ok());
    }

    #[tokio::test]
    async fn test_semaphore_no_limit() {
        let limiter = SemaphoreLimiter::new();
        let provider_id = "test-provider";
        let max_concurrent = 0; // No limit

        // Should allow many concurrent requests
        let mut guards = Vec::new();
        for _ in 0..100 {
            let guard = limiter.try_acquire(provider_id, max_concurrent).await;
            assert!(guard.is_ok());
            guards.push(guard.unwrap());
        }
    }

    #[tokio::test]
    async fn test_semaphore_available_permits() {
        let limiter = SemaphoreLimiter::new();
        let provider_id = "test-provider";
        let max_concurrent = 5;

        // Initially should have all permits available
        let _guard1 = limiter
            .try_acquire(provider_id, max_concurrent)
            .await
            .unwrap();
        let available = limiter.available_permits(provider_id);
        assert_eq!(available, Some(4));

        let _guard2 = limiter
            .try_acquire(provider_id, max_concurrent)
            .await
            .unwrap();
        let available = limiter.available_permits(provider_id);
        assert_eq!(available, Some(3));
    }

    #[tokio::test]
    async fn test_semaphore_acquire_waits() {
        let limiter = Arc::new(SemaphoreLimiter::new());
        let provider_id = "test-provider";
        let max_concurrent = 1;

        // Acquire the only permit
        let guard1 = limiter
            .try_acquire(provider_id, max_concurrent)
            .await
            .unwrap();

        // Spawn a task that will wait for the permit
        let limiter_clone = limiter.clone();
        let provider_id_clone = provider_id.to_string();
        let handle = tokio::spawn(async move {
            let _guard = limiter_clone
                .acquire(&provider_id_clone, max_concurrent)
                .await;
            "acquired"
        });

        // Give the task time to start waiting
        sleep(Duration::from_millis(100)).await;

        // Task should still be waiting
        assert!(!handle.is_finished());

        // Release the permit
        drop(guard1);

        // Task should complete
        let result = tokio::time::timeout(Duration::from_secs(1), handle).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), "acquired");
    }

    #[tokio::test]
    async fn test_semaphore_reset() {
        let limiter = SemaphoreLimiter::new();
        let provider_id = "test-provider";
        let max_concurrent = 1;

        // Acquire the permit
        let _guard = limiter
            .try_acquire(provider_id, max_concurrent)
            .await
            .unwrap();

        // Should be blocked
        assert!(limiter
            .try_acquire(provider_id, max_concurrent)
            .await
            .is_err());

        // Reset (note: this doesn't release existing permits, just removes the semaphore)
        limiter.reset(provider_id);

        // After reset, a new semaphore is created
        let guard2 = limiter.try_acquire(provider_id, max_concurrent).await;
        assert!(guard2.is_ok());
    }

    #[tokio::test]
    async fn test_semaphore_multiple_providers() {
        let limiter = SemaphoreLimiter::new();
        let provider1 = "provider-1";
        let provider2 = "provider-2";
        let max_concurrent = 1;

        // Acquire permit for provider1
        let _guard1 = limiter
            .try_acquire(provider1, max_concurrent)
            .await
            .unwrap();

        // Should still be able to acquire for provider2
        let guard2 = limiter.try_acquire(provider2, max_concurrent).await;
        assert!(guard2.is_ok());

        // But not another for provider1
        assert!(limiter
            .try_acquire(provider1, max_concurrent)
            .await
            .is_err());
    }
}
