use crate::models::{Backend, BackendMetrics, LoadBalancingStrategy};
use rand::Rng;
use std::collections::HashMap;

/// Load balancer for selecting backends based on configured strategy
pub struct LoadBalancer {
    strategy: LoadBalancingStrategy,
    // Track round-robin state for each priority group
    round_robin_state: std::sync::Mutex<HashMap<i32, usize>>,
}

impl LoadBalancer {
    /// Create a new load balancer with the specified strategy
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            round_robin_state: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Create a load balancer with default priority-based strategy
    pub fn default() -> Self {
        Self::new(LoadBalancingStrategy::PriorityBased)
    }

    /// Select a backend from the available backends based on the configured strategy
    pub fn select_backend(
        &self,
        backends: Vec<Backend>,
        _metrics: Option<&HashMap<String, BackendMetrics>>,
    ) -> Option<Backend> {
        if backends.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::PriorityBased => self.select_priority_based(backends),
            LoadBalancingStrategy::WeightedRoundRobin => self.select_weighted_round_robin(backends),
            LoadBalancingStrategy::LeastLatency => {
                // LeastLatency strategy would use metrics, but for now we fall back to priority-based
                // This will be implemented in a future task when metrics tracking is added
                self.select_priority_based(backends)
            }
        }
    }

    /// Select backend using priority-based strategy with weighted random selection within priority groups
    fn select_priority_based(&self, mut backends: Vec<Backend>) -> Option<Backend> {
        if backends.is_empty() {
            return None;
        }

        // Sort by priority (higher priority first)
        backends.sort_by(|a, b| b.provider.priority.cmp(&a.provider.priority));

        // Get the highest priority
        let highest_priority = backends[0].provider.priority;

        // Filter backends with the highest priority
        let highest_priority_backends: Vec<Backend> = backends
            .into_iter()
            .filter(|b| b.provider.priority == highest_priority)
            .collect();

        // Use weighted random selection within the priority group
        self.weighted_random_select(highest_priority_backends)
    }

    /// Select backend using weighted round-robin strategy
    fn select_weighted_round_robin(&self, mut backends: Vec<Backend>) -> Option<Backend> {
        if backends.is_empty() {
            return None;
        }

        // Sort by priority first, then by weight
        backends.sort_by(|a, b| {
            b.provider
                .priority
                .cmp(&a.provider.priority)
                .then_with(|| b.provider.weight.cmp(&a.provider.weight))
        });

        // Group by priority
        let mut priority_groups: HashMap<i32, Vec<Backend>> = HashMap::new();
        for backend in backends {
            priority_groups
                .entry(backend.provider.priority)
                .or_insert_with(Vec::new)
                .push(backend);
        }

        // Get the highest priority group
        let highest_priority = priority_groups.keys().max().copied()?;
        let backends_in_group = priority_groups.get(&highest_priority)?;

        // Perform weighted round-robin within the priority group
        self.weighted_round_robin_select(highest_priority, backends_in_group.clone())
    }

    /// Weighted random selection algorithm
    /// Backends with higher weights have proportionally higher probability of being selected
    fn weighted_random_select(&self, backends: Vec<Backend>) -> Option<Backend> {
        if backends.is_empty() {
            return None;
        }

        // Calculate total weight
        let total_weight: i32 = backends.iter().map(|b| b.provider.weight.max(1)).sum();

        if total_weight <= 0 {
            // If all weights are 0 or negative, select randomly
            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..backends.len());
            return Some(backends[index].clone());
        }

        // Generate random number in range [0, total_weight)
        let mut rng = rand::thread_rng();
        let mut random_weight = rng.gen_range(0..total_weight);

        // Select backend based on cumulative weight
        for backend in backends {
            let weight = backend.provider.weight.max(1);
            if random_weight < weight {
                return Some(backend);
            }
            random_weight -= weight;
        }

        // Fallback (should not reach here)
        None
    }

    /// Weighted round-robin selection algorithm
    /// Distributes requests according to weights in a round-robin fashion
    fn weighted_round_robin_select(
        &self,
        priority: i32,
        backends: Vec<Backend>,
    ) -> Option<Backend> {
        if backends.is_empty() {
            return None;
        }

        // Get or initialize the round-robin counter for this priority group
        let mut state = self.round_robin_state.lock().unwrap();
        let counter = state.entry(priority).or_insert(0);

        // Calculate total weight
        let total_weight: i32 = backends.iter().map(|b| b.provider.weight.max(1)).sum();

        if total_weight <= 0 {
            // If all weights are 0 or negative, use simple round-robin
            let index = *counter % backends.len();
            *counter = (*counter + 1) % backends.len();
            return Some(backends[index].clone());
        }

        // Map counter to weighted position
        let position = *counter % total_weight as usize;
        *counter = (*counter + 1) % total_weight as usize;

        // Find the backend corresponding to this position
        let mut cumulative_weight = 0;
        for backend in backends {
            let weight = backend.provider.weight.max(1) as usize;
            cumulative_weight += weight;
            if position < cumulative_weight {
                return Some(backend);
            }
        }

        // Fallback (should not reach here)
        None
    }

    /// Get the current load balancing strategy
    pub fn strategy(&self) -> &LoadBalancingStrategy {
        &self.strategy
    }

    /// Update the load balancing strategy
    pub fn set_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.strategy = strategy;
        // Clear round-robin state when strategy changes
        if let Ok(mut state) = self.round_robin_state.lock() {
            state.clear();
        }
    }

    /// Reset round-robin state (useful for testing or when backend configuration changes)
    pub fn reset_state(&self) {
        if let Ok(mut state) = self.round_robin_state.lock() {
            state.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Provider, ProviderType};

    fn create_test_backend(id: &str, priority: i32, weight: i32) -> Backend {
        let provider = Provider {
            id: id.to_string(),
            name: format!("Provider {}", id),
            provider_type: ProviderType::OpenAI,
            endpoint: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            priority,
            weight,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };
        Backend::new(provider)
    }

    #[test]
    fn test_empty_backends() {
        let lb = LoadBalancer::default();
        let result = lb.select_backend(vec![], None);
        assert!(result.is_none());
    }

    #[test]
    fn test_single_backend() {
        let lb = LoadBalancer::default();
        let backend = create_test_backend("1", 0, 1);
        let result = lb.select_backend(vec![backend.clone()], None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider.id, "1");
    }

    #[test]
    fn test_priority_based_selection() {
        let lb = LoadBalancer::new(LoadBalancingStrategy::PriorityBased);
        let backends = vec![
            create_test_backend("low", 0, 1),
            create_test_backend("high", 10, 1),
            create_test_backend("medium", 5, 1),
        ];

        // Should always select from highest priority (10)
        for _ in 0..10 {
            let result = lb.select_backend(backends.clone(), None);
            assert!(result.is_some());
            assert_eq!(result.unwrap().provider.id, "high");
        }
    }

    #[test]
    fn test_weighted_random_distribution() {
        let lb = LoadBalancer::new(LoadBalancingStrategy::PriorityBased);
        let backends = vec![
            create_test_backend("heavy", 0, 80),
            create_test_backend("light", 0, 20),
        ];

        // Run many iterations to test distribution
        let mut heavy_count = 0;
        let mut _light_count = 0;
        let iterations = 1000;

        for _ in 0..iterations {
            let result = lb.select_backend(backends.clone(), None);
            assert!(result.is_some());
            match result.unwrap().provider.id.as_str() {
                "heavy" => heavy_count += 1,
                "light" => _light_count += 1,
                _ => panic!("Unexpected backend selected"),
            }
        }

        // Check that distribution is roughly 80:20 (with some tolerance)
        let heavy_ratio = heavy_count as f64 / iterations as f64;
        assert!(
            heavy_ratio > 0.7 && heavy_ratio < 0.9,
            "Heavy ratio: {}",
            heavy_ratio
        );
    }

    #[test]
    fn test_weighted_round_robin() {
        let lb = LoadBalancer::new(LoadBalancingStrategy::WeightedRoundRobin);
        let backends = vec![
            create_test_backend("a", 0, 2),
            create_test_backend("b", 0, 1),
        ];

        // Should follow pattern: a, a, b, a, a, b, ...
        let mut selections = Vec::new();
        for _ in 0..9 {
            let result = lb.select_backend(backends.clone(), None);
            assert!(result.is_some());
            selections.push(result.unwrap().provider.id);
        }

        // Count occurrences
        let a_count = selections.iter().filter(|&id| id == "a").count();
        let b_count = selections.iter().filter(|&id| id == "b").count();

        // Should be roughly 2:1 ratio
        assert_eq!(a_count, 6);
        assert_eq!(b_count, 3);
    }

    #[test]
    fn test_priority_groups_with_weights() {
        let lb = LoadBalancer::new(LoadBalancingStrategy::PriorityBased);
        let backends = vec![
            create_test_backend("high1", 10, 3),
            create_test_backend("high2", 10, 1),
            create_test_backend("low", 0, 10), // Should never be selected despite high weight
        ];

        // Run multiple selections
        for _ in 0..20 {
            let result = lb.select_backend(backends.clone(), None);
            assert!(result.is_some());
            let id = result.unwrap().provider.id;
            // Should only select from high priority group
            assert!(id == "high1" || id == "high2");
        }
    }

    #[test]
    fn test_zero_and_negative_weights() {
        let lb = LoadBalancer::new(LoadBalancingStrategy::PriorityBased);
        let backends = vec![
            create_test_backend("zero", 0, 0),
            create_test_backend("negative", 0, -5),
        ];

        // Should still select a backend (treated as equal weights)
        let result = lb.select_backend(backends, None);
        assert!(result.is_some());
    }

    #[test]
    fn test_strategy_change() {
        let mut lb = LoadBalancer::new(LoadBalancingStrategy::PriorityBased);
        assert!(matches!(
            lb.strategy(),
            LoadBalancingStrategy::PriorityBased
        ));

        lb.set_strategy(LoadBalancingStrategy::WeightedRoundRobin);
        assert!(matches!(
            lb.strategy(),
            LoadBalancingStrategy::WeightedRoundRobin
        ));
    }

    #[test]
    fn test_reset_state() {
        let lb = LoadBalancer::new(LoadBalancingStrategy::WeightedRoundRobin);
        let backends = vec![
            create_test_backend("a", 0, 1),
            create_test_backend("b", 0, 1),
        ];

        // Make some selections
        for _ in 0..5 {
            lb.select_backend(backends.clone(), None);
        }

        // Reset state
        lb.reset_state();

        // State should be cleared (we can't directly verify, but it shouldn't panic)
        let result = lb.select_backend(backends, None);
        assert!(result.is_some());
    }
}
