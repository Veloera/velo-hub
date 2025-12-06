use crate::config::CircuitBreakerConfig;
use crate::error::CircuitBreakerError;
use crate::models::CircuitState;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Circuit Breaker implementation for protecting against cascading failures
///
/// The circuit breaker tracks the error rate of each provider and automatically
/// isolates failing providers to prevent cascading failures. It implements a
/// state machine with three states: Closed, Open, and HalfOpen.
pub struct CircuitBreaker {
    /// Configuration for circuit breaker behavior
    config: CircuitBreakerConfig,
    /// State tracking for each provider (keyed by provider_id)
    states: Arc<DashMap<String, ProviderCircuitState>>,
}

/// Internal state tracking for a single provider's circuit breaker
#[derive(Debug, Clone)]
struct ProviderCircuitState {
    /// Current state of the circuit breaker
    state: CircuitState,
    /// Sliding window of request results (true = success, false = failure)
    window: Vec<RequestResult>,
    /// Timestamp when the circuit was opened (for cooldown calculation)
    opened_at: Option<Instant>,
    /// Number of test requests made in HalfOpen state
    half_open_attempts: u32,
}

/// Result of a single request in the sliding window
#[derive(Debug, Clone, Copy)]
struct RequestResult {
    success: bool,
    timestamp: Instant,
}

impl CircuitBreaker {
    /// Create a new CircuitBreaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            states: Arc::new(DashMap::new()),
        }
    }

    /// Create a new CircuitBreaker with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Check if a provider is available (circuit is not open)
    ///
    /// Returns Ok(()) if the provider can be used, or an error if the circuit is open
    pub fn check_available(&self, provider_id: &str) -> Result<(), CircuitBreakerError> {
        let mut state_entry = self
            .states
            .entry(provider_id.to_string())
            .or_insert_with(|| ProviderCircuitState {
                state: CircuitState::Closed,
                window: Vec::new(),
                opened_at: None,
                half_open_attempts: 0,
            });

        let state = &mut *state_entry;

        // Check if we need to transition from Open to HalfOpen
        if state.state == CircuitState::Open {
            if let Some(opened_at) = state.opened_at {
                let cooldown = Duration::from_secs(self.config.cooldown_duration_secs);
                if opened_at.elapsed() >= cooldown {
                    // Transition to HalfOpen
                    state.state = CircuitState::HalfOpen;
                    state.half_open_attempts = 0;
                    tracing::info!(
                        provider_id = provider_id,
                        "Circuit breaker transitioning from Open to HalfOpen after cooldown"
                    );
                } else {
                    return Err(CircuitBreakerError::Open(provider_id.to_string()));
                }
            }
        }

        // In HalfOpen state, limit the number of test requests
        if state.state == CircuitState::HalfOpen {
            if state.half_open_attempts >= self.config.half_open_max_requests {
                return Err(CircuitBreakerError::Open(provider_id.to_string()));
            }
            state.half_open_attempts += 1;
        }

        Ok(())
    }

    /// Record a successful request for a provider
    ///
    /// This updates the sliding window and may transition the circuit breaker state
    pub fn record_success(&self, provider_id: &str) {
        let mut state_entry = self
            .states
            .entry(provider_id.to_string())
            .or_insert_with(|| ProviderCircuitState {
                state: CircuitState::Closed,
                window: Vec::new(),
                opened_at: None,
                half_open_attempts: 0,
            });

        let state = &mut *state_entry;

        // Add success to sliding window
        state.window.push(RequestResult {
            success: true,
            timestamp: Instant::now(),
        });

        // Clean up old entries outside the window
        self.cleanup_window(&mut state.window);

        // Handle state transitions based on success
        match state.state {
            CircuitState::HalfOpen => {
                // In HalfOpen, a success means we can close the circuit
                state.state = CircuitState::Closed;
                state.opened_at = None;
                state.half_open_attempts = 0;
                tracing::info!(
                    provider_id = provider_id,
                    "Circuit breaker transitioning from HalfOpen to Closed after successful request"
                );
            }
            CircuitState::Open => {
                // This shouldn't happen, but handle it gracefully
                tracing::warn!(
                    provider_id = provider_id,
                    "Received success while circuit is Open - this should not happen"
                );
            }
            CircuitState::Closed => {
                // Already closed, nothing to do
            }
        }
    }

    /// Record a failed request for a provider
    ///
    /// This updates the sliding window and may open the circuit if error rate exceeds threshold
    pub fn record_failure(&self, provider_id: &str) {
        let mut state_entry = self
            .states
            .entry(provider_id.to_string())
            .or_insert_with(|| ProviderCircuitState {
                state: CircuitState::Closed,
                window: Vec::new(),
                opened_at: None,
                half_open_attempts: 0,
            });

        let state = &mut *state_entry;

        // Add failure to sliding window
        state.window.push(RequestResult {
            success: false,
            timestamp: Instant::now(),
        });

        // Clean up old entries outside the window
        self.cleanup_window(&mut state.window);

        // Calculate error rate
        let error_rate = self.calculate_error_rate(&state.window);

        // Handle state transitions based on failure
        match state.state {
            CircuitState::Closed => {
                // Check if we should open the circuit
                if error_rate >= self.config.failure_threshold && state.window.len() >= 5 {
                    state.state = CircuitState::Open;
                    state.opened_at = Some(Instant::now());
                    tracing::warn!(
                        provider_id = provider_id,
                        error_rate = error_rate,
                        threshold = self.config.failure_threshold,
                        "Circuit breaker opening due to high error rate"
                    );
                }
            }
            CircuitState::HalfOpen => {
                // In HalfOpen, any failure reopens the circuit
                state.state = CircuitState::Open;
                state.opened_at = Some(Instant::now());
                state.half_open_attempts = 0;
                tracing::warn!(
                    provider_id = provider_id,
                    "Circuit breaker reopening from HalfOpen after failed test request"
                );
            }
            CircuitState::Open => {
                // Already open, just update the timestamp
                state.opened_at = Some(Instant::now());
            }
        }
    }

    /// Get the current state of a provider's circuit breaker
    pub fn get_state(&self, provider_id: &str) -> CircuitState {
        self.states
            .get(provider_id)
            .map(|state| state.state.clone())
            .unwrap_or(CircuitState::Closed)
    }

    /// Get detailed metrics for a provider's circuit breaker
    pub fn get_metrics(&self, provider_id: &str) -> CircuitBreakerMetrics {
        let state = self.states.get(provider_id);

        if let Some(state) = state {
            let error_rate = self.calculate_error_rate(&state.window);
            let total_requests = state.window.len();
            let failed_requests = state.window.iter().filter(|r| !r.success).count();

            CircuitBreakerMetrics {
                state: state.state.clone(),
                error_rate,
                total_requests,
                failed_requests,
                opened_at: state.opened_at.map(|instant| {
                    chrono::Utc::now().timestamp() - instant.elapsed().as_secs() as i64
                }),
            }
        } else {
            CircuitBreakerMetrics {
                state: CircuitState::Closed,
                error_rate: 0.0,
                total_requests: 0,
                failed_requests: 0,
                opened_at: None,
            }
        }
    }

    /// Reset the circuit breaker state for a provider (useful for testing or manual intervention)
    pub fn reset(&self, provider_id: &str) {
        if let Some(mut state) = self.states.get_mut(provider_id) {
            state.state = CircuitState::Closed;
            state.window.clear();
            state.opened_at = None;
            state.half_open_attempts = 0;
            tracing::info!(provider_id = provider_id, "Circuit breaker manually reset");
        }
    }

    /// Remove old entries from the sliding window that are outside the time window
    fn cleanup_window(&self, window: &mut Vec<RequestResult>) {
        let window_duration = Duration::from_secs(self.config.window_duration_secs);
        let cutoff = Instant::now() - window_duration;

        window.retain(|result| result.timestamp > cutoff);
    }

    /// Calculate the error rate from the sliding window
    ///
    /// Returns a value between 0.0 and 1.0
    fn calculate_error_rate(&self, window: &[RequestResult]) -> f64 {
        if window.is_empty() {
            return 0.0;
        }

        let failed = window.iter().filter(|r| !r.success).count();
        failed as f64 / window.len() as f64
    }
}

/// Metrics for a provider's circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub state: CircuitState,
    pub error_rate: f64,
    pub total_requests: usize,
    pub failed_requests: usize,
    pub opened_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::with_defaults();
        assert_eq!(cb.get_state("test-provider"), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens_on_high_error_rate() {
        let config = CircuitBreakerConfig {
            failure_threshold: 0.5,
            window_duration_secs: 60,
            cooldown_duration_secs: 30,
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new(config);

        // Record some failures to trigger opening
        for _ in 0..3 {
            cb.record_success("test-provider");
        }
        for _ in 0..5 {
            cb.record_failure("test-provider");
        }

        assert_eq!(cb.get_state("test-provider"), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_stays_closed_with_low_error_rate() {
        let cb = CircuitBreaker::with_defaults();

        // Record mostly successes
        for _ in 0..10 {
            cb.record_success("test-provider");
        }
        cb.record_failure("test-provider");

        assert_eq!(cb.get_state("test-provider"), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_check_available_when_closed() {
        let cb = CircuitBreaker::with_defaults();
        assert!(cb.check_available("test-provider").is_ok());
    }

    #[test]
    fn test_circuit_breaker_check_available_when_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 0.5,
            window_duration_secs: 60,
            cooldown_duration_secs: 30,
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..3 {
            cb.record_success("test-provider");
        }
        for _ in 0..5 {
            cb.record_failure("test-provider");
        }

        assert!(cb.check_available("test-provider").is_err());
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 0.5,
            window_duration_secs: 60,
            cooldown_duration_secs: 30,
            half_open_max_requests: 3,
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..3 {
            cb.record_success("test-provider");
        }
        for _ in 0..5 {
            cb.record_failure("test-provider");
        }

        assert_eq!(cb.get_state("test-provider"), CircuitState::Open);

        // Reset
        cb.reset("test-provider");
        assert_eq!(cb.get_state("test-provider"), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_metrics() {
        let cb = CircuitBreaker::with_defaults();

        cb.record_success("test-provider");
        cb.record_success("test-provider");
        cb.record_failure("test-provider");

        let metrics = cb.get_metrics("test-provider");
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.failed_requests, 1);
        assert!((metrics.error_rate - 0.333).abs() < 0.01);
    }
}
