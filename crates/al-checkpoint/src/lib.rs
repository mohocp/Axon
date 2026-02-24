//! AgentLang Checkpoint/Resume system.
//!
//! MVP v0.1: In-memory checkpoint store with schema version validation,
//! hash integrity checking, and effect journal for idempotency-safe resume.

use al_diagnostics::{ErrorCode, RuntimeFailure};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current schema version for checkpoint format.
pub const CHECKPOINT_SCHEMA_VERSION: &str = "1";

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

impl Checkpoint {
    /// Serialize this checkpoint to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize a checkpoint from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Compute the hash of the checkpoint state for integrity validation.
    pub fn compute_state_hash(&self) -> String {
        let state_str = serde_json::to_string(&self.state).unwrap_or_default();
        simple_hash(&state_str)
    }

    /// Validate that the stored hash matches the computed hash.
    pub fn validate_hash(&self) -> bool {
        self.meta.hash == self.compute_state_hash()
    }
}

/// An entry in the effect journal for idempotency tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectEntry {
    pub idempotency_key: String,
    pub committed: bool,
    pub description: String,
}

/// An effect journal that tracks side-effects for idempotency-safe resume.
///
/// During execution, side-effecting operations record entries in the journal.
/// On resume from checkpoint, previously-committed effects are skipped
/// to avoid re-execution of non-idempotent operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffectJournal {
    entries: Vec<EffectEntry>,
}

impl EffectJournal {
    /// Create a new empty effect journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a journal from existing entries (e.g. restored from checkpoint).
    pub fn from_entries(entries: Vec<EffectEntry>) -> Self {
        Self { entries }
    }

    /// Record a new effect. Returns `true` if the effect was newly recorded,
    /// `false` if it was already committed (idempotency skip).
    pub fn record_effect(&mut self, key: &str, description: &str) -> bool {
        // Check if this effect was already committed in a prior run.
        if self.is_committed(key) {
            return false; // Skip: already executed
        }

        // Check if already recorded but not committed.
        if self.entries.iter().any(|e| e.idempotency_key == key) {
            return false;
        }

        self.entries.push(EffectEntry {
            idempotency_key: key.to_string(),
            committed: false,
            description: description.to_string(),
        });
        true
    }

    /// Mark an effect as committed (successfully completed).
    pub fn commit_effect(&mut self, key: &str) -> bool {
        for entry in &mut self.entries {
            if entry.idempotency_key == key && !entry.committed {
                entry.committed = true;
                return true;
            }
        }
        false
    }

    /// Check if an effect with the given key has already been committed.
    pub fn is_committed(&self, key: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.idempotency_key == key && e.committed)
    }

    /// Get all entries in the journal.
    pub fn entries(&self) -> &[EffectEntry] {
        &self.entries
    }

    /// Get all committed entries.
    pub fn committed_entries(&self) -> Vec<&EffectEntry> {
        self.entries.iter().filter(|e| e.committed).collect()
    }

    /// Get all uncommitted entries.
    pub fn uncommitted_entries(&self) -> Vec<&EffectEntry> {
        self.entries.iter().filter(|e| !e.committed).collect()
    }

    /// Convert to a Vec<EffectEntry> for checkpoint serialization.
    pub fn to_entries(&self) -> Vec<EffectEntry> {
        self.entries.clone()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Compute a simple deterministic hash of a string (DJB2 variant).
/// Used for checkpoint state integrity validation.
pub fn simple_hash(input: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{:016x}", hash)
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
    pub fn validate(
        &self,
        checkpoint_id: &str,
        expected_profile: &str,
    ) -> Result<(), RuntimeFailure> {
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

    /// Validate checkpoint with hash integrity check.
    pub fn validate_with_hash(
        &self,
        checkpoint_id: &str,
        expected_profile: &str,
    ) -> Result<(), RuntimeFailure> {
        self.validate(checkpoint_id, expected_profile)?;
        let cp = self.restore(checkpoint_id)?;
        if !cp.validate_hash() {
            return Err(RuntimeFailure::with_details(
                ErrorCode::CheckpointInvalid,
                format!("Checkpoint '{}' hash integrity check failed", checkpoint_id),
                serde_json::json!({
                    "stored_hash": cp.meta.hash,
                    "computed_hash": cp.compute_state_hash(),
                }),
            ));
        }
        Ok(())
    }

    /// Validate schema version compatibility.
    pub fn validate_schema_version(&self, checkpoint_id: &str) -> Result<(), RuntimeFailure> {
        let cp = self.restore(checkpoint_id)?;
        if cp.meta.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(RuntimeFailure::with_details(
                ErrorCode::CheckpointInvalid,
                format!(
                    "Schema version mismatch: checkpoint has '{}', runtime expects '{}'",
                    cp.meta.schema_version, CHECKPOINT_SCHEMA_VERSION
                ),
                serde_json::json!({
                    "checkpoint_version": cp.meta.schema_version,
                    "runtime_version": CHECKPOINT_SCHEMA_VERSION,
                }),
            ));
        }
        Ok(())
    }

    /// List all checkpoint IDs.
    pub fn list(&self) -> Vec<&str> {
        self.checkpoints.keys().map(|k| k.as_str()).collect()
    }

    /// Get checkpoint count.
    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_checkpoint(id: &str) -> Checkpoint {
        let state = serde_json::json!({"x": 42});
        let state_str = serde_json::to_string(&state).unwrap();
        let hash = simple_hash(&state_str);
        Checkpoint {
            meta: CheckpointMeta {
                checkpoint_id: id.to_string(),
                created_at: "2026-02-24T00:00:00Z".to_string(),
                profile: "mvp-0.1".to_string(),
                schema_version: CHECKPOINT_SCHEMA_VERSION.to_string(),
                hash,
            },
            state,
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

    // -- New Round 6 tests --

    #[test]
    fn checkpoint_hash_integrity_valid() {
        let mut store = CheckpointStore::new();
        store.create(test_checkpoint("cp1"));
        assert!(store.validate_with_hash("cp1", "mvp-0.1").is_ok());
    }

    #[test]
    fn checkpoint_hash_integrity_tampered() {
        let mut store = CheckpointStore::new();
        let mut cp = test_checkpoint("cp1");
        cp.meta.hash = "tampered_hash".to_string();
        store.create(cp);
        let err = store.validate_with_hash("cp1", "mvp-0.1").unwrap_err();
        assert_eq!(err.code, ErrorCode::CheckpointInvalid);
        assert!(err.message.contains("hash integrity"));
    }

    #[test]
    fn checkpoint_schema_version_valid() {
        let mut store = CheckpointStore::new();
        store.create(test_checkpoint("cp1"));
        assert!(store.validate_schema_version("cp1").is_ok());
    }

    #[test]
    fn checkpoint_schema_version_mismatch() {
        let mut store = CheckpointStore::new();
        let mut cp = test_checkpoint("cp1");
        cp.meta.schema_version = "99".to_string();
        store.create(cp);
        let err = store.validate_schema_version("cp1").unwrap_err();
        assert_eq!(err.code, ErrorCode::CheckpointInvalid);
        assert!(err.message.contains("Schema version mismatch"));
    }

    #[test]
    fn checkpoint_serialization_roundtrip() {
        let cp = test_checkpoint("cp1");
        let json = cp.to_json().unwrap();
        let restored = Checkpoint::from_json(&json).unwrap();
        assert_eq!(restored.meta.checkpoint_id, "cp1");
        assert_eq!(restored.state, serde_json::json!({"x": 42}));
        assert!(restored.validate_hash());
    }

    #[test]
    fn checkpoint_with_effect_journal_roundtrip() {
        let mut cp = test_checkpoint("cp1");
        cp.effect_journal = vec![
            EffectEntry {
                idempotency_key: "write-file-1".to_string(),
                committed: true,
                description: "wrote output.txt".to_string(),
            },
            EffectEntry {
                idempotency_key: "http-post-1".to_string(),
                committed: false,
                description: "POST /api/data".to_string(),
            },
        ];
        let json = cp.to_json().unwrap();
        let restored = Checkpoint::from_json(&json).unwrap();
        assert_eq!(restored.effect_journal.len(), 2);
        assert!(restored.effect_journal[0].committed);
        assert!(!restored.effect_journal[1].committed);
    }

    #[test]
    fn simple_hash_deterministic() {
        let h1 = simple_hash("hello world");
        let h2 = simple_hash("hello world");
        assert_eq!(h1, h2);
        let h3 = simple_hash("hello world!");
        assert_ne!(h1, h3);
    }

    #[test]
    fn store_len_and_is_empty() {
        let mut store = CheckpointStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        store.create(test_checkpoint("cp1"));
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }

    // -- Effect journal tests --

    #[test]
    fn effect_journal_record_and_commit() {
        let mut journal = EffectJournal::new();
        assert!(journal.record_effect("key-1", "first effect"));
        assert_eq!(journal.entries().len(), 1);
        assert!(!journal.is_committed("key-1"));

        assert!(journal.commit_effect("key-1"));
        assert!(journal.is_committed("key-1"));
    }

    #[test]
    fn effect_journal_skip_committed() {
        let mut journal = EffectJournal::new();
        journal.record_effect("key-1", "first effect");
        journal.commit_effect("key-1");

        // Should return false — already committed
        assert!(!journal.record_effect("key-1", "duplicate"));
        assert_eq!(journal.entries().len(), 1);
    }

    #[test]
    fn effect_journal_skip_recorded_not_committed() {
        let mut journal = EffectJournal::new();
        journal.record_effect("key-1", "first effect");

        // Should return false — already recorded (not yet committed)
        assert!(!journal.record_effect("key-1", "duplicate"));
        assert_eq!(journal.entries().len(), 1);
    }

    #[test]
    fn effect_journal_committed_vs_uncommitted() {
        let mut journal = EffectJournal::new();
        journal.record_effect("key-1", "first");
        journal.record_effect("key-2", "second");
        journal.commit_effect("key-1");

        assert_eq!(journal.committed_entries().len(), 1);
        assert_eq!(journal.uncommitted_entries().len(), 1);
        assert_eq!(journal.committed_entries()[0].idempotency_key, "key-1");
        assert_eq!(journal.uncommitted_entries()[0].idempotency_key, "key-2");
    }

    #[test]
    fn effect_journal_from_entries_restores_state() {
        let entries = vec![EffectEntry {
            idempotency_key: "key-1".to_string(),
            committed: true,
            description: "done".to_string(),
        }];
        let journal = EffectJournal::from_entries(entries);
        assert!(journal.is_committed("key-1"));
        assert_eq!(journal.entries().len(), 1);
    }

    #[test]
    fn effect_journal_clear() {
        let mut journal = EffectJournal::new();
        journal.record_effect("key-1", "test");
        journal.clear();
        assert_eq!(journal.entries().len(), 0);
    }

    #[test]
    fn effect_journal_to_entries() {
        let mut journal = EffectJournal::new();
        journal.record_effect("key-1", "test");
        journal.commit_effect("key-1");
        let entries = journal.to_entries();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].committed);
    }
}
