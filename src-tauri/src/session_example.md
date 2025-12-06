# SessionManager Usage Example

This document demonstrates how to use the SessionManager module.

## Initialization

```rust
use std::sync::Arc;
use crate::session::SessionManager;
use crate::repository::{SessionRepository, DecisionChainRepository};
use sqlx::SqlitePool;

// Initialize repositories
let pool = SqlitePool::connect("sqlite:data.db").await?;
let session_repo = Arc::new(SessionRepository::new(pool.clone()));
let decision_chain_repo = Arc::new(DecisionChainRepository::new(pool.clone()));

// Create SessionManager with 5-minute TTL (300 seconds)
let session_manager = Arc::new(SessionManager::new(
    session_repo,
    decision_chain_repo,
    300, // TTL in seconds
));

// Start the cleanup task
session_manager.clone().start_cleanup_task();
```

## Creating or Getting a Session

```rust
// Create a new session
let active_session = session_manager
    .get_or_create_session(None, "backend-provider-id".to_string())
    .await?;

println!("Session ID: {}", active_session.session.id);

// Get existing session
let active_session = session_manager
    .get_or_create_session(
        Some("existing-session-id".to_string()),
        "backend-provider-id".to_string()
    )
    .await?;
```

## Updating Session Activity

```rust
// Update the last_activity timestamp
session_manager
    .update_activity("session-id")
    .await?;
```

## Recording Decision Chain

```rust
use crate::models::{DecisionRecord, DecisionAction};

// Add a decision to the session's decision chain
let decision = DecisionRecord::new(
    1, // step number
    DecisionAction::Selected,
    "Selected backend based on priority".to_string(),
).with_provider("provider-id".to_string());

session_manager.add_decision("session-id", decision);

// Add more decisions
let decision2 = DecisionRecord::new(
    2,
    DecisionAction::Failed,
    "Backend request failed".to_string(),
).with_provider("provider-id".to_string());

session_manager.add_decision("session-id", decision2);

let decision3 = DecisionRecord::new(
    3,
    DecisionAction::Retried,
    "Retrying with different backend".to_string(),
).with_provider("backup-provider-id".to_string());

session_manager.add_decision("session-id", decision3);
```

## Persisting Decision Chain

```rust
// After a request completes, persist the decision chain to the database
session_manager
    .persist_decision_chain("session-id", "usage-log-id")
    .await?;

// Optionally clear the decision chain from memory after persisting
session_manager.clear_decision_chain("session-id");
```

## Querying Sessions

```rust
// Get a specific session
if let Some(active_session) = session_manager.get_session("session-id") {
    println!("Backend ID: {}", active_session.session.backend_id);
    println!("Decision chain length: {}", active_session.decision_chain.len());
}

// Get all active sessions
let active_sessions = session_manager.get_active_sessions();
println!("Active sessions: {}", active_sessions.len());

// Get active session count
let count = session_manager.active_session_count();
println!("Total active sessions: {}", count);

// Get active session count for a specific backend
let backend_count = session_manager
    .active_session_count_for_backend("backend-provider-id");
println!("Active sessions for backend: {}", backend_count);
```

## Retrieving Decision Chain from Database

```rust
// Get decision chain for a usage log from the database
let decision_chain = session_manager
    .get_decision_chain_from_db("usage-log-id")
    .await?;

for decision in decision_chain {
    println!(
        "Step {}: {:?} - {}",
        decision.step_number,
        decision.action,
        decision.reason
    );
}
```

## Manual Cleanup

```rust
// Manually trigger cleanup of expired sessions
let expired_count = session_manager.cleanup_expired().await?;
println!("Cleaned up {} expired sessions", expired_count);
```

## Complete Request Flow Example

```rust
async fn handle_request(
    session_manager: Arc<SessionManager>,
    session_id: Option<String>,
    backend_id: String,
) -> Result<()> {
    // 1. Get or create session
    let active_session = session_manager
        .get_or_create_session(session_id.clone(), backend_id.clone())
        .await?;
    
    let session_id = active_session.session.id.clone();
    
    // 2. Record decision: backend selected
    let decision = DecisionRecord::new(
        1,
        DecisionAction::Selected,
        format!("Selected backend {}", backend_id),
    ).with_provider(backend_id.clone());
    
    session_manager.add_decision(&session_id, decision);
    
    // 3. Make request (simulated)
    // ... make actual request to backend ...
    
    // 4. Update activity
    session_manager.update_activity(&session_id).await?;
    
    // 5. Create usage log
    let usage_log_id = "generated-usage-log-id";
    
    // 6. Persist decision chain
    session_manager
        .persist_decision_chain(&session_id, usage_log_id)
        .await?;
    
    // 7. Clear decision chain from memory
    session_manager.clear_decision_chain(&session_id);
    
    Ok(())
}
```

## Integration with Request Router

The SessionManager is designed to be used by the RequestRouter to:

1. **Maintain session affinity**: Keep requests from the same session routed to the same backend
2. **Track routing decisions**: Record why each backend was selected or rejected
3. **Audit trail**: Provide complete visibility into routing decisions for debugging
4. **Concurrency control**: Track active sessions per backend for rate limiting

Example integration:

```rust
// In RequestRouter
pub async fn route_request(
    &self,
    request: ChatCompletionRequest,
    session_id: Option<String>,
) -> Result<ChatCompletionResponse> {
    // Get or create session
    let active_session = self.session_manager
        .get_or_create_session(session_id, backend_id)
        .await?;
    
    // If session exists, prefer the same backend
    let backend = if let Some(session) = self.session_manager.get_session(&active_session.session.id) {
        // Record decision: using cached backend
        let decision = DecisionRecord::new(
            1,
            DecisionAction::Selected,
            "Using cached backend from session".to_string(),
        ).with_provider(session.session.backend_id.clone());
        
        self.session_manager.add_decision(&active_session.session.id, decision);
        
        // Get the backend
        self.get_backend(&session.session.backend_id)?
    } else {
        // Select new backend using load balancer
        self.select_backend(&request.model).await?
    };
    
    // Make request and handle response...
    // Update activity and persist decision chain...
}
```
