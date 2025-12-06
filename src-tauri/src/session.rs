use crate::error::Result;
use crate::models::{DecisionRecord, Session};
use crate::repository::{DecisionChainRepository, SessionRepository};
use dashmap::DashMap;
use std::sync::Arc;

/// SessionManager manages active sessions in memory with DashMap
/// and persists them to the database
pub struct SessionManager {
    /// Active sessions stored in memory
    sessions: Arc<DashMap<String, ActiveSession>>,
    /// Session repository for database operations
    repository: Arc<SessionRepository>,
    /// Decision chain repository for persisting decision chains
    decision_chain_repository: Arc<DecisionChainRepository>,
    /// Session TTL in seconds (default: 5 minutes = 300 seconds)
    ttl_seconds: i64,
}

/// Active session with decision chain tracking
#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub session: Session,
    pub decision_chain: Vec<DecisionRecord>,
}

impl SessionManager {
    pub fn new(
        repository: Arc<SessionRepository>,
        decision_chain_repository: Arc<DecisionChainRepository>,
        ttl_seconds: i64,
    ) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            repository,
            decision_chain_repository,
            ttl_seconds,
        }
    }

    /// Get an existing session or create a new one
    /// Requirements: 6.1, 6.2
    pub async fn get_or_create_session(
        &self,
        session_id: Option<String>,
        backend_id: String,
    ) -> Result<ActiveSession> {
        // If session_id is provided, try to get existing session
        if let Some(id) = session_id {
            if let Some(active_session) = self.sessions.get(&id) {
                // Check if session is expired
                if !active_session.session.is_expired(self.ttl_seconds) {
                    return Ok(active_session.clone());
                } else {
                    // Remove expired session from memory
                    self.sessions.remove(&id);
                }
            }
        }

        // Create new session
        let session = Session::new(backend_id.clone());
        let active_session = ActiveSession {
            session: session.clone(),
            decision_chain: Vec::new(),
        };

        tracing::info!(
            session_id = %session.id,
            backend_id = %backend_id,
            "Created new session"
        );

        // Store in memory
        self.sessions
            .insert(session.id.clone(), active_session.clone());

        // Persist to database
        self.repository.create(&session).await?;

        Ok(active_session)
    }

    /// Update session activity timestamp
    /// Requirements: 6.2
    pub async fn update_activity(&self, session_id: &str) -> Result<()> {
        // Update in-memory session
        if let Some(mut active_session) = self.sessions.get_mut(session_id) {
            active_session.session.update_activity();

            // Persist to database
            self.repository
                .update_activity(session_id, active_session.session.last_activity)
                .await?;
        }

        Ok(())
    }

    /// Get active session by ID
    pub fn get_session(&self, session_id: &str) -> Option<ActiveSession> {
        self.sessions
            .get(session_id)
            .map(|entry| entry.value().clone())
    }

    /// Add a decision record to the session's decision chain
    pub fn add_decision(&self, session_id: &str, decision: DecisionRecord) {
        if let Some(mut active_session) = self.sessions.get_mut(session_id) {
            active_session.decision_chain.push(decision);
        }
    }

    /// Get the decision chain for a session
    pub fn get_decision_chain(&self, session_id: &str) -> Vec<DecisionRecord> {
        self.sessions
            .get(session_id)
            .map(|entry| entry.decision_chain.clone())
            .unwrap_or_default()
    }

    /// Get all active sessions
    pub fn get_active_sessions(&self) -> Vec<ActiveSession> {
        self.sessions
            .iter()
            .filter(|entry| !entry.session.is_expired(self.ttl_seconds))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get count of active sessions
    pub fn active_session_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|entry| !entry.session.is_expired(self.ttl_seconds))
            .count()
    }

    /// Get count of active sessions for a specific backend
    pub fn active_session_count_for_backend(&self, backend_id: &str) -> usize {
        self.sessions
            .iter()
            .filter(|entry| {
                entry.session.backend_id == backend_id
                    && !entry.session.is_expired(self.ttl_seconds)
            })
            .count()
    }

    /// Clean up expired sessions
    /// This method removes expired sessions from memory and marks them as expired in the database
    /// Requirements: 6.3
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let threshold = now - self.ttl_seconds;
        let mut expired_count = 0;

        // Collect expired session IDs
        let expired_ids: Vec<String> = self
            .sessions
            .iter()
            .filter(|entry| entry.session.is_expired(self.ttl_seconds))
            .map(|entry| entry.session.id.clone())
            .collect();

        // Remove from memory and mark as expired in database
        for session_id in expired_ids {
            if let Some((_, active_session)) = self.sessions.remove(&session_id) {
                // Mark as expired in database
                if let Err(e) = self.repository.expire(&active_session.session.id).await {
                    tracing::warn!(
                        "Failed to mark session {} as expired in database: {}",
                        active_session.session.id,
                        e
                    );
                }
                expired_count += 1;
            }
        }

        // Also expire inactive sessions directly in the database
        // (in case some sessions were not in memory)
        match self.repository.expire_inactive(threshold).await {
            Ok(db_expired) => {
                tracing::debug!(
                    "Cleaned up {} expired sessions from memory, {} from database",
                    expired_count,
                    db_expired
                );
            }
            Err(e) => {
                tracing::warn!("Failed to expire inactive sessions in database: {}", e);
            }
        }

        Ok(expired_count)
    }

    /// Start a background task that periodically cleans up expired sessions
    /// Requirements: 6.3
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                match self.cleanup_expired().await {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!("Cleaned up {} expired sessions", count);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error during session cleanup: {}", e);
                    }
                }
            }
        });
    }

    /// Persist decision chain to database
    /// Requirements: 2.5, 6.4
    pub async fn persist_decision_chain(&self, session_id: &str, usage_log_id: &str) -> Result<()> {
        if let Some(active_session) = self.sessions.get(session_id) {
            if !active_session.decision_chain.is_empty() {
                self.decision_chain_repository
                    .create_batch(usage_log_id, &active_session.decision_chain)
                    .await?;
            }
        }
        Ok(())
    }

    /// Get decision chain from database for a usage log
    /// Requirements: 6.4
    pub async fn get_decision_chain_from_db(
        &self,
        usage_log_id: &str,
    ) -> Result<Vec<DecisionRecord>> {
        self.decision_chain_repository
            .get_by_usage_log(usage_log_id)
            .await
    }

    /// Clear decision chain for a session (useful after persisting)
    pub fn clear_decision_chain(&self, session_id: &str) {
        if let Some(mut active_session) = self.sessions.get_mut(session_id) {
            active_session.decision_chain.clear();
        }
    }
}
