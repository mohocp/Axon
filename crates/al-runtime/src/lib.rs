//! # al-runtime
//!
//! Runtime foundations for AgentLang MVP v0.1.
//!
//! Implements the core runtime state machine from the formal semantics:
//!
//! | Symbol | Meaning              | Representation                          |
//! |--------|----------------------|-----------------------------------------|
//! | **H**  | Heap                 | `HashMap<u64, Value>`                   |
//! | **R**  | Registers            | `HashMap<String, Value>`                |
//! | **M**  | Messages/Events      | `VecDeque<Message>`                     |
//! | **K**  | Continuation stack   | `Vec<Continuation>`                     |
//! | **Q**  | Task queue           | `VecDeque<Task>`                        |
//! | **L**  | Lock set             | `HashSet<String>`                       |
//!
//! The runtime provides:
//! - A rich `Value` enum covering all AgentLang runtime values.
//! - Agent lifecycle management with capability enforcement.
//! - Retry, escalation, assertion, fork/join, and checkpoint semantics.
//! - Full audit logging via `al_diagnostics::AuditEvent`.
//! - A tree-walking interpreter for end-to-end program execution (Round 5).

pub mod interpreter;

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use al_capabilities::{Capability, CapabilitySet, check_capability as cap_check};
use al_checkpoint::{Checkpoint, CheckpointMeta, CheckpointStore};
use al_diagnostics::{
    AuditEvent, AuditEventType, ErrorCode, RuntimeFailure, MVP_PROFILE,
};
use chrono::Utc;
use uuid::Uuid;

// ===========================================================================
// Value — the universal runtime value type
// ===========================================================================

/// A runtime value in AgentLang.
///
/// Covers all first-class values that can appear in registers, on the heap,
/// in messages, or as function return values.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A 64-bit signed integer.
    Int(i64),
    /// A 64-bit floating-point number.
    Float(f64),
    /// A UTF-8 string.
    Str(String),
    /// A boolean.
    Bool(bool),
    /// The unit / absence-of-value sentinel.
    None,
    /// An ordered, heterogeneous list.
    List(Vec<Value>),
    /// A string-keyed ordered map.
    Map(BTreeMap<String, Value>),
    /// A successful result wrapper.
    Success(Box<Value>),
    /// A failure result with structured error information.
    Failure {
        /// Machine-readable error code (SCREAMING_SNAKE_CASE).
        code: String,
        /// Human-readable error message.
        message: String,
        /// Arbitrary structured details.
        details: Box<Value>,
    },
    /// A reference to an agent by its unique identifier.
    AgentId(String),
    /// A reference to a task by its unique identifier.
    TaskId(String),
}

impl Value {
    /// Convert this value to a `serde_json::Value` for interop with the
    /// diagnostics and checkpoint crates.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Int(n) => serde_json::Value::Number((*n).into()),
            Value::Float(f) => serde_json::json!(*f),
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::None => serde_json::Value::Null,
            Value::List(items) => {
                serde_json::Value::Array(items.iter().map(|v| v.to_json()).collect())
            }
            Value::Map(map) => {
                let obj: serde_json::Map<String, serde_json::Value> =
                    map.iter().map(|(k, v)| (k.clone(), v.to_json())).collect();
                serde_json::Value::Object(obj)
            }
            Value::Success(inner) => {
                serde_json::json!({ "kind": "SUCCESS", "value": inner.to_json() })
            }
            Value::Failure {
                code,
                message,
                details,
            } => {
                serde_json::json!({
                    "kind": "FAILURE",
                    "code": code,
                    "message": message,
                    "details": details.to_json(),
                })
            }
            Value::AgentId(id) => {
                serde_json::json!({ "kind": "AGENT_ID", "id": id })
            }
            Value::TaskId(id) => {
                serde_json::json!({ "kind": "TASK_ID", "id": id })
            }
        }
    }

    /// Attempt to reconstruct a `Value` from a `serde_json::Value`.
    ///
    /// This is a best-effort conversion used primarily for checkpoint
    /// restore. Round-tripping through JSON is not perfectly lossless for
    /// all variants (e.g. `Success`/`Failure`/`AgentId`/`TaskId` rely on
    /// the `kind` discriminator tag).
    pub fn from_json(json: &serde_json::Value) -> Self {
        match json {
            serde_json::Value::Null => Value::None,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::None
                }
            }
            serde_json::Value::String(s) => Value::Str(s.clone()),
            serde_json::Value::Array(arr) => {
                Value::List(arr.iter().map(Value::from_json).collect())
            }
            serde_json::Value::Object(obj) => {
                // Check for tagged variants.
                if let Some(kind) = obj.get("kind").and_then(|v| v.as_str()) {
                    match kind {
                        "SUCCESS" => {
                            let inner = obj
                                .get("value")
                                .map(Value::from_json)
                                .unwrap_or(Value::None);
                            return Value::Success(Box::new(inner));
                        }
                        "FAILURE" => {
                            let code = obj
                                .get("code")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let message = obj
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let details = obj
                                .get("details")
                                .map(Value::from_json)
                                .unwrap_or(Value::None);
                            return Value::Failure {
                                code,
                                message,
                                details: Box::new(details),
                            };
                        }
                        "AGENT_ID" => {
                            let id = obj
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            return Value::AgentId(id);
                        }
                        "TASK_ID" => {
                            let id = obj
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            return Value::TaskId(id);
                        }
                        _ => {}
                    }
                }
                // Generic map.
                let map: BTreeMap<String, Value> = obj
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::from_json(v)))
                    .collect();
                Value::Map(map)
            }
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::None => write!(f, "none"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Map(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Success(inner) => write!(f, "SUCCESS({})", inner),
            Value::Failure {
                code, message, ..
            } => write!(f, "FAILURE({}, {})", code, message),
            Value::AgentId(id) => write!(f, "AgentId({})", id),
            Value::TaskId(id) => write!(f, "TaskId({})", id),
        }
    }
}

// ===========================================================================
// Formal semantics components: H, M, K, Q, L
// ===========================================================================

/// A message/event in the runtime event queue (**M** in the formal semantics).
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// Unique message identifier.
    pub id: String,
    /// The agent that sent this message.
    pub sender: String,
    /// The intended recipient agent.
    pub recipient: String,
    /// The message payload.
    pub payload: Value,
}

/// A continuation frame on the continuation stack (**K**).
///
/// Continuations capture what to do after the current computation completes.
#[derive(Debug, Clone, PartialEq)]
pub enum Continuation {
    /// Return a value to the caller, restoring the given register snapshot.
    Return {
        /// The register snapshot to restore when this continuation fires.
        saved_registers: HashMap<String, Value>,
    },
    /// A retry continuation: re-execute an operation on failure.
    Retry {
        /// Remaining retry attempts.
        remaining: u64,
        /// A label identifying the retryable operation.
        operation_label: String,
    },
    /// A fork/join barrier: wait for N branches to complete.
    ForkJoinBarrier {
        /// Total number of branches expected.
        total: usize,
        /// Results collected so far.
        completed: Vec<Value>,
    },
}

/// A task in the task queue (**Q**).
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    /// Unique task identifier.
    pub id: String,
    /// The agent this task is assigned to.
    pub agent_id: String,
    /// The task payload / description.
    pub payload: Value,
    /// Task status.
    pub status: TaskStatus,
}

/// Task execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Waiting to be picked up.
    Pending,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed with an error.
    Failed,
}

// ===========================================================================
// AgentStatus & AgentState
// ===========================================================================

/// Lifecycle status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is being initialized (loading config, capabilities, etc.).
    Initializing,
    /// Agent is ready to accept tasks.
    Ready,
    /// Agent is actively executing a task.
    Executing,
    /// Agent state has been checkpointed (persisted for later resume).
    Checkpointed,
    /// Agent execution is suspended (e.g. awaiting external input).
    Suspended,
    /// Agent has encountered an unrecoverable error.
    Failed,
    /// Agent has been gracefully shut down.
    Terminated,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AgentStatus::Initializing => "INITIALIZING",
            AgentStatus::Ready => "READY",
            AgentStatus::Executing => "EXECUTING",
            AgentStatus::Checkpointed => "CHECKPOINTED",
            AgentStatus::Suspended => "SUSPENDED",
            AgentStatus::Failed => "FAILED",
            AgentStatus::Terminated => "TERMINATED",
        };
        write!(f, "{}", s)
    }
}

/// The full runtime state of an agent.
#[derive(Debug, Clone)]
pub struct AgentState {
    /// Unique agent identifier.
    pub id: String,
    /// The set of capabilities granted to this agent.
    pub capabilities: CapabilitySet,
    /// Current lifecycle status.
    pub status: AgentStatus,
    /// Agent-local registers (agent-scoped variable bindings).
    pub registers: HashMap<String, Value>,
}

impl AgentState {
    /// Create a new agent in the `Initializing` status with the given
    /// capability set.
    pub fn new(id: impl Into<String>, capabilities: CapabilitySet) -> Self {
        Self {
            id: id.into(),
            capabilities,
            status: AgentStatus::Initializing,
            registers: HashMap::new(),
        }
    }

    /// Transition the agent to the `Ready` status.
    pub fn mark_ready(&mut self) {
        self.status = AgentStatus::Ready;
    }

    /// Transition the agent to the `Executing` status.
    pub fn mark_executing(&mut self) {
        self.status = AgentStatus::Executing;
    }

    /// Transition the agent to the `Failed` status.
    pub fn mark_failed(&mut self) {
        self.status = AgentStatus::Failed;
    }

    /// Transition the agent to the `Terminated` status.
    pub fn mark_terminated(&mut self) {
        self.status = AgentStatus::Terminated;
    }
}

// ===========================================================================
// Runtime — the top-level execution engine
// ===========================================================================

/// The AgentLang runtime engine.
///
/// Encapsulates the six components of the formal semantics (H, R, M, K, Q, L)
/// together with agent management, audit logging, and checkpoint support.
pub struct Runtime {
    // -- Formal semantics state -----------------------------------------------

    /// **H** — Heap: values addressable by numeric address.
    pub heap: HashMap<u64, Value>,
    /// Next heap address to allocate.
    next_heap_addr: u64,

    /// **R** — Registers: named values in the current scope.
    pub registers: HashMap<String, Value>,

    /// **M** — Messages/Events queue.
    pub messages: VecDeque<Message>,

    /// **K** — Continuation stack.
    pub continuations: Vec<Continuation>,

    /// **Q** — Task queue.
    pub task_queue: VecDeque<Task>,

    /// **L** — Lock set: currently held lock identifiers.
    pub locks: HashSet<String>,

    // -- Agent management -----------------------------------------------------

    /// All registered agents, keyed by agent ID.
    pub agents: HashMap<String, AgentState>,

    // -- Audit & diagnostics --------------------------------------------------

    /// Append-only audit log (JSONL events).
    pub audit_log: Vec<AuditEvent>,

    /// The current task ID used as context for audit events.
    /// Defaults to `"runtime"` when no specific task is active.
    pub current_task_id: String,

    // -- Checkpoint -----------------------------------------------------------

    /// In-memory checkpoint store.
    pub checkpoint_store: CheckpointStore,

    // -- Capability registry (global overrides) --------------------------------

    /// Per-agent capability overrides. If an agent appears here, these
    /// capabilities are used *instead of* the agent's own `AgentState.capabilities`.
    /// This allows the runtime to dynamically grant/revoke capabilities.
    pub capability_overrides: HashMap<String, CapabilitySet>,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("heap_size", &self.heap.len())
            .field("registers", &self.registers.len())
            .field("messages", &self.messages.len())
            .field("continuations", &self.continuations.len())
            .field("task_queue", &self.task_queue.len())
            .field("locks", &self.locks.len())
            .field("agents", &self.agents.len())
            .field("audit_log", &self.audit_log.len())
            .finish()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    // =======================================================================
    // Construction
    // =======================================================================

    /// Create a new, empty runtime.
    pub fn new() -> Self {
        Self {
            heap: HashMap::new(),
            next_heap_addr: 1,
            registers: HashMap::new(),
            messages: VecDeque::new(),
            continuations: Vec::new(),
            task_queue: VecDeque::new(),
            locks: HashSet::new(),
            agents: HashMap::new(),
            audit_log: Vec::new(),
            current_task_id: "runtime".to_string(),
            checkpoint_store: CheckpointStore::new(),
            capability_overrides: HashMap::new(),
        }
    }

    // =======================================================================
    // Heap operations (H)
    // =======================================================================

    /// Allocate a value on the heap and return its address.
    pub fn heap_alloc(&mut self, value: Value) -> u64 {
        let addr = self.next_heap_addr;
        self.next_heap_addr += 1;
        self.heap.insert(addr, value);
        addr
    }

    /// Read a value from the heap by address.
    pub fn heap_get(&self, addr: u64) -> Option<&Value> {
        self.heap.get(&addr)
    }

    /// Write a value to an existing heap address. Returns the old value if present.
    pub fn heap_set(&mut self, addr: u64, value: Value) -> Option<Value> {
        self.heap.insert(addr, value)
    }

    // =======================================================================
    // Register operations (R)
    // =======================================================================

    /// Set a register to the given value.
    pub fn reg_set(&mut self, name: impl Into<String>, value: Value) {
        self.registers.insert(name.into(), value);
    }

    /// Get a register value by name.
    pub fn reg_get(&self, name: &str) -> Option<&Value> {
        self.registers.get(name)
    }

    /// Remove a register binding.
    pub fn reg_remove(&mut self, name: &str) -> Option<Value> {
        self.registers.remove(name)
    }

    // =======================================================================
    // Message operations (M)
    // =======================================================================

    /// Enqueue a message.
    pub fn send_message(&mut self, sender: &str, recipient: &str, payload: Value) {
        self.messages.push_back(Message {
            id: Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            payload,
        });
    }

    /// Dequeue the next message, if any.
    pub fn recv_message(&mut self) -> Option<Message> {
        self.messages.pop_front()
    }

    // =======================================================================
    // Lock operations (L)
    // =======================================================================

    /// Attempt to acquire a lock. Returns `true` if newly acquired.
    pub fn lock_acquire(&mut self, name: impl Into<String>) -> bool {
        self.locks.insert(name.into())
    }

    /// Release a lock. Returns `true` if the lock was held.
    pub fn lock_release(&mut self, name: &str) -> bool {
        self.locks.remove(name)
    }

    /// Check whether a lock is currently held.
    pub fn lock_held(&self, name: &str) -> bool {
        self.locks.contains(name)
    }

    // =======================================================================
    // Agent management
    // =======================================================================

    /// Register a new agent with the runtime.
    pub fn register_agent(&mut self, id: impl Into<String>, capabilities: CapabilitySet) -> String {
        let id = id.into();
        let agent = AgentState::new(id.clone(), capabilities);
        self.agents.insert(id.clone(), agent);
        id
    }

    /// Retrieve an agent's state.
    pub fn get_agent(&self, agent_id: &str) -> Option<&AgentState> {
        self.agents.get(agent_id)
    }

    /// Retrieve a mutable reference to an agent's state.
    pub fn get_agent_mut(&mut self, agent_id: &str) -> Option<&mut AgentState> {
        self.agents.get_mut(agent_id)
    }

    /// Resolve the effective capability set for an agent.
    ///
    /// If a capability override exists in `self.capability_overrides`, that
    /// set is used. Otherwise the agent's own `AgentState.capabilities` is
    /// returned.
    fn effective_capabilities(&self, agent_id: &str) -> Option<CapabilitySet> {
        if let Some(overrides) = self.capability_overrides.get(agent_id) {
            return Some(overrides.clone());
        }
        self.agents.get(agent_id).map(|a| a.capabilities.clone())
    }

    // =======================================================================
    // Audit helpers
    // =======================================================================

    /// Emit an audit event and append it to the audit log.
    fn emit_audit(
        &mut self,
        agent_id: &str,
        event_type: AuditEventType,
        details: serde_json::Value,
    ) {
        let event = AuditEvent::with_details(
            agent_id,
            &self.current_task_id,
            event_type,
            details,
        );
        self.audit_log.push(event);
    }

    // =======================================================================
    // 1. execute_retry
    // =======================================================================

    /// Execute an operation with up to `count` additional retry attempts.
    ///
    /// The `operation` closure is called once initially. If it returns `Err`,
    /// it is retried up to `count` more times. If all attempts (1 + count)
    /// fail, the final `RuntimeFailure` is returned.
    ///
    /// On success at any attempt, the `Value` is returned immediately.
    pub fn execute_retry<F>(
        &mut self,
        count: u64,
        mut operation: F,
    ) -> Result<Value, RuntimeFailure>
    where
        F: FnMut(&mut Self) -> Result<Value, RuntimeFailure>,
    {
        let total_attempts = count + 1; // 1 initial + count retries
        let mut last_error: Option<RuntimeFailure> = None;

        for attempt in 0..total_attempts {
            match operation(self) {
                Ok(value) => return Ok(value),
                Err(e) => {
                    last_error = Some(e);
                    // Continue to next attempt if retries remain.
                    if attempt < total_attempts - 1 {
                        continue;
                    }
                }
            }
        }

        // All attempts exhausted — return the last error.
        Err(last_error.unwrap_or_else(|| {
            RuntimeFailure::new(
                ErrorCode::NotImplemented,
                "retry exhausted with no error captured",
            )
        }))
    }

    // =======================================================================
    // 2. execute_escalate
    // =======================================================================

    /// Escalate an error from an agent context.
    ///
    /// Emits an `ESCALATED` audit event and returns a `RuntimeFailure` with
    /// the `Escalated` error code. The optional `message` provides additional
    /// context; if `None`, a default message is used.
    pub fn execute_escalate(
        &mut self,
        message: Option<String>,
        agent_id: &str,
    ) -> RuntimeFailure {
        let msg = message.unwrap_or_else(|| {
            format!("agent '{}' escalated", agent_id)
        });

        self.emit_audit(
            agent_id,
            AuditEventType::Escalated,
            serde_json::json!({
                "message": msg,
            }),
        );

        // Mark the agent as failed if it exists.
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.mark_failed();
        }

        RuntimeFailure::with_details(
            ErrorCode::Escalated,
            msg.clone(),
            serde_json::json!({
                "agent_id": agent_id,
                "message": msg,
            }),
        )
    }

    // =======================================================================
    // 3. check_capability
    // =======================================================================

    /// Check whether an agent holds a required capability.
    ///
    /// If the agent does not hold the capability, a `CAPABILITY_DENIED`
    /// audit event is emitted and a `RuntimeFailure` is returned.
    pub fn check_capability(
        &mut self,
        agent_id: &str,
        required: Capability,
    ) -> Result<(), RuntimeFailure> {
        let caps = self.effective_capabilities(agent_id).unwrap_or_default();

        match cap_check(&caps, required) {
            Ok(()) => Ok(()),
            Err(cap_err) => {
                self.emit_audit(
                    agent_id,
                    AuditEventType::CapabilityDenied,
                    serde_json::json!({
                        "required": required.canonical_name(),
                        "error": cap_err.to_string(),
                    }),
                );

                Err(RuntimeFailure::with_details(
                    ErrorCode::CapabilityDenied,
                    format!(
                        "agent '{}' lacks required capability '{}'",
                        agent_id,
                        required.canonical_name()
                    ),
                    serde_json::json!({
                        "agent_id": agent_id,
                        "required": required.canonical_name(),
                    }),
                ))
            }
        }
    }

    // =======================================================================
    // 4. create_checkpoint
    // =======================================================================

    /// Create a checkpoint of the given agent's state.
    ///
    /// Emits a `CHECKPOINT_CREATED` audit event and returns the checkpoint ID.
    pub fn create_checkpoint(&mut self, agent_id: &str, state: Value) -> String {
        let checkpoint_id = Uuid::new_v4().to_string();
        let json_state = state.to_json();

        // Compute a simple hash of the serialized state for integrity.
        let state_str = serde_json::to_string(&json_state).unwrap_or_default();
        let hash = simple_hash(&state_str);

        let checkpoint = Checkpoint {
            meta: CheckpointMeta {
                checkpoint_id: checkpoint_id.clone(),
                created_at: Utc::now().to_rfc3339(),
                profile: MVP_PROFILE.to_string(),
                schema_version: "1".to_string(),
                hash,
            },
            state: json_state,
            effect_journal: vec![],
        };

        self.checkpoint_store.create(checkpoint);

        // Mark the agent as checkpointed if it exists.
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = AgentStatus::Checkpointed;
        }

        self.emit_audit(
            agent_id,
            AuditEventType::CheckpointCreated,
            serde_json::json!({
                "checkpoint_id": checkpoint_id,
            }),
        );

        checkpoint_id
    }

    // =======================================================================
    // 5. restore_checkpoint
    // =======================================================================

    /// Restore a checkpoint by ID.
    ///
    /// Validates the checkpoint profile and hash integrity, then emits a
    /// `CHECKPOINT_RESTORED` audit event.
    pub fn restore_checkpoint(
        &mut self,
        checkpoint_id: &str,
    ) -> Result<Value, RuntimeFailure> {
        // Validate profile.
        self.checkpoint_store
            .validate(checkpoint_id, MVP_PROFILE)?;

        // Restore the checkpoint.
        let checkpoint = self.checkpoint_store.restore(checkpoint_id)?;

        // Validate hash integrity.
        let state_str =
            serde_json::to_string(&checkpoint.state).unwrap_or_default();
        let computed_hash = simple_hash(&state_str);
        if computed_hash != checkpoint.meta.hash {
            return Err(RuntimeFailure::with_details(
                ErrorCode::CheckpointInvalid,
                format!(
                    "checkpoint '{}' hash mismatch: expected '{}', computed '{}'",
                    checkpoint_id, checkpoint.meta.hash, computed_hash
                ),
                serde_json::json!({
                    "checkpoint_id": checkpoint_id,
                    "expected_hash": checkpoint.meta.hash,
                    "computed_hash": computed_hash,
                }),
            ));
        }

        let value = Value::from_json(&checkpoint.state);

        // Determine agent_id from context (use "runtime" if not determinable).
        // In a real runtime the checkpoint would store the agent_id.
        let agent_id = "runtime";

        self.emit_audit(
            agent_id,
            AuditEventType::CheckpointRestored,
            serde_json::json!({
                "checkpoint_id": checkpoint_id,
            }),
        );

        Ok(value)
    }

    // =======================================================================
    // 6. execute_assert
    // =======================================================================

    /// Execute a runtime assertion.
    ///
    /// If `condition` is `false`, emits an `ASSERT_FAILED` audit event and
    /// returns an `ASSERTION_FAILED` failure. If `true`, succeeds silently.
    ///
    /// # Parameters
    ///
    /// * `condition` — the boolean predicate to check.
    /// * `vc_id` — the verification condition identifier (for traceability).
    /// * `solver_reason` — the solver's explanation of why this assertion
    ///   was required or why it failed.
    pub fn execute_assert(
        &mut self,
        condition: bool,
        vc_id: &str,
        solver_reason: &str,
    ) -> Result<(), RuntimeFailure> {
        if condition {
            return Ok(());
        }

        self.emit_audit(
            "runtime",
            AuditEventType::AssertFailed,
            serde_json::json!({
                "vc_id": vc_id,
                "solver_reason": solver_reason,
            }),
        );

        Err(RuntimeFailure::with_details(
            ErrorCode::AssertionFailed,
            format!("assertion failed: vc_id={}, reason={}", vc_id, solver_reason),
            serde_json::json!({
                "vc_id": vc_id,
                "solver_reason": solver_reason,
            }),
        ))
    }

    // =======================================================================
    // 7. insert_runtime_assert
    // =======================================================================

    /// Insert a runtime assertion marker.
    ///
    /// This records that a verification condition has been inserted into the
    /// execution trace. It emits an `ASSERT_INSERTED` audit event but does
    /// not evaluate any condition — evaluation happens later via
    /// `execute_assert`.
    pub fn insert_runtime_assert(&mut self, vc_id: &str, solver_reason: &str) {
        self.emit_audit(
            "runtime",
            AuditEventType::AssertInserted,
            serde_json::json!({
                "vc_id": vc_id,
                "solver_reason": solver_reason,
            }),
        );
    }

    // =======================================================================
    // 8. execute_fork_join
    // =======================================================================

    /// Execute multiple branches in fork/join mode (ALL_COMPLETE semantics).
    ///
    /// All branches must succeed. The closures are executed sequentially
    /// (MVP v0.1 does not require parallel execution). If any branch
    /// fails, execution halts immediately and the failure propagates.
    ///
    /// Returns a vector of `Value` results, one per branch, in order.
    pub fn execute_fork_join<F>(
        &mut self,
        branches: Vec<F>,
    ) -> Result<Vec<Value>, RuntimeFailure>
    where
        F: FnOnce(&mut Self) -> Result<Value, RuntimeFailure>,
    {
        let mut results = Vec::with_capacity(branches.len());

        for branch in branches {
            match branch(self) {
                Ok(value) => results.push(value),
                Err(failure) => {
                    // ALL_COMPLETE: any failure propagates immediately.
                    return Err(failure);
                }
            }
        }

        Ok(results)
    }

    // =======================================================================
    // Task queue operations (Q)
    // =======================================================================

    /// Enqueue a task.
    pub fn enqueue_task(&mut self, agent_id: &str, payload: Value) -> String {
        let task_id = Uuid::new_v4().to_string();
        self.task_queue.push_back(Task {
            id: task_id.clone(),
            agent_id: agent_id.to_string(),
            payload,
            status: TaskStatus::Pending,
        });
        task_id
    }

    /// Dequeue the next pending task, if any.
    pub fn dequeue_task(&mut self) -> Option<Task> {
        self.task_queue.pop_front()
    }
}

// ===========================================================================
// Utility: simple hash function for checkpoint integrity
// ===========================================================================

/// A simple, deterministic hash for checkpoint integrity validation.
///
/// Uses a basic DJB2-style hash. This is sufficient for MVP v0.1 integrity
/// checks but should be replaced with a cryptographic hash in production.
fn simple_hash(data: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in data.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u64::from(byte));
    }
    format!("{:016x}", hash)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use al_capabilities::Capability;

    // -----------------------------------------------------------------------
    // Value tests
    // -----------------------------------------------------------------------

    #[test]
    fn value_display() {
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::Float(3.14).to_string(), "3.14");
        assert_eq!(Value::Str("hello".into()).to_string(), "\"hello\"");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::None.to_string(), "none");
    }

    #[test]
    fn value_json_roundtrip() {
        let values = vec![
            Value::Int(42),
            Value::Float(3.14),
            Value::Str("hello".into()),
            Value::Bool(true),
            Value::None,
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::Map({
                let mut m = BTreeMap::new();
                m.insert("a".into(), Value::Int(1));
                m
            }),
            Value::Success(Box::new(Value::Int(99))),
            Value::Failure {
                code: "TEST".into(),
                message: "test failure".into(),
                details: Box::new(Value::None),
            },
            Value::AgentId("agent-1".into()),
            Value::TaskId("task-1".into()),
        ];

        for val in &values {
            let json = val.to_json();
            let restored = Value::from_json(&json);
            assert_eq!(val, &restored, "round-trip failed for {:?}", val);
        }
    }

    // -----------------------------------------------------------------------
    // Heap tests
    // -----------------------------------------------------------------------

    #[test]
    fn heap_alloc_and_get() {
        let mut rt = Runtime::new();
        let addr = rt.heap_alloc(Value::Int(42));
        assert_eq!(rt.heap_get(addr), Some(&Value::Int(42)));
    }

    // -----------------------------------------------------------------------
    // Register tests
    // -----------------------------------------------------------------------

    #[test]
    fn register_set_get_remove() {
        let mut rt = Runtime::new();
        rt.reg_set("x", Value::Int(10));
        assert_eq!(rt.reg_get("x"), Some(&Value::Int(10)));
        let removed = rt.reg_remove("x");
        assert_eq!(removed, Some(Value::Int(10)));
        assert_eq!(rt.reg_get("x"), None);
    }

    // -----------------------------------------------------------------------
    // Message tests
    // -----------------------------------------------------------------------

    #[test]
    fn message_send_recv() {
        let mut rt = Runtime::new();
        rt.send_message("a", "b", Value::Str("hello".into()));
        let msg = rt.recv_message().unwrap();
        assert_eq!(msg.sender, "a");
        assert_eq!(msg.recipient, "b");
        assert_eq!(msg.payload, Value::Str("hello".into()));
        assert!(rt.recv_message().is_none());
    }

    // -----------------------------------------------------------------------
    // Lock tests
    // -----------------------------------------------------------------------

    #[test]
    fn lock_acquire_release() {
        let mut rt = Runtime::new();
        assert!(rt.lock_acquire("my-lock"));
        assert!(rt.lock_held("my-lock"));
        // Second acquire returns false (already held).
        assert!(!rt.lock_acquire("my-lock"));
        assert!(rt.lock_release("my-lock"));
        assert!(!rt.lock_held("my-lock"));
    }

    // -----------------------------------------------------------------------
    // Agent management tests
    // -----------------------------------------------------------------------

    #[test]
    fn agent_lifecycle() {
        let mut rt = Runtime::new();
        let mut caps = CapabilitySet::empty();
        caps.insert(Capability::FileRead);

        let id = rt.register_agent("agent-1", caps);
        assert_eq!(id, "agent-1");

        let agent = rt.get_agent("agent-1").unwrap();
        assert_eq!(agent.status, AgentStatus::Initializing);
        assert!(agent.capabilities.contains(&Capability::FileRead));

        rt.get_agent_mut("agent-1").unwrap().mark_ready();
        assert_eq!(rt.get_agent("agent-1").unwrap().status, AgentStatus::Ready);
    }

    // -----------------------------------------------------------------------
    // 1. Retry exhaustion
    // -----------------------------------------------------------------------

    #[test]
    fn retry_succeeds_on_last_attempt() {
        let mut rt = Runtime::new();
        let mut attempt = 0u64;

        let result = rt.execute_retry(2, |_rt| {
            attempt += 1;
            if attempt < 3 {
                Err(RuntimeFailure::new(
                    ErrorCode::NotImplemented,
                    format!("attempt {}", attempt),
                ))
            } else {
                Ok(Value::Str("success".into()))
            }
        });

        assert_eq!(result.unwrap(), Value::Str("success".into()));
        assert_eq!(attempt, 3); // 1 initial + 2 retries = 3 total
    }

    #[test]
    fn retry_exhaustion_returns_last_error() {
        let mut rt = Runtime::new();
        let mut attempt = 0u64;

        let result = rt.execute_retry(2, |_rt| {
            attempt += 1;
            Err(RuntimeFailure::new(
                ErrorCode::NotImplemented,
                format!("failed attempt {}", attempt),
            ))
        });

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::NotImplemented);
        assert_eq!(err.message, "failed attempt 3"); // 3 total attempts
        assert_eq!(attempt, 3);
    }

    #[test]
    fn retry_zero_means_single_attempt() {
        let mut rt = Runtime::new();
        let mut attempt = 0u64;

        let result = rt.execute_retry(0, |_rt| {
            attempt += 1;
            Err(RuntimeFailure::new(
                ErrorCode::NotImplemented,
                "single attempt",
            ))
        });

        assert!(result.is_err());
        assert_eq!(attempt, 1);
    }

    #[test]
    fn retry_succeeds_immediately() {
        let mut rt = Runtime::new();
        let mut attempt = 0u64;

        let result = rt.execute_retry(5, |_rt| {
            attempt += 1;
            Ok(Value::Int(42))
        });

        assert_eq!(result.unwrap(), Value::Int(42));
        assert_eq!(attempt, 1); // Only one attempt needed.
    }

    // -----------------------------------------------------------------------
    // 2. Escalation semantics
    // -----------------------------------------------------------------------

    #[test]
    fn escalation_emits_audit_event_and_returns_failure() {
        let mut rt = Runtime::new();
        rt.register_agent("agent-1", CapabilitySet::empty());
        rt.get_agent_mut("agent-1").unwrap().mark_ready();

        let failure = rt.execute_escalate(
            Some("something went wrong".into()),
            "agent-1",
        );

        // Verify the failure.
        assert_eq!(failure.code, ErrorCode::Escalated);
        assert!(failure.message.contains("something went wrong"));

        // Verify the audit log.
        assert_eq!(rt.audit_log.len(), 1);
        assert_eq!(rt.audit_log[0].event_type, AuditEventType::Escalated);
        assert_eq!(rt.audit_log[0].agent_id, "agent-1");
        assert_eq!(rt.audit_log[0].details["message"], "something went wrong");

        // Verify the agent was marked as failed.
        assert_eq!(
            rt.get_agent("agent-1").unwrap().status,
            AgentStatus::Failed
        );
    }

    #[test]
    fn escalation_with_default_message() {
        let mut rt = Runtime::new();
        rt.register_agent("agent-x", CapabilitySet::empty());

        let failure = rt.execute_escalate(None, "agent-x");
        assert_eq!(failure.code, ErrorCode::Escalated);
        assert!(failure.message.contains("agent-x"));
        assert!(failure.message.contains("escalated"));
    }

    // -----------------------------------------------------------------------
    // 3. Capability checks
    // -----------------------------------------------------------------------

    #[test]
    fn capability_check_succeeds_when_granted() {
        let mut rt = Runtime::new();
        let mut caps = CapabilitySet::empty();
        caps.insert(Capability::FileRead);
        caps.insert(Capability::ApiCall);
        rt.register_agent("agent-1", caps);

        assert!(rt.check_capability("agent-1", Capability::FileRead).is_ok());
        assert!(rt.check_capability("agent-1", Capability::ApiCall).is_ok());
        assert_eq!(rt.audit_log.len(), 0); // No audit events for success.
    }

    #[test]
    fn capability_check_fails_and_emits_audit() {
        let mut rt = Runtime::new();
        let caps = CapabilitySet::empty();
        rt.register_agent("agent-1", caps);

        let result = rt.check_capability("agent-1", Capability::DbWrite);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::CapabilityDenied);
        assert!(err.message.contains("DB_WRITE"));

        // Verify audit event.
        assert_eq!(rt.audit_log.len(), 1);
        assert_eq!(
            rt.audit_log[0].event_type,
            AuditEventType::CapabilityDenied
        );
        assert_eq!(rt.audit_log[0].details["required"], "DB_WRITE");
    }

    #[test]
    fn capability_check_with_override() {
        let mut rt = Runtime::new();
        // Agent has no capabilities by default.
        rt.register_agent("agent-1", CapabilitySet::empty());

        // Override grants LlmInfer.
        let mut override_caps = CapabilitySet::empty();
        override_caps.insert(Capability::LlmInfer);
        rt.capability_overrides
            .insert("agent-1".to_string(), override_caps);

        // Check should succeed via override.
        assert!(rt
            .check_capability("agent-1", Capability::LlmInfer)
            .is_ok());
    }

    #[test]
    fn capability_check_unknown_agent_fails() {
        let mut rt = Runtime::new();
        let result = rt.check_capability("nonexistent", Capability::FileRead);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ErrorCode::CapabilityDenied);
    }

    // -----------------------------------------------------------------------
    // 4. Assert failure
    // -----------------------------------------------------------------------

    #[test]
    fn assert_true_succeeds() {
        let mut rt = Runtime::new();
        let result = rt.execute_assert(true, "vc-001", "x > 0");
        assert!(result.is_ok());
        assert_eq!(rt.audit_log.len(), 0); // No audit events for success.
    }

    #[test]
    fn assert_false_fails_and_emits_audit() {
        let mut rt = Runtime::new();
        let result = rt.execute_assert(false, "vc-002", "balance >= 0");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::AssertionFailed);
        assert!(err.message.contains("vc-002"));
        assert!(err.message.contains("balance >= 0"));

        // Verify audit event.
        assert_eq!(rt.audit_log.len(), 1);
        assert_eq!(rt.audit_log[0].event_type, AuditEventType::AssertFailed);
        assert_eq!(rt.audit_log[0].details["vc_id"], "vc-002");
        assert_eq!(rt.audit_log[0].details["solver_reason"], "balance >= 0");
    }

    #[test]
    fn insert_runtime_assert_emits_audit() {
        let mut rt = Runtime::new();
        rt.insert_runtime_assert("vc-010", "invariant: list non-empty");

        assert_eq!(rt.audit_log.len(), 1);
        assert_eq!(rt.audit_log[0].event_type, AuditEventType::AssertInserted);
        assert_eq!(rt.audit_log[0].details["vc_id"], "vc-010");
        assert_eq!(
            rt.audit_log[0].details["solver_reason"],
            "invariant: list non-empty"
        );
    }

    // -----------------------------------------------------------------------
    // 5. Checkpoint create/restore
    // -----------------------------------------------------------------------

    #[test]
    fn checkpoint_create_and_restore() {
        let mut rt = Runtime::new();
        rt.register_agent("agent-1", CapabilitySet::empty());

        let state = Value::Map({
            let mut m = BTreeMap::new();
            m.insert("counter".into(), Value::Int(42));
            m.insert("name".into(), Value::Str("test".into()));
            m
        });

        let cp_id = rt.create_checkpoint("agent-1", state.clone());
        assert!(!cp_id.is_empty());

        // Verify agent status.
        assert_eq!(
            rt.get_agent("agent-1").unwrap().status,
            AgentStatus::Checkpointed
        );

        // Verify CHECKPOINT_CREATED audit event.
        assert_eq!(rt.audit_log.len(), 1);
        assert_eq!(
            rt.audit_log[0].event_type,
            AuditEventType::CheckpointCreated
        );

        // Restore.
        let restored = rt.restore_checkpoint(&cp_id).unwrap();
        assert_eq!(restored, state);

        // Verify CHECKPOINT_RESTORED audit event.
        assert_eq!(rt.audit_log.len(), 2);
        assert_eq!(
            rt.audit_log[1].event_type,
            AuditEventType::CheckpointRestored
        );
    }

    #[test]
    fn checkpoint_restore_missing_fails() {
        let mut rt = Runtime::new();
        let result = rt.restore_checkpoint("nonexistent-id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ErrorCode::CheckpointInvalid);
    }

    #[test]
    fn checkpoint_roundtrip_preserves_complex_state() {
        let mut rt = Runtime::new();
        rt.register_agent("a", CapabilitySet::empty());

        let state = Value::List(vec![
            Value::Int(1),
            Value::Str("two".into()),
            Value::Bool(false),
            Value::Map({
                let mut m = BTreeMap::new();
                m.insert("nested".into(), Value::Float(2.718));
                m
            }),
        ]);

        let cp_id = rt.create_checkpoint("a", state.clone());
        let restored = rt.restore_checkpoint(&cp_id).unwrap();
        assert_eq!(restored, state);
    }

    // -----------------------------------------------------------------------
    // 6. Fork/join ALL_COMPLETE
    // -----------------------------------------------------------------------

    #[test]
    fn fork_join_all_succeed() {
        let mut rt = Runtime::new();

        let branches: Vec<Box<dyn FnOnce(&mut Runtime) -> Result<Value, RuntimeFailure>>> = vec![
            Box::new(|_rt| Ok(Value::Int(1))),
            Box::new(|_rt| Ok(Value::Int(2))),
            Box::new(|_rt| Ok(Value::Int(3))),
        ];

        let results = rt.execute_fork_join(branches).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Value::Int(1));
        assert_eq!(results[1], Value::Int(2));
        assert_eq!(results[2], Value::Int(3));
    }

    #[test]
    fn fork_join_failure_propagates() {
        let mut rt = Runtime::new();

        let branches: Vec<Box<dyn FnOnce(&mut Runtime) -> Result<Value, RuntimeFailure>>> = vec![
            Box::new(|_rt| Ok(Value::Int(1))),
            Box::new(|_rt| {
                Err(RuntimeFailure::new(
                    ErrorCode::NotImplemented,
                    "branch 2 failed",
                ))
            }),
            Box::new(|_rt| Ok(Value::Int(3))), // Should not execute.
        ];

        let result = rt.execute_fork_join(branches);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::NotImplemented);
        assert_eq!(err.message, "branch 2 failed");
    }

    #[test]
    fn fork_join_first_branch_fails() {
        let mut rt = Runtime::new();

        let branches: Vec<Box<dyn FnOnce(&mut Runtime) -> Result<Value, RuntimeFailure>>> = vec![
            Box::new(|_rt| {
                Err(RuntimeFailure::new(
                    ErrorCode::AssertionFailed,
                    "immediate failure",
                ))
            }),
            Box::new(|_rt| Ok(Value::Int(2))), // Should not execute.
        ];

        let result = rt.execute_fork_join(branches);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ErrorCode::AssertionFailed);
    }

    #[test]
    fn fork_join_empty_branches_returns_empty() {
        let mut rt = Runtime::new();
        let branches: Vec<Box<dyn FnOnce(&mut Runtime) -> Result<Value, RuntimeFailure>>> =
            vec![];
        let results = rt.execute_fork_join(branches).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn fork_join_branches_can_modify_runtime() {
        let mut rt = Runtime::new();

        let branches: Vec<Box<dyn FnOnce(&mut Runtime) -> Result<Value, RuntimeFailure>>> = vec![
            Box::new(|rt| {
                rt.reg_set("x", Value::Int(10));
                Ok(Value::Int(10))
            }),
            Box::new(|rt| {
                // Should see x=10 set by previous branch (sequential execution).
                let x = rt.reg_get("x").cloned().unwrap_or(Value::None);
                Ok(x)
            }),
        ];

        let results = rt.execute_fork_join(branches).unwrap();
        assert_eq!(results[0], Value::Int(10));
        assert_eq!(results[1], Value::Int(10)); // Sees the register from branch 0.
    }

    // -----------------------------------------------------------------------
    // Task queue tests
    // -----------------------------------------------------------------------

    #[test]
    fn task_queue_enqueue_dequeue() {
        let mut rt = Runtime::new();
        let tid = rt.enqueue_task("agent-1", Value::Str("do work".into()));
        assert!(!tid.is_empty());

        let task = rt.dequeue_task().unwrap();
        assert_eq!(task.agent_id, "agent-1");
        assert_eq!(task.payload, Value::Str("do work".into()));
        assert_eq!(task.status, TaskStatus::Pending);

        assert!(rt.dequeue_task().is_none());
    }

    // -----------------------------------------------------------------------
    // Integration: retry + escalation
    // -----------------------------------------------------------------------

    #[test]
    fn retry_exhaustion_then_escalate() {
        let mut rt = Runtime::new();
        rt.register_agent("agent-1", CapabilitySet::empty());

        // Retry 2 times (3 total attempts), all fail.
        let retry_result = rt.execute_retry(2, |_rt| {
            Err(RuntimeFailure::new(
                ErrorCode::NotImplemented,
                "operation failed",
            ))
        });

        assert!(retry_result.is_err());

        // Escalate after retry exhaustion.
        let failure = rt.execute_escalate(
            Some("all retries exhausted, escalating".into()),
            "agent-1",
        );
        assert_eq!(failure.code, ErrorCode::Escalated);

        // Verify agent is failed.
        assert_eq!(
            rt.get_agent("agent-1").unwrap().status,
            AgentStatus::Failed
        );

        // Verify audit log has the escalation event.
        assert_eq!(rt.audit_log.len(), 1);
        assert_eq!(rt.audit_log[0].event_type, AuditEventType::Escalated);
    }

    // -----------------------------------------------------------------------
    // Integration: capability check + escalation
    // -----------------------------------------------------------------------

    #[test]
    fn capability_denied_then_escalate() {
        let mut rt = Runtime::new();
        rt.register_agent("agent-1", CapabilitySet::empty());

        // Capability check fails.
        let cap_result = rt.check_capability("agent-1", Capability::DbWrite);
        assert!(cap_result.is_err());

        // Escalate.
        let failure = rt.execute_escalate(
            Some("required DB_WRITE not available".into()),
            "agent-1",
        );
        assert_eq!(failure.code, ErrorCode::Escalated);

        // Audit log: CAPABILITY_DENIED + ESCALATED.
        assert_eq!(rt.audit_log.len(), 2);
        assert_eq!(
            rt.audit_log[0].event_type,
            AuditEventType::CapabilityDenied
        );
        assert_eq!(rt.audit_log[1].event_type, AuditEventType::Escalated);
    }
}
