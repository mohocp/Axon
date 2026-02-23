//! # al-diagnostics
//!
//! Diagnostics foundation for AgentLang MVP v0.1.
//!
//! Provides compile-time diagnostics, runtime failure representations,
//! audit event logging (JSONL schema), and a diagnostic sink for
//! collecting errors and warnings throughout compilation and execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// 1. Span — source location tracking
// ---------------------------------------------------------------------------

/// A source location span tracking where a construct appears in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// Byte offset from the beginning of the source.
    pub offset: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number (in bytes).
    pub column: usize,
    /// Length in bytes of the spanned region.
    pub length: usize,
}

impl Span {
    /// Create a new `Span`.
    pub fn new(offset: usize, line: usize, column: usize, length: usize) -> Self {
        Self {
            offset,
            line,
            column,
            length,
        }
    }

    /// A dummy span used when no real location is available.
    pub fn dummy() -> Self {
        Self {
            offset: 0,
            line: 0,
            column: 0,
            length: 0,
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::dummy()
    }
}

// ---------------------------------------------------------------------------
// 2. Error codes
// ---------------------------------------------------------------------------

/// Mandatory error codes for AgentLang MVP v0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// Feature or construct is not yet implemented.
    NotImplemented,
    /// A type mismatch was detected.
    TypeMismatch,
    /// A failure-mode arity mismatch (wrong number of failure arms / parameters).
    FailureArityMismatch,
    /// A capability was denied by the capability policy.
    CapabilityDenied,
    /// A verifiable-credential is invalid or failed verification.
    VcInvalid,
    /// An assertion failed at runtime.
    AssertionFailed,
    /// A checkpoint reference is invalid or corrupted.
    CheckpointInvalid,
    /// An error was escalated from a sub-agent or nested context.
    Escalated,
    /// A parse error occurred while reading AgentLang source.
    ParseError,
    /// An identifier could not be resolved in the current scope.
    UnknownIdentifier,
    /// A definition with the same name already exists in scope.
    DuplicateDefinition,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display in the canonical SCREAMING_SNAKE_CASE form.
        let s = match self {
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::TypeMismatch => "TYPE_MISMATCH",
            Self::FailureArityMismatch => "FAILURE_ARITY_MISMATCH",
            Self::CapabilityDenied => "CAPABILITY_DENIED",
            Self::VcInvalid => "VC_INVALID",
            Self::AssertionFailed => "ASSERTION_FAILED",
            Self::CheckpointInvalid => "CHECKPOINT_INVALID",
            Self::Escalated => "ESCALATED",
            Self::ParseError => "PARSE_ERROR",
            Self::UnknownIdentifier => "UNKNOWN_IDENTIFIER",
            Self::DuplicateDefinition => "DUPLICATE_DEFINITION",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// 3. Warning codes
// ---------------------------------------------------------------------------

/// Warning codes for AgentLang MVP v0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    /// A capability alias has been deprecated and may be removed in a future version.
    CapAliasDeprecated,
}

impl std::fmt::Display for WarningCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapAliasDeprecated => write!(f, "CAP_ALIAS_DEPRECATED"),
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Unified diagnostic code wrapper
// ---------------------------------------------------------------------------

/// A diagnostic code — either an error or a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DiagnosticCode {
    /// An error-level code.
    Error(ErrorCode),
    /// A warning-level code.
    Warning(WarningCode),
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error(e) => write!(f, "{}", e),
            Self::Warning(w) => write!(f, "{}", w),
        }
    }
}

impl From<ErrorCode> for DiagnosticCode {
    fn from(code: ErrorCode) -> Self {
        Self::Error(code)
    }
}

impl From<WarningCode> for DiagnosticCode {
    fn from(code: WarningCode) -> Self {
        Self::Warning(code)
    }
}

// ---------------------------------------------------------------------------
// 5. Severity
// ---------------------------------------------------------------------------

/// Severity level attached to a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A hard error — compilation or execution cannot proceed.
    Error,
    /// A warning — something suspicious but not fatal.
    Warning,
    /// Informational note.
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Diagnostic (compile-time diagnostic shape)
// ---------------------------------------------------------------------------

/// A compile-time diagnostic.
///
/// JSON shape:
/// ```json
/// {
///   "code": "PARSE_ERROR",
///   "message": "unexpected token `;`",
///   "severity": "error",
///   "span": { "offset": 42, "line": 3, "column": 10, "length": 1 },
///   "profile": "mvp-0.1",
///   "notes": ["did you mean `:`?"]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The diagnostic code (error or warning variant).
    pub code: DiagnosticCode,
    /// Human-readable message.
    pub message: String,
    /// Severity level.
    pub severity: Severity,
    /// Source location where the diagnostic was raised.
    pub span: Span,
    /// Profile tag — always `"mvp-0.1"` for this version.
    pub profile: String,
    /// Additional notes providing context or suggestions.
    pub notes: Vec<String>,
}

/// The profile string for MVP v0.1 diagnostics.
pub const MVP_PROFILE: &str = "mvp-0.1";

impl Diagnostic {
    // -- convenience constructors ------------------------------------------------

    /// Create a new error diagnostic.
    pub fn error(code: ErrorCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code: DiagnosticCode::Error(code),
            message: message.into(),
            severity: Severity::Error,
            span,
            profile: MVP_PROFILE.to_string(),
            notes: Vec::new(),
        }
    }

    /// Create a new warning diagnostic.
    pub fn warning(code: WarningCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code: DiagnosticCode::Warning(code),
            message: message.into(),
            severity: Severity::Warning,
            span,
            profile: MVP_PROFILE.to_string(),
            notes: Vec::new(),
        }
    }

    /// Create a new info-level diagnostic (uses an error code but at info severity).
    pub fn info(code: ErrorCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code: DiagnosticCode::Error(code),
            message: message.into(),
            severity: Severity::Info,
            span,
            profile: MVP_PROFILE.to_string(),
            notes: Vec::new(),
        }
    }

    /// Add a note to this diagnostic (builder pattern).
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add multiple notes to this diagnostic (builder pattern).
    pub fn with_notes(mut self, notes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.notes.extend(notes.into_iter().map(Into::into));
        self
    }

    /// Returns `true` if this diagnostic is an error.
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Returns `true` if this diagnostic is a warning.
    pub fn is_warning(&self) -> bool {
        self.severity == Severity::Warning
    }

    /// Serialize this diagnostic to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize this diagnostic to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ---------------------------------------------------------------------------
// 7. RuntimeFailure (runtime failure shape)
// ---------------------------------------------------------------------------

/// A runtime failure.
///
/// JSON shape:
/// ```json
/// {
///   "kind": "FAILURE",
///   "code": "CAPABILITY_DENIED",
///   "message": "agent lacks `net` capability",
///   "details": { "capability": "net", "agent": "fetch-agent" }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeFailure {
    /// Always `"FAILURE"`.
    pub kind: String,
    /// The error code identifying this failure.
    pub code: ErrorCode,
    /// Human-readable description of what went wrong.
    pub message: String,
    /// Arbitrary structured details (JSON object).
    pub details: serde_json::Value,
}

/// Constant for the `kind` field of a runtime failure.
pub const FAILURE_KIND: &str = "FAILURE";

impl RuntimeFailure {
    /// Create a new runtime failure with no extra details.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            kind: FAILURE_KIND.to_string(),
            code,
            message: message.into(),
            details: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Create a new runtime failure with structured details.
    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            kind: FAILURE_KIND.to_string(),
            code,
            message: message.into(),
            details,
        }
    }

    /// Serialize this failure to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize this failure to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl std::fmt::Display for RuntimeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.kind, self.code, self.message)
    }
}

impl std::error::Error for RuntimeFailure {}

// ---------------------------------------------------------------------------
// 8. DiagnosticSink
// ---------------------------------------------------------------------------

/// A collector for diagnostics emitted during compilation or analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticSink {
    /// All collected diagnostics, in emission order.
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSink {
    /// Create an empty sink.
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Emit a diagnostic into this sink.
    pub fn emit(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Emit an error diagnostic (convenience).
    pub fn error(&mut self, code: ErrorCode, message: impl Into<String>, span: Span) {
        self.emit(Diagnostic::error(code, message, span));
    }

    /// Emit a warning diagnostic (convenience).
    pub fn warning(&mut self, code: WarningCode, message: impl Into<String>, span: Span) {
        self.emit(Diagnostic::warning(code, message, span));
    }

    /// Returns `true` if any error-level diagnostic has been emitted.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    /// Returns `true` if any warning-level diagnostic has been emitted.
    pub fn has_warnings(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_warning())
    }

    /// Returns the total number of diagnostics collected.
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Returns `true` if no diagnostics have been collected.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Count the number of error-level diagnostics.
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    /// Count the number of warning-level diagnostics.
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_warning()).count()
    }

    /// Return an iterator over all diagnostics.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    /// Return only the error diagnostics.
    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| d.is_error()).collect()
    }

    /// Return only the warning diagnostics.
    pub fn warnings(&self) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| d.is_warning()).collect()
    }

    /// Drain and return all diagnostics, leaving the sink empty.
    pub fn take(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Merge another sink into this one (appends all diagnostics).
    pub fn merge(&mut self, other: DiagnosticSink) {
        self.diagnostics.extend(other.diagnostics);
    }
}

impl IntoIterator for DiagnosticSink {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.diagnostics.into_iter()
    }
}

// ---------------------------------------------------------------------------
// 9. AuditEvent (JSONL schema)
// ---------------------------------------------------------------------------

/// Event types for audit logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditEventType {
    /// An assertion was inserted into the program or agent context.
    AssertInserted,
    /// An assertion check failed at runtime.
    AssertFailed,
    /// A capability request was denied.
    CapabilityDenied,
    /// A checkpoint was created (snapshot of agent state).
    CheckpointCreated,
    /// A checkpoint was restored (rollback of agent state).
    CheckpointRestored,
    /// An error or task was escalated.
    Escalated,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::AssertInserted => "ASSERT_INSERTED",
            Self::AssertFailed => "ASSERT_FAILED",
            Self::CapabilityDenied => "CAPABILITY_DENIED",
            Self::CheckpointCreated => "CHECKPOINT_CREATED",
            Self::CheckpointRestored => "CHECKPOINT_RESTORED",
            Self::Escalated => "ESCALATED",
        };
        write!(f, "{}", s)
    }
}

/// An audit event following the AgentLang JSONL audit schema.
///
/// JSON shape:
/// ```json
/// {
///   "event_id": "550e8400-e29b-41d4-a716-446655440000",
///   "timestamp": "2026-02-24T12:00:00Z",
///   "agent_id": "planner-agent",
///   "task_id": "task-42",
///   "event_type": "CAPABILITY_DENIED",
///   "profile": "mvp-0.1",
///   "details": {}
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique identifier for this event (UUID v4).
    pub event_id: String,
    /// ISO 8601 timestamp of when the event occurred.
    pub timestamp: String,
    /// Identifier of the agent that triggered the event.
    pub agent_id: String,
    /// Identifier of the task context in which the event occurred.
    pub task_id: String,
    /// The type of audit event.
    pub event_type: AuditEventType,
    /// Profile tag — always `"mvp-0.1"` for this version.
    pub profile: String,
    /// Arbitrary structured details (JSON object).
    pub details: serde_json::Value,
}

impl AuditEvent {
    /// Create a new audit event with auto-generated `event_id` and `timestamp`.
    pub fn new(
        agent_id: impl Into<String>,
        task_id: impl Into<String>,
        event_type: AuditEventType,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            agent_id: agent_id.into(),
            task_id: task_id.into(),
            event_type,
            profile: MVP_PROFILE.to_string(),
            details: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Create a new audit event with structured details.
    pub fn with_details(
        agent_id: impl Into<String>,
        task_id: impl Into<String>,
        event_type: AuditEventType,
        details: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            agent_id: agent_id.into(),
            task_id: task_id.into(),
            event_type,
            profile: MVP_PROFILE.to_string(),
            details,
        }
    }

    /// Create an audit event with an explicit `event_id` and `timestamp`
    /// (useful for deterministic testing).
    pub fn with_fixed_id(
        event_id: impl Into<String>,
        timestamp: impl Into<String>,
        agent_id: impl Into<String>,
        task_id: impl Into<String>,
        event_type: AuditEventType,
        details: serde_json::Value,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            timestamp: timestamp.into(),
            agent_id: agent_id.into(),
            task_id: task_id.into(),
            event_type,
            profile: MVP_PROFILE.to_string(),
            details,
        }
    }

    /// Serialize this audit event to a single-line JSON string (JSONL format).
    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize this audit event to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl std::fmt::Display for AuditEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} agent={} task={} type={}",
            self.timestamp, self.event_id, self.agent_id, self.task_id, self.event_type
        )
    }
}

// ---------------------------------------------------------------------------
// 10. Thiserror-based error type for this crate
// ---------------------------------------------------------------------------

/// Crate-level error type for `al-diagnostics` operations.
#[derive(Debug, thiserror::Error)]
pub enum DiagnosticsError {
    /// JSON serialization / deserialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// A generic diagnostic error with a message.
    #[error("{0}")]
    Message(String),
}

/// Convenience result type alias.
pub type DiagnosticsResult<T> = Result<T, DiagnosticsError>;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_default_is_dummy() {
        let span = Span::default();
        assert_eq!(span, Span::dummy());
    }

    #[test]
    fn error_code_display() {
        assert_eq!(ErrorCode::TypeMismatch.to_string(), "TYPE_MISMATCH");
        assert_eq!(
            ErrorCode::FailureArityMismatch.to_string(),
            "FAILURE_ARITY_MISMATCH"
        );
        assert_eq!(ErrorCode::CapabilityDenied.to_string(), "CAPABILITY_DENIED");
    }

    #[test]
    fn error_code_serde_roundtrip() {
        let code = ErrorCode::ParseError;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, r#""PARSE_ERROR""#);
        let back: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, code);
    }

    #[test]
    fn warning_code_serde_roundtrip() {
        let code = WarningCode::CapAliasDeprecated;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, r#""CAP_ALIAS_DEPRECATED""#);
        let back: WarningCode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, code);
    }

    #[test]
    fn diagnostic_error_json_shape() {
        let diag = Diagnostic::error(
            ErrorCode::ParseError,
            "unexpected token `;`",
            Span::new(42, 3, 10, 1),
        )
        .with_note("did you mean `:`?");

        let json_val: serde_json::Value = serde_json::to_value(&diag).unwrap();
        assert_eq!(json_val["code"], "PARSE_ERROR");
        assert_eq!(json_val["message"], "unexpected token `;`");
        assert_eq!(json_val["severity"], "error");
        assert_eq!(json_val["profile"], "mvp-0.1");
        assert_eq!(json_val["span"]["offset"], 42);
        assert_eq!(json_val["span"]["line"], 3);
        assert_eq!(json_val["span"]["column"], 10);
        assert_eq!(json_val["span"]["length"], 1);
        assert_eq!(json_val["notes"][0], "did you mean `:`?");
    }

    #[test]
    fn diagnostic_warning_json_shape() {
        let diag = Diagnostic::warning(
            WarningCode::CapAliasDeprecated,
            "use `network` instead of `net`",
            Span::new(100, 7, 5, 3),
        );

        let json_val: serde_json::Value = serde_json::to_value(&diag).unwrap();
        assert_eq!(json_val["code"], "CAP_ALIAS_DEPRECATED");
        assert_eq!(json_val["severity"], "warning");
        assert_eq!(json_val["profile"], "mvp-0.1");
    }

    #[test]
    fn runtime_failure_json_shape() {
        let failure = RuntimeFailure::with_details(
            ErrorCode::CapabilityDenied,
            "agent lacks `net` capability",
            serde_json::json!({ "capability": "net", "agent": "fetch-agent" }),
        );

        let json_val: serde_json::Value = serde_json::to_value(&failure).unwrap();
        assert_eq!(json_val["kind"], "FAILURE");
        assert_eq!(json_val["code"], "CAPABILITY_DENIED");
        assert_eq!(json_val["message"], "agent lacks `net` capability");
        assert_eq!(json_val["details"]["capability"], "net");
        assert_eq!(json_val["details"]["agent"], "fetch-agent");
    }

    #[test]
    fn diagnostic_sink_collects_and_queries() {
        let mut sink = DiagnosticSink::new();
        assert!(sink.is_empty());
        assert!(!sink.has_errors());

        sink.error(ErrorCode::UnknownIdentifier, "unknown `foo`", Span::dummy());
        sink.warning(
            WarningCode::CapAliasDeprecated,
            "deprecated alias",
            Span::dummy(),
        );

        assert_eq!(sink.len(), 2);
        assert!(sink.has_errors());
        assert!(sink.has_warnings());
        assert_eq!(sink.error_count(), 1);
        assert_eq!(sink.warning_count(), 1);
        assert_eq!(sink.errors().len(), 1);
        assert_eq!(sink.warnings().len(), 1);
    }

    #[test]
    fn diagnostic_sink_merge() {
        let mut sink_a = DiagnosticSink::new();
        let mut sink_b = DiagnosticSink::new();

        sink_a.error(ErrorCode::ParseError, "a", Span::dummy());
        sink_b.error(ErrorCode::TypeMismatch, "b", Span::dummy());

        sink_a.merge(sink_b);
        assert_eq!(sink_a.len(), 2);
    }

    #[test]
    fn diagnostic_sink_take_drains() {
        let mut sink = DiagnosticSink::new();
        sink.error(ErrorCode::ParseError, "oops", Span::dummy());

        let taken = sink.take();
        assert_eq!(taken.len(), 1);
        assert!(sink.is_empty());
    }

    #[test]
    fn audit_event_json_shape() {
        let event = AuditEvent::with_fixed_id(
            "550e8400-e29b-41d4-a716-446655440000",
            "2026-02-24T12:00:00Z",
            "planner-agent",
            "task-42",
            AuditEventType::CapabilityDenied,
            serde_json::json!({}),
        );

        let json_val: serde_json::Value = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json_val["event_id"],
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(json_val["timestamp"], "2026-02-24T12:00:00Z");
        assert_eq!(json_val["agent_id"], "planner-agent");
        assert_eq!(json_val["task_id"], "task-42");
        assert_eq!(json_val["event_type"], "CAPABILITY_DENIED");
        assert_eq!(json_val["profile"], "mvp-0.1");
        assert_eq!(json_val["details"], serde_json::json!({}));
    }

    #[test]
    fn audit_event_all_types_serialize() {
        let types = [
            (AuditEventType::AssertInserted, "ASSERT_INSERTED"),
            (AuditEventType::AssertFailed, "ASSERT_FAILED"),
            (AuditEventType::CapabilityDenied, "CAPABILITY_DENIED"),
            (AuditEventType::CheckpointCreated, "CHECKPOINT_CREATED"),
            (AuditEventType::CheckpointRestored, "CHECKPOINT_RESTORED"),
            (AuditEventType::Escalated, "ESCALATED"),
        ];

        for (event_type, expected_str) in types {
            let json = serde_json::to_string(&event_type).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected_str));
            assert_eq!(event_type.to_string(), expected_str);
        }
    }

    #[test]
    fn audit_event_to_jsonl() {
        let event = AuditEvent::new("agent-1", "task-1", AuditEventType::AssertInserted);
        let jsonl = event.to_jsonl().unwrap();
        // JSONL is a single line — no embedded newlines.
        assert!(!jsonl.contains('\n'));
        // Deserializes back cleanly.
        let back: AuditEvent = serde_json::from_str(&jsonl).unwrap();
        assert_eq!(back.event_type, AuditEventType::AssertInserted);
        assert_eq!(back.profile, "mvp-0.1");
    }

    #[test]
    fn all_error_codes_roundtrip() {
        let codes = [
            ErrorCode::NotImplemented,
            ErrorCode::TypeMismatch,
            ErrorCode::FailureArityMismatch,
            ErrorCode::CapabilityDenied,
            ErrorCode::VcInvalid,
            ErrorCode::AssertionFailed,
            ErrorCode::CheckpointInvalid,
            ErrorCode::Escalated,
            ErrorCode::ParseError,
            ErrorCode::UnknownIdentifier,
            ErrorCode::DuplicateDefinition,
        ];

        for code in codes {
            let json = serde_json::to_string(&code).unwrap();
            let back: ErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(back, code);
        }
    }

    #[test]
    fn diagnostic_into_iterator() {
        let mut sink = DiagnosticSink::new();
        sink.error(ErrorCode::ParseError, "a", Span::dummy());
        sink.error(ErrorCode::TypeMismatch, "b", Span::dummy());

        let collected: Vec<Diagnostic> = sink.into_iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn runtime_failure_display() {
        let failure = RuntimeFailure::new(ErrorCode::AssertionFailed, "x must be > 0");
        let display = format!("{}", failure);
        assert_eq!(display, "[FAILURE] ASSERTION_FAILED: x must be > 0");
    }
}
