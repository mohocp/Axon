//! AgentLang Checkpoint/Resume system.
//!
//! MVP v0.1: In-memory checkpoint store with schema version validation.

use al_diagnostics::{ErrorCode, RuntimeFailure};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Checkpoint metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointMeta {
    pub checkpoint_id: String,
    pub created_at: String,
    pub profile: String,
    pub schema_version: String,
    pub hash: String,
}

/// A checkpoint snapshot of task-local runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub meta: CheckpointMeta,
    pub state: serde_json::Value,
    pub effect_journal: Vec<EffectEntry>,
}

/// An entry in the effect journal for idempotency tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectEntry {
    pub idempotency_key: String,
    pub committed: bool,
    pub description: String,
}

/// In-memory checkpoint store.
#[derive(Debug, Default)]
pub struct CheckpointStore {
    checkpoints: HashMap<String, Checkpoint>,
}

impl CheckpointStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new checkpoint.
    pub fn create(&mut self, checkpoint: Checkpoint) -> String {
        let id = checkpoint.meta.checkpoint_id.clone();
        self.checkpoints.insert(id.clone(), checkpoint);
        id
    }

    /// Restore a checkpoint by ID.
    pub fn restore(&self, checkpoint_id: &str) -> Result<&Checkpoint, RuntimeFailure> {
        self.checkpoints.get(checkpoint_id).ok_or_else(|| {
            RuntimeFailure::new(
                ErrorCode::CheckpointInvalid,
                format!("Checkpoint '{}' not found", checkpoint_id),
            )
        })
    }

    /// Validate checkpoint integrity before restore.
    pub fn validate(&self, checkpoint_id: &str, expected_profile: &str) -> Result<(), RuntimeFailure> {
        let cp = self.restore(checkpoint_id)?;
        if cp.meta.profile != expected_profile {
            return Err(RuntimeFailure::new(
                ErrorCode::CheckpointInvalid,
                format!(
                    "Profile mismatch: checkpoint has '{}', expected '{}'",
                    cp.meta.profile, expected_profile
                ),
            ));
        }
        Ok(())
    }

    /// List all checkpoint IDs.
    pub fn list(&self) -> Vec<&str> {
        self.checkpoints.keys().map(|k| k.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_checkpoint(id: &str) -> Checkpoint {
        Checkpoint {
            meta: CheckpointMeta {
                checkpoint_id: id.to_string(),
                created_at: "2026-02-24T00:00:00Z".to_string(),
                profile: "mvp-0.1".to_string(),
                schema_version: "1".to_string(),
                hash: "abc123".to_string(),
            },
            state: serde_json::json!({"x": 42}),
            effect_journal: vec![],
        }
    }

    #[test]
    fn create_and_restore() {
        let mut store = CheckpointStore::new();
        store.create(test_checkpoint("cp1"));
        let cp = store.restore("cp1").unwrap();
        assert_eq!(cp.meta.checkpoint_id, "cp1");
    }

    #[test]
    fn restore_missing_fails() {
        let store = CheckpointStore::new();
        let err = store.restore("missing").unwrap_err();
        assert_eq!(err.code, ErrorCode::CheckpointInvalid);
    }

    #[test]
    fn validate_profile_mismatch() {
        let mut store = CheckpointStore::new();
        store.create(test_checkpoint("cp1"));
        let err = store.validate("cp1", "wrong-profile").unwrap_err();
        assert_eq!(err.code, ErrorCode::CheckpointInvalid);
    }

    #[test]
    fn validate_correct_profile() {
        let mut store = CheckpointStore::new();
        store.create(test_checkpoint("cp1"));
        assert!(store.validate("cp1", "mvp-0.1").is_ok());
    }
}
