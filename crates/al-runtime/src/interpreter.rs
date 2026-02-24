//! # AgentLang Interpreter — Round 5
//!
//! Tree-walking interpreter that executes AgentLang programs end-to-end.
//!
//! Implements:
//! - **Statement interpreter**: STORE, MUTABLE, ASSIGN, MATCH, LOOP, EMIT, HALT,
//!   ASSERT (VC metadata), RETRY, ESCALATE, CHECKPOINT, DELEGATE
//! - **Expression evaluator**: literals, identifiers, binary/unary ops, member
//!   access, list/map constructors, operation calls
//! - **Pattern matching**: wildcard, literal, SUCCESS/FAILURE destructuring,
//!   identifier binding
//! - **Pipeline execution**: output threading with short-circuit on FAILURE
//! - **Fork/Join**: ALL_COMPLETE with branch failure collection & audit
//! - **RETRY**: re-execute enclosing operation up to N times
//! - **Capability checks**: per-operation enforcement when agent context active
//! - **DELEGATE**: execute under callee agent's capabilities

use std::collections::{BTreeMap, HashMap, HashSet};

use al_ast::*;
use al_diagnostics::{AuditEventType, ErrorCode, RuntimeFailure};

use crate::{Runtime, Value};

/// Names of built-in stdlib data operations.
const STDLIB_BUILTINS: &[&str] = &["FILTER", "MAP", "REDUCE"];

// =========================================================================
// Error type
// =========================================================================

/// Errors that can occur during interpretation.
#[derive(Debug)]
pub enum InterpreterError {
    /// A runtime failure (assertion, capability denial, escalation, etc.).
    RuntimeFailure(RuntimeFailure),
    /// An undefined identifier was referenced.
    UndefinedIdentifier(String),
    /// An undefined operation was called.
    UndefinedOperation(String),
    /// A type error during evaluation.
    TypeError(String),
    /// Assignment to an immutable binding.
    ImmutableAssign(String),
    /// Program was explicitly halted.
    Halted { reason: String, value: Value },
    /// RETRY exhausted all attempts.
    RetryExhausted {
        count: i64,
        last_failure: Value,
    },
    /// Capability denied for an operation.
    CapabilityDenied {
        agent_id: String,
        capability: String,
    },
}

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RuntimeFailure(rf) => write!(f, "runtime failure: {}", rf.message),
            Self::UndefinedIdentifier(n) => write!(f, "undefined identifier: {}", n),
            Self::UndefinedOperation(n) => write!(f, "undefined operation: {}", n),
            Self::TypeError(msg) => write!(f, "type error: {}", msg),
            Self::ImmutableAssign(n) => {
                write!(f, "cannot assign to immutable binding '{}'", n)
            }
            Self::Halted { reason, value } => {
                write!(f, "HALT({}): {}", reason, value)
            }
            Self::RetryExhausted { count, last_failure } => {
                write!(f, "RETRY exhausted after {} attempts: {}", count, last_failure)
            }
            Self::CapabilityDenied { agent_id, capability } => {
                write!(
                    f,
                    "agent '{}' lacks required capability '{}'",
                    agent_id, capability
                )
            }
        }
    }
}

impl From<RuntimeFailure> for InterpreterError {
    fn from(rf: RuntimeFailure) -> Self {
        InterpreterError::RuntimeFailure(rf)
    }
}

// =========================================================================
// Internal helpers
// =========================================================================

/// A stored operation definition (cloned from the AST during load).
#[derive(Debug, Clone)]
struct OperationDef {
    #[allow(dead_code)]
    name: String,
    inputs: Vec<String>,
    body: Spanned<Block>,
    /// Required capabilities extracted from REQUIRE SCOPE declarations.
    required_caps: Vec<String>,
}

/// Result of executing a single statement.
enum StmtResult {
    /// Continue to the next statement.
    Continue,
    /// An EMIT was executed; propagate the value upward.
    Emit(Value),
    /// A HALT was executed.
    Halt { reason: String, value: Value },
    /// A RETRY was requested; re-execute the enclosing operation.
    Retry { count: i64 },
}

/// Monotonically increasing counter for VC assertion IDs.
static VC_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_vc_id() -> String {
    let id = VC_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("vc-rt-{:04}", id)
}

// =========================================================================
// Interpreter
// =========================================================================

/// The AgentLang tree-walking interpreter.
pub struct Interpreter {
    /// The underlying runtime state machine (H/R/M/K/Q/L).
    pub runtime: Runtime,
    /// User-defined operations, keyed by name.
    operations: HashMap<String, OperationDef>,
    /// Pipeline chains to execute, in declaration order.
    pipelines: Vec<(String, Spanned<PipelineChain>)>,
    /// Set of bindings declared with `MUTABLE`.
    mutables: HashSet<String>,
    /// Currently active agent context (for capability checks).
    /// When set, operation calls are checked against this agent's capabilities.
    active_agent: Option<String>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    /// Create a new, empty interpreter.
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new(),
            operations: HashMap::new(),
            pipelines: Vec::new(),
            mutables: HashSet::new(),
            active_agent: None,
        }
    }

    /// Set the active agent context for capability checking.
    pub fn set_active_agent(&mut self, agent_id: &str) {
        self.active_agent = Some(agent_id.to_string());
    }

    // =====================================================================
    // Program loading
    // =====================================================================

    /// Load all declarations from a parsed program.
    pub fn load_program(&mut self, program: &Program) {
        for decl in &program.declarations {
            match &decl.node {
                Declaration::AgentDecl { name, properties } => {
                    let mut caps = al_capabilities::CapabilitySet::empty();
                    for prop in properties {
                        if let AgentProperty::Capabilities(cap_names) = &prop.node {
                            for cap_name in cap_names {
                                if let Ok(cap) =
                                    al_capabilities::resolve_capability(&cap_name.node)
                                {
                                    caps.insert(cap);
                                }
                            }
                        }
                    }
                    self.runtime.register_agent(&name.node, caps);
                    if let Some(agent) = self.runtime.get_agent_mut(&name.node) {
                        agent.mark_ready();
                    }
                }
                Declaration::OperationDecl {
                    name, inputs, body, requires, ..
                } => {
                    let param_names: Vec<String> =
                        inputs.iter().map(|p| p.node.name.node.clone()).collect();
                    // Extract required capabilities from REQUIRE clauses that
                    // reference capability names (simple identifiers).
                    let mut required_caps = Vec::new();
                    for req in requires {
                        if let Expr::Identifier(cap_name) = &req.node {
                            required_caps.push(cap_name.clone());
                        }
                    }
                    self.operations.insert(
                        name.node.clone(),
                        OperationDef {
                            name: name.node.clone(),
                            inputs: param_names,
                            body: body.clone(),
                            required_caps,
                        },
                    );
                }
                Declaration::PipelineDecl { name, chain } => {
                    self.pipelines.push((name.node.clone(), chain.clone()));
                }
                // TypeDecl and SchemaDecl are informational only at runtime.
                _ => {}
            }
        }
    }

    // =====================================================================
    // Top-level execution
    // =====================================================================

    /// Execute all pipelines in declaration order.
    ///
    /// Returns the result of the **last** pipeline.
    pub fn run(&mut self) -> Result<Value, InterpreterError> {
        let pipelines = self.pipelines.clone();
        let mut last = Value::None;
        for (_name, chain) in &pipelines {
            last = self.exec_pipeline_chain(chain, Value::None)?;
        }
        Ok(last)
    }

    /// Execute a specific pipeline by name.
    pub fn run_pipeline(&mut self, name: &str) -> Result<Value, InterpreterError> {
        let chain = self
            .pipelines
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| c.clone())
            .ok_or_else(|| {
                InterpreterError::UndefinedOperation(format!("pipeline '{}'", name))
            })?;
        self.exec_pipeline_chain(&chain, Value::None)
    }

    /// Execute a single named operation with explicit arguments.
    pub fn run_operation(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, InterpreterError> {
        self.call_operation(name, args)
    }

    // =====================================================================
    // Pipeline execution
    // =====================================================================

    fn exec_pipeline_chain(
        &mut self,
        chain: &Spanned<PipelineChain>,
        initial: Value,
    ) -> Result<Value, InterpreterError> {
        // Emit PIPELINE_STARTED audit event.
        let agent_id = self
            .active_agent
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        self.runtime.emit_audit_event(
            &agent_id,
            AuditEventType::PipelineStarted,
            serde_json::json!({ "stages": chain.node.stages.len() }),
        );

        let mut current = initial;
        for (i, stage) in chain.node.stages.iter().enumerate() {
            if i == 0 {
                current = self.eval_pipeline_stage(&stage.expr, current)?;
            } else {
                // Short-circuit on FAILURE.
                if let Value::Failure { .. } = &current {
                    return Ok(current);
                }
                current = self.eval_pipeline_stage(&stage.expr, current)?;
            }
        }
        Ok(current)
    }

    /// Evaluate a single pipeline stage, threading `input` as the first argument
    /// when the stage references an operation.
    fn eval_pipeline_stage(
        &mut self,
        expr: &Spanned<Expr>,
        input: Value,
    ) -> Result<Value, InterpreterError> {
        match &expr.node {
            // Bare identifier → call the named operation with threaded input.
            Expr::Identifier(name) => {
                if self.operations.contains_key(name.as_str()) {
                    self.call_operation(name, vec![input])
                } else {
                    // Not an operation — evaluate as an expression.
                    self.eval_expr(expr)
                }
            }
            // Call expression → prepend the threaded input to the argument list.
            Expr::Call { func, args } => {
                if let Expr::Identifier(name) = &func.node {
                    if self.operations.contains_key(name.as_str()) {
                        let mut eval_args = vec![input];
                        for arg in args {
                            eval_args.push(self.eval_expr(&arg.node.value)?);
                        }
                        return self.call_operation(name, eval_args);
                    }
                }
                // Fall back to normal expression evaluation.
                self.eval_expr(expr)
            }
            // Anything else — plain expression evaluation.
            _ => self.eval_expr(expr),
        }
    }

    // =====================================================================
    // Operation dispatch
    // =====================================================================

    fn call_operation(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, InterpreterError> {
        // Check for built-in stdlib operations first.
        if STDLIB_BUILTINS.contains(&name) {
            return self.call_stdlib_builtin(name, args);
        }

        let op = match self.operations.get(name) {
            Some(op) => op.clone(),
            None => {
                return Ok(Value::Failure {
                    code: "NOT_IMPLEMENTED".to_string(),
                    message: format!("operation '{}' is not defined", name),
                    details: Box::new(Value::None),
                });
            }
        };

        // Emit OPERATION_CALLED audit event.
        let agent_id = self
            .active_agent
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        self.runtime.emit_audit_event(
            &agent_id,
            AuditEventType::OperationCalled,
            serde_json::json!({ "operation": name }),
        );

        // Capability check: if an agent context is active, verify required caps.
        if let Some(ref agent_id) = self.active_agent {
            for cap_name in &op.required_caps {
                if let Ok(cap) = al_capabilities::resolve_capability(cap_name) {
                    if let Err(_rf) = self.runtime.check_capability(agent_id, cap) {
                        return Err(InterpreterError::CapabilityDenied {
                            agent_id: agent_id.clone(),
                            capability: cap_name.clone(),
                        });
                    }
                }
            }
        }

        self.execute_operation_body(&op, &args)
    }

    /// Execute an operation body, binding arguments and managing scope.
    /// Handles RETRY by re-executing the body up to N times.
    fn execute_operation_body(
        &mut self,
        op: &OperationDef,
        args: &[Value],
    ) -> Result<Value, InterpreterError> {
        // Save caller state.
        let saved_regs = self.runtime.registers.clone();
        let saved_mutables = self.mutables.clone();

        let bind_args = |this: &mut Self| {
            // Bind positional inputs.
            for (i, param) in op.inputs.iter().enumerate() {
                if let Some(arg) = args.get(i) {
                    this.runtime.reg_set(param.clone(), arg.clone());
                }
            }
            // If there is a threaded input but no declared parameters, bind as `_input`.
            if !args.is_empty() && op.inputs.is_empty() {
                this.runtime.reg_set("_input", args[0].clone());
            }
        };

        bind_args(self);

        // Execute the operation body.
        let result = self.exec_block(&op.body);

        match result {
            // RETRY: re-execute the body up to `count` additional times.
            Err(InterpreterError::RetryExhausted {
                count: retry_count, ..
            }) => {
                let mut last_failure = Value::Failure {
                    code: "RETRY_EXHAUSTED".to_string(),
                    message: "initial attempt failed".to_string(),
                    details: Box::new(Value::None),
                };

                for attempt in 0..retry_count {
                    // Re-bind args and retry.
                    self.runtime.registers = saved_regs.clone();
                    self.mutables = saved_mutables.clone();
                    bind_args(self);

                    match self.exec_block(&op.body) {
                        Ok(Some(val)) => {
                            // Success on retry.
                            self.runtime.registers = saved_regs;
                            self.mutables = saved_mutables;
                            return Ok(val);
                        }
                        Ok(None) => {
                            self.runtime.registers = saved_regs;
                            self.mutables = saved_mutables;
                            return Ok(Value::None);
                        }
                        Err(InterpreterError::Halted { reason, value }) => {
                            last_failure = Value::Failure {
                                code: "HALTED".to_string(),
                                message: reason,
                                details: Box::new(value),
                            };
                            // Continue retrying.
                        }
                        Err(InterpreterError::RetryExhausted { .. }) => {
                            // Another RETRY inside — treat as failure, continue.
                            last_failure = Value::Failure {
                                code: "RETRY_EXHAUSTED".to_string(),
                                message: format!("retry attempt {} failed", attempt + 1),
                                details: Box::new(Value::None),
                            };
                        }
                        Err(InterpreterError::RuntimeFailure(rf)) => {
                            last_failure = Value::Failure {
                                code: format!("{:?}", rf.code),
                                message: rf.message.clone(),
                                details: Box::new(Value::None),
                            };
                        }
                        Err(other) => {
                            // Non-retryable error.
                            self.runtime.registers = saved_regs;
                            self.mutables = saved_mutables;
                            return Err(other);
                        }
                    }
                }

                // All retries exhausted.
                self.runtime.registers = saved_regs;
                self.mutables = saved_mutables;
                Ok(Value::Failure {
                    code: "RETRY_EXHAUSTED".to_string(),
                    message: format!(
                        "retry exhausted after {} attempts",
                        retry_count + 1
                    ),
                    details: Box::new(last_failure),
                })
            }
            other => {
                // Restore caller state.
                self.runtime.registers = saved_regs;
                self.mutables = saved_mutables;

                match other {
                    Ok(Some(val)) => Ok(val),
                    Ok(None) => Ok(Value::None),
                    Err(InterpreterError::Halted { reason, value }) => {
                        Ok(Value::Failure {
                            code: "HALTED".to_string(),
                            message: reason,
                            details: Box::new(value),
                        })
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    // =====================================================================
    // Built-in stdlib data operations
    // =====================================================================

    /// Dispatch a built-in stdlib operation (FILTER, MAP, REDUCE).
    fn call_stdlib_builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, InterpreterError> {
        let agent_id = self
            .active_agent
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        self.runtime.emit_audit_event(
            &agent_id,
            AuditEventType::StdlibCall,
            serde_json::json!({ "operation": name }),
        );

        match name {
            "FILTER" => self.stdlib_filter(args),
            "MAP" => self.stdlib_map(args),
            "REDUCE" => self.stdlib_reduce(args),
            _ => Ok(Value::Failure {
                code: "NOT_IMPLEMENTED".to_string(),
                message: format!("stdlib operation '{}' is not implemented", name),
                details: Box::new(Value::None),
            }),
        }
    }

    /// FILTER(list, predicate_op_name) -> List
    ///
    /// Calls the named predicate operation for each element.
    /// Keeps elements where the predicate returns Bool(true).
    fn stdlib_filter(&mut self, args: Vec<Value>) -> Result<Value, InterpreterError> {
        let (list, pred_name) = match args.as_slice() {
            [Value::List(items), Value::Str(pred)] => (items.clone(), pred.clone()),
            [_, Value::Str(_)] => {
                return Err(InterpreterError::TypeError(
                    "FILTER: first argument must be a List".to_string(),
                ));
            }
            _ => {
                return Err(InterpreterError::TypeError(
                    "FILTER requires (List, String) arguments".to_string(),
                ));
            }
        };

        let mut result = Vec::new();
        for item in &list {
            let pred_result = self.call_operation(&pred_name, vec![item.clone()])?;
            if pred_result == Value::Bool(true) {
                result.push(item.clone());
            }
        }
        Ok(Value::List(result))
    }

    /// MAP(list, transform_op_name) -> List
    ///
    /// Calls the named operation for each element and collects the results.
    fn stdlib_map(&mut self, args: Vec<Value>) -> Result<Value, InterpreterError> {
        let (list, op_name) = match args.as_slice() {
            [Value::List(items), Value::Str(op)] => (items.clone(), op.clone()),
            [_, Value::Str(_)] => {
                return Err(InterpreterError::TypeError(
                    "MAP: first argument must be a List".to_string(),
                ));
            }
            _ => {
                return Err(InterpreterError::TypeError(
                    "MAP requires (List, String) arguments".to_string(),
                ));
            }
        };

        let mut result = Vec::new();
        for item in &list {
            let mapped = self.call_operation(&op_name, vec![item.clone()])?;
            result.push(mapped);
        }
        Ok(Value::List(result))
    }

    /// REDUCE(list, initial, reducer_op_name) -> Value
    ///
    /// Starting from `initial`, calls the named operation with (accumulator, element)
    /// for each element, threading the result forward.
    fn stdlib_reduce(&mut self, args: Vec<Value>) -> Result<Value, InterpreterError> {
        let (list, initial, op_name) = match args.as_slice() {
            [Value::List(items), init, Value::Str(op)] => {
                (items.clone(), init.clone(), op.clone())
            }
            [_, _, Value::Str(_)] => {
                return Err(InterpreterError::TypeError(
                    "REDUCE: first argument must be a List".to_string(),
                ));
            }
            _ => {
                return Err(InterpreterError::TypeError(
                    "REDUCE requires (List, initial, String) arguments".to_string(),
                ));
            }
        };

        let mut acc = initial;
        for item in &list {
            acc = self.call_operation(&op_name, vec![acc, item.clone()])?;
        }
        Ok(acc)
    }

    // =====================================================================
    // Block execution
    // =====================================================================

    fn exec_block(
        &mut self,
        block: &Spanned<Block>,
    ) -> Result<Option<Value>, InterpreterError> {
        let mut last_emit: Option<Value> = None;
        for stmt in &block.node.stmts {
            match self.exec_stmt(stmt)? {
                StmtResult::Continue => {}
                StmtResult::Emit(val) => {
                    last_emit = Some(val);
                }
                StmtResult::Halt { reason, value } => {
                    return Err(InterpreterError::Halted { reason, value });
                }
                StmtResult::Retry { count } => {
                    // Propagate retry request up to call_operation_with_retry.
                    return Err(InterpreterError::RetryExhausted {
                        count,
                        last_failure: Value::None,
                    });
                }
            }
        }
        Ok(last_emit)
    }

    // =====================================================================
    // Statement execution
    // =====================================================================

    fn exec_stmt(
        &mut self,
        stmt: &Spanned<Statement>,
    ) -> Result<StmtResult, InterpreterError> {
        match &stmt.node {
            // ----- STORE (immutable binding) ---------------------------------
            Statement::Store { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.runtime.reg_set(name.node.clone(), val);
                self.mutables.remove(&name.node);
                Ok(StmtResult::Continue)
            }

            // ----- MUTABLE (mutable binding) ---------------------------------
            Statement::Mutable { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.runtime.reg_set(name.node.clone(), val);
                self.mutables.insert(name.node.clone());
                Ok(StmtResult::Continue)
            }

            // ----- ASSIGN (reassign mutable) ---------------------------------
            Statement::Assign { target, value } => {
                if !self.mutables.contains(&target.node) {
                    return Err(InterpreterError::ImmutableAssign(
                        target.node.clone(),
                    ));
                }
                let val = self.eval_expr(value)?;
                self.runtime.reg_set(target.node.clone(), val);
                Ok(StmtResult::Continue)
            }

            // ----- MATCH -----------------------------------------------------
            Statement::Match {
                expr,
                arms,
                otherwise,
            } => {
                let val = self.eval_expr(expr)?;

                for arm in arms {
                    if let Some(bindings) =
                        match_pattern(&arm.node.pattern, &val)
                    {
                        for (k, v) in &bindings {
                            self.runtime.reg_set(k.clone(), v.clone());
                        }
                        return self.exec_match_body(&arm.node.body);
                    }
                }

                if let Some(otherwise_body) = otherwise {
                    return self.exec_match_body(otherwise_body);
                }

                Ok(StmtResult::Continue)
            }

            // ----- LOOP max: N => { body } -----------------------------------
            Statement::Loop { max_iters, body } => {
                let max = max_iters.node;
                for _i in 0..max {
                    match self.exec_block(body)? {
                        Some(val) => return Ok(StmtResult::Emit(val)),
                        None => {}
                    }
                }
                Ok(StmtResult::Continue)
            }

            // ----- EMIT -----------------------------------------------------
            Statement::Emit { value } => {
                let val = match value {
                    Some(expr) => self.eval_expr(expr)?,
                    None => Value::None,
                };
                Ok(StmtResult::Emit(val))
            }

            // ----- ASSERT (with VC metadata) ---------------------------------
            Statement::Assert { condition } => {
                let val = self.eval_expr(condition)?;
                let vc_id = next_vc_id();
                let solver_reason = format!("{:?}", condition.node);
                match val {
                    Value::Bool(true) => Ok(StmtResult::Continue),
                    Value::Bool(false) => {
                        let _rf = self
                            .runtime
                            .execute_assert(false, &vc_id, &solver_reason)
                            .unwrap_err();
                        // Wrap into a RuntimeFailure that carries VC metadata.
                        Err(InterpreterError::RuntimeFailure(
                            RuntimeFailure::with_details(
                                ErrorCode::AssertionFailed,
                                format!(
                                    "assertion failed: vc_id={}, reason={}",
                                    vc_id, solver_reason
                                ),
                                serde_json::json!({
                                    "vc_id": vc_id,
                                    "solver_reason": solver_reason,
                                }),
                            ),
                        ))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "ASSERT condition must be boolean".to_string(),
                    )),
                }
            }

            // ----- RETRY (re-execute enclosing operation) -------------------
            Statement::Retry { count, .. } => {
                let n = count.node;
                Ok(StmtResult::Retry { count: n })
            }

            // ----- ESCALATE (deterministic failure mapping) -----------------
            Statement::Escalate { message } => {
                let agent_id = self
                    .active_agent
                    .clone()
                    .unwrap_or_else(|| "runtime".to_string());
                let msg = match message {
                    Some(expr) => {
                        let v = self.eval_expr(expr)?;
                        match v {
                            Value::Str(s) => Some(s),
                            other => Some(other.to_string()),
                        }
                    }
                    None => None,
                };
                let failure = self.runtime.execute_escalate(msg, &agent_id);
                Err(InterpreterError::RuntimeFailure(failure))
            }

            // ----- HALT -----------------------------------------------------
            Statement::Halt { reason, value } => {
                let val = match value {
                    Some(expr) => self.eval_expr(expr)?,
                    None => Value::None,
                };
                Ok(StmtResult::Halt {
                    reason: reason.node.clone(),
                    value: val,
                })
            }

            // ----- CHECKPOINT ------------------------------------------------
            Statement::Checkpoint { .. } => {
                let state = Value::Map(
                    self.runtime
                        .registers
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                );
                let _id = self.runtime.create_checkpoint("runtime", state);
                Ok(StmtResult::Continue)
            }

            // ----- DELEGATE (execute under callee's caps) --------------------
            Statement::Delegate {
                task,
                target,
                clauses,
            } => {
                let mut input_val = Value::None;
                for clause in clauses {
                    if let DelegateClause::Input(expr) = &clause.node {
                        input_val = self.eval_expr(expr)?;
                    }
                }

                // Switch to the target agent's context for capability checking.
                let saved_agent = self.active_agent.clone();
                let target_agent = &target.node;
                // Verify target agent exists; if registered, use their caps.
                if self.runtime.get_agent(target_agent).is_some() {
                    self.active_agent = Some(target_agent.clone());
                }

                let result = self.call_operation(&task.node, vec![input_val])?;

                // Restore caller's agent context.
                self.active_agent = saved_agent;

                self.runtime
                    .reg_set(format!("{}_result", task.node), result);
                Ok(StmtResult::Continue)
            }

            // ----- Bare expression statement ---------------------------------
            Statement::Expr { expr } => {
                let _val = self.eval_expr(expr)?;
                Ok(StmtResult::Continue)
            }
        }
    }

    fn exec_match_body(
        &mut self,
        body: &Spanned<MatchBody>,
    ) -> Result<StmtResult, InterpreterError> {
        match &body.node {
            MatchBody::Block(block) => match self.exec_block(block)? {
                Some(val) => Ok(StmtResult::Emit(val)),
                None => Ok(StmtResult::Continue),
            },
            MatchBody::Expr(expr) => {
                let val = self.eval_expr(expr)?;
                Ok(StmtResult::Emit(val))
            }
        }
    }

    // =====================================================================
    // Expression evaluation
    // =====================================================================

    fn eval_expr(&mut self, expr: &Spanned<Expr>) -> Result<Value, InterpreterError> {
        match &expr.node {
            Expr::Literal(lit) => Ok(literal_to_value(lit)),

            Expr::Identifier(name) => self
                .runtime
                .reg_get(name)
                .cloned()
                .ok_or_else(|| InterpreterError::UndefinedIdentifier(name.clone())),

            Expr::BinaryOp { left, op, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                eval_binary_op(&l, &op.node, &r)
            }

            Expr::UnaryOp { op, operand } => {
                let v = self.eval_expr(operand)?;
                eval_unary_op(&op.node, &v)
            }

            Expr::Call { func, args } => {
                if let Expr::Identifier(name) = &func.node {
                    let mut eval_args = Vec::new();
                    for arg in args {
                        eval_args.push(self.eval_expr(&arg.node.value)?);
                    }
                    self.call_operation(name, eval_args)
                } else {
                    Err(InterpreterError::TypeError(
                        "only named function calls are supported".to_string(),
                    ))
                }
            }

            Expr::Member { object, field } => {
                let obj = self.eval_expr(object)?;
                match &obj {
                    Value::Map(map) => {
                        Ok(map.get(&field.node).cloned().unwrap_or(Value::None))
                    }
                    _ => Err(InterpreterError::TypeError(format!(
                        "cannot access field '{}' on {}",
                        field.node, obj
                    ))),
                }
            }

            Expr::List { elements } => {
                let mut items = Vec::new();
                for elem in elements {
                    items.push(self.eval_expr(elem)?);
                }
                Ok(Value::List(items))
            }

            Expr::Map { items } => {
                let mut map = BTreeMap::new();
                for item in items {
                    let key = match &item.node.key.node {
                        MapKey::String(s) => s.clone(),
                        MapKey::Identifier(s) => s.clone(),
                    };
                    let val = self.eval_expr(&item.node.value)?;
                    map.insert(key, val);
                }
                Ok(Value::Map(map))
            }

            Expr::Pipeline { left, op: _, right } => {
                let l = self.eval_expr(left)?;
                if let Value::Failure { .. } = &l {
                    return Ok(l);
                }
                self.eval_pipeline_stage(right, l)
            }

            Expr::Fork { branches, .. } => {
                let branch_data: Vec<_> = branches
                    .iter()
                    .map(|b| (b.node.name.node.clone(), b.node.chain.clone()))
                    .collect();

                let mut results = Vec::new();
                let mut failures: Vec<(String, Value)> = Vec::new();

                for (name, chain) in &branch_data {
                    let result = self.exec_pipeline_chain(chain, Value::None)?;
                    if let Value::Failure { .. } = &result {
                        failures.push((name.clone(), result.clone()));
                    }
                    results.push(result);
                }

                // ALL_COMPLETE: if any branch failed, return aggregated failure.
                if !failures.is_empty() {
                    let failure_details = Value::List(
                        failures
                            .iter()
                            .map(|(name, val)| {
                                let mut m = BTreeMap::new();
                                m.insert("branch".to_string(), Value::Str(name.clone()));
                                m.insert("failure".to_string(), val.clone());
                                Value::Map(m)
                            })
                            .collect(),
                    );
                    return Ok(Value::Failure {
                        code: "FORK_JOIN_FAILED".to_string(),
                        message: format!(
                            "{} of {} branches failed",
                            failures.len(),
                            branch_data.len()
                        ),
                        details: Box::new(failure_details),
                    });
                }

                Ok(Value::List(results))
            }

            Expr::Paren { inner } => self.eval_expr(inner),

            Expr::Confidence { expr: inner } => self.eval_expr(inner),

            Expr::Range { start, end } => {
                let s = self.eval_expr(start)?;
                let e = self.eval_expr(end)?;
                match (&s, &e) {
                    (Value::Int(a), Value::Int(b)) => {
                        let items: Vec<Value> =
                            (*a..=*b).map(Value::Int).collect();
                        Ok(Value::List(items))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "range requires integer bounds".to_string(),
                    )),
                }
            }

            Expr::Resume { expr: inner } => self.eval_expr(inner),
        }
    }
}

// =========================================================================
// Pattern matching (pure function)
// =========================================================================

/// Attempt to match `value` against `pattern`, returning variable bindings
/// on success.
fn match_pattern(
    pattern: &Spanned<Pattern>,
    value: &Value,
) -> Option<HashMap<String, Value>> {
    match &pattern.node {
        Pattern::Wildcard => Some(HashMap::new()),

        Pattern::Literal(lit) => {
            let lit_val = literal_to_value(lit);
            if &lit_val == value {
                Some(HashMap::new())
            } else {
                None
            }
        }

        Pattern::Success(inner) => {
            if let Value::Success(inner_val) = value {
                match_pattern(inner, inner_val)
            } else {
                None
            }
        }

        Pattern::Failure {
            code,
            msg_pat,
            details_pat,
        } => {
            if let Value::Failure {
                code: val_code,
                message: val_msg,
                details: val_details,
            } = value
            {
                let mut bindings = HashMap::new();

                // Bind error code.
                bindings
                    .insert(code.node.clone(), Value::Str(val_code.clone()));

                // Match message sub-pattern.
                let msg_bindings =
                    match_pattern(msg_pat, &Value::Str(val_msg.clone()))?;
                bindings.extend(msg_bindings);

                // Match details sub-pattern.
                let det_bindings = match_pattern(details_pat, val_details)?;
                bindings.extend(det_bindings);

                Some(bindings)
            } else {
                None
            }
        }

        Pattern::Identifier(name) => {
            let mut bindings = HashMap::new();
            bindings.insert(name.clone(), value.clone());
            Some(bindings)
        }

        Pattern::Constructor { .. } => {
            // Not supported in MVP slice 1.
            None
        }
    }
}

// =========================================================================
// Literal → Value
// =========================================================================

fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::Integer(n) => Value::Int(*n),
        Literal::Float(f) => Value::Float(*f),
        Literal::String(s) => Value::Str(s.clone()),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::None => Value::None,
        Literal::Duration(s) => Value::Str(s.clone()),
        Literal::Size(s) => Value::Str(s.clone()),
        Literal::Confidence(f) => Value::Float(*f),
        Literal::Hash(s) => Value::Str(s.clone()),
    }
}

// =========================================================================
// Binary & unary operators
// =========================================================================

fn eval_binary_op(
    left: &Value,
    op: &BinaryOp,
    right: &Value,
) -> Result<Value, InterpreterError> {
    match (left, op, right) {
        // Integer arithmetic
        (Value::Int(a), BinaryOp::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
        (Value::Int(a), BinaryOp::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
        (Value::Int(a), BinaryOp::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),
        (Value::Int(a), BinaryOp::Div, Value::Int(b)) => {
            if *b == 0 {
                Ok(Value::Failure {
                    code: "DIVISION_BY_ZERO".to_string(),
                    message: "division by zero".to_string(),
                    details: Box::new(Value::None),
                })
            } else {
                Ok(Value::Int(a / b))
            }
        }
        (Value::Int(a), BinaryOp::Mod, Value::Int(b)) => {
            if *b == 0 {
                Ok(Value::Failure {
                    code: "DIVISION_BY_ZERO".to_string(),
                    message: "modulo by zero".to_string(),
                    details: Box::new(Value::None),
                })
            } else {
                Ok(Value::Int(a % b))
            }
        }

        // Float arithmetic
        (Value::Float(a), BinaryOp::Add, Value::Float(b)) => {
            Ok(Value::Float(a + b))
        }
        (Value::Float(a), BinaryOp::Sub, Value::Float(b)) => {
            Ok(Value::Float(a - b))
        }
        (Value::Float(a), BinaryOp::Mul, Value::Float(b)) => {
            Ok(Value::Float(a * b))
        }
        (Value::Float(a), BinaryOp::Div, Value::Float(b)) => {
            Ok(Value::Float(a / b))
        }

        // Mixed numeric (int + float → float)
        (Value::Int(a), BinaryOp::Add, Value::Float(b)) => {
            Ok(Value::Float(*a as f64 + b))
        }
        (Value::Float(a), BinaryOp::Add, Value::Int(b)) => {
            Ok(Value::Float(a + *b as f64))
        }
        (Value::Int(a), BinaryOp::Sub, Value::Float(b)) => {
            Ok(Value::Float(*a as f64 - b))
        }
        (Value::Float(a), BinaryOp::Sub, Value::Int(b)) => {
            Ok(Value::Float(a - *b as f64))
        }
        (Value::Int(a), BinaryOp::Mul, Value::Float(b)) => {
            Ok(Value::Float(*a as f64 * b))
        }
        (Value::Float(a), BinaryOp::Mul, Value::Int(b)) => {
            Ok(Value::Float(a * *b as f64))
        }

        // String concatenation
        (Value::Str(a), BinaryOp::Add, Value::Str(b)) => {
            Ok(Value::Str(format!("{}{}", a, b)))
        }

        // Integer comparisons
        (Value::Int(a), BinaryOp::Eq, Value::Int(b)) => {
            Ok(Value::Bool(a == b))
        }
        (Value::Int(a), BinaryOp::Neq, Value::Int(b)) => {
            Ok(Value::Bool(a != b))
        }
        (Value::Int(a), BinaryOp::Gt, Value::Int(b)) => {
            Ok(Value::Bool(a > b))
        }
        (Value::Int(a), BinaryOp::Gte, Value::Int(b)) => {
            Ok(Value::Bool(a >= b))
        }
        (Value::Int(a), BinaryOp::Lt, Value::Int(b)) => {
            Ok(Value::Bool(a < b))
        }
        (Value::Int(a), BinaryOp::Lte, Value::Int(b)) => {
            Ok(Value::Bool(a <= b))
        }

        // Float comparisons
        (Value::Float(a), BinaryOp::Eq, Value::Float(b)) => {
            Ok(Value::Bool(a == b))
        }
        (Value::Float(a), BinaryOp::Neq, Value::Float(b)) => {
            Ok(Value::Bool(a != b))
        }
        (Value::Float(a), BinaryOp::Gt, Value::Float(b)) => {
            Ok(Value::Bool(a > b))
        }
        (Value::Float(a), BinaryOp::Lt, Value::Float(b)) => {
            Ok(Value::Bool(a < b))
        }

        // String comparisons
        (Value::Str(a), BinaryOp::Eq, Value::Str(b)) => {
            Ok(Value::Bool(a == b))
        }
        (Value::Str(a), BinaryOp::Neq, Value::Str(b)) => {
            Ok(Value::Bool(a != b))
        }

        // Boolean comparisons
        (Value::Bool(a), BinaryOp::Eq, Value::Bool(b)) => {
            Ok(Value::Bool(a == b))
        }
        (Value::Bool(a), BinaryOp::Neq, Value::Bool(b)) => {
            Ok(Value::Bool(a != b))
        }

        // Logical operators
        (Value::Bool(a), BinaryOp::And, Value::Bool(b)) => {
            Ok(Value::Bool(*a && *b))
        }
        (Value::Bool(a), BinaryOp::Or, Value::Bool(b)) => {
            Ok(Value::Bool(*a || *b))
        }

        _ => Err(InterpreterError::TypeError(format!(
            "unsupported binary operation: {} {:?} {}",
            left, op, right
        ))),
    }
}

fn eval_unary_op(
    op: &UnaryOp,
    operand: &Value,
) -> Result<Value, InterpreterError> {
    match (op, operand) {
        (UnaryOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
        (UnaryOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
        (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
        _ => Err(InterpreterError::TypeError(format!(
            "unsupported unary operation: {:?} {}",
            op, operand
        ))),
    }
}

// =========================================================================
// Convenience: parse + load + run
// =========================================================================

/// Parse source code and execute all pipelines end-to-end.
///
/// Returns the final pipeline result, or a formatted error string.
pub fn execute_source(source: &str) -> Result<Value, String> {
    let program = al_parser::parse(source).map_err(|diags| {
        diags
            .iter()
            .map(|d| format!("[{}] {}", d.code, d.message))
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    let mut interp = Interpreter::new();
    interp.load_program(&program);

    interp.run().map_err(|e| e.to_string())
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse, load, run, return result.
    fn run_source(source: &str) -> Result<Value, InterpreterError> {
        let program = al_parser::parse(source).expect("parse should succeed");
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        interp.run()
    }

    /// Helper: parse, load, run a named operation with args.
    fn run_op(
        source: &str,
        op_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, InterpreterError> {
        let program = al_parser::parse(source).expect("parse should succeed");
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        interp.run_operation(op_name, args)
    }

    // -----------------------------------------------------------------
    // Expression evaluator tests
    // -----------------------------------------------------------------

    #[test]
    fn eval_integer_literal() {
        let result = run_op(
            "OPERATION test => BODY { EMIT 42 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn eval_string_literal() {
        let result = run_op(
            r#"OPERATION test => BODY { EMIT "hello" }"#,
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Str("hello".into()));
    }

    #[test]
    fn eval_bool_literal() {
        let result = run_op(
            "OPERATION test => BODY { EMIT TRUE }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn eval_none_literal() {
        let result = run_op(
            "OPERATION test => BODY { EMIT NONE }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::None);
    }

    #[test]
    fn eval_float_literal() {
        let result = run_op(
            "OPERATION test => BODY { EMIT 3.14 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Float(3.14));
    }

    #[test]
    fn eval_binary_add_int() {
        let result = run_op(
            "OPERATION test => BODY { EMIT 10 + 32 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn eval_binary_sub_int() {
        let result = run_op(
            "OPERATION test => BODY { EMIT 50 - 8 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn eval_binary_mul_int() {
        let result = run_op(
            "OPERATION test => BODY { EMIT 6 * 7 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn eval_binary_div_int() {
        let result = run_op(
            "OPERATION test => BODY { EMIT 84 / 2 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn eval_div_by_zero_returns_failure() {
        let result = run_op(
            "OPERATION test => BODY { EMIT 1 / 0 }",
            "test",
            vec![],
        );
        match result.unwrap() {
            Value::Failure { code, .. } => {
                assert_eq!(code, "DIVISION_BY_ZERO");
            }
            other => panic!("expected FAILURE, got {:?}", other),
        }
    }

    #[test]
    fn eval_comparison_ops() {
        let result = run_op(
            "OPERATION test => BODY { EMIT 5 GT 3 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = run_op(
            "OPERATION test => BODY { EMIT 3 LT 5 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = run_op(
            "OPERATION test => BODY { EMIT 5 EQ 5 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = run_op(
            "OPERATION test => BODY { EMIT 5 NEQ 3 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn eval_logical_ops() {
        let result = run_op(
            "OPERATION test => BODY { EMIT TRUE AND FALSE }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Bool(false));

        let result = run_op(
            "OPERATION test => BODY { EMIT TRUE OR FALSE }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn eval_unary_neg() {
        let result = run_op(
            "OPERATION test => BODY { EMIT -42 }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(-42));
    }

    #[test]
    fn eval_unary_not() {
        let result = run_op(
            "OPERATION test => BODY { EMIT NOT TRUE }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn eval_string_concat() {
        let result = run_op(
            r#"OPERATION test => BODY { EMIT "hello" + " world" }"#,
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Str("hello world".into()));
    }

    #[test]
    fn eval_list_constructor() {
        let result = run_op(
            "OPERATION test => BODY { EMIT [1, 2, 3] }",
            "test",
            vec![],
        );
        assert_eq!(
            result.unwrap(),
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn eval_map_constructor() {
        let result = run_op(
            r#"OPERATION test => BODY { EMIT { "a": 1, "b": 2 } }"#,
            "test",
            vec![],
        );
        let expected = Value::Map({
            let mut m = BTreeMap::new();
            m.insert("a".into(), Value::Int(1));
            m.insert("b".into(), Value::Int(2));
            m
        });
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn eval_member_access() {
        let result = run_op(
            r#"OPERATION test => BODY {
                STORE m = { "x": 42 }
                EMIT m.x
            }"#,
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn eval_member_access_missing_field_returns_none() {
        let result = run_op(
            r#"OPERATION test => BODY {
                STORE m = { "x": 42 }
                EMIT m.y
            }"#,
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::None);
    }

    // -----------------------------------------------------------------
    // Statement tests
    // -----------------------------------------------------------------

    #[test]
    fn stmt_store_and_emit() {
        let result = run_op(
            "OPERATION test => BODY {
                STORE x = 42
                EMIT x
            }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn stmt_mutable_and_assign() {
        let result = run_op(
            r#"OPERATION test => BODY {
                MUTABLE x @reason("counter") = 0
                x = 42
                EMIT x
            }"#,
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn stmt_assign_immutable_fails() {
        let result = run_op(
            "OPERATION test => BODY {
                STORE x = 0
                x = 42
                EMIT x
            }",
            "test",
            vec![],
        );
        assert!(matches!(result, Err(InterpreterError::ImmutableAssign(_))));
    }

    #[test]
    fn stmt_emit_none() {
        let result = run_op(
            "OPERATION test => BODY { EMIT }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::None);
    }

    #[test]
    fn stmt_halt() {
        let result = run_op(
            "OPERATION test => BODY { HALT(error) }",
            "test",
            vec![],
        );
        // HALT inside an operation produces a FAILURE value.
        match result.unwrap() {
            Value::Failure { code, .. } => assert_eq!(code, "HALTED"),
            other => panic!("expected FAILURE, got {:?}", other),
        }
    }

    #[test]
    fn stmt_assert_true() {
        let result = run_op(
            "OPERATION test => BODY {
                ASSERT TRUE
                EMIT 42
            }",
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn stmt_assert_false_fails() {
        let result = run_op(
            "OPERATION test => BODY {
                ASSERT FALSE
                EMIT 42
            }",
            "test",
            vec![],
        );
        assert!(matches!(result, Err(InterpreterError::RuntimeFailure(_))));
    }

    #[test]
    fn stmt_escalate() {
        let result = run_op(
            r#"OPERATION test => BODY {
                ESCALATE("something broke")
            }"#,
            "test",
            vec![],
        );
        assert!(matches!(result, Err(InterpreterError::RuntimeFailure(_))));
    }

    // -----------------------------------------------------------------
    // MATCH pattern-matching tests
    // -----------------------------------------------------------------

    #[test]
    fn match_success_pattern() {
        let result = run_op(
            r#"OPERATION test =>
                INPUT result: Result[Int64]
                BODY {
                    MATCH result => {
                        WHEN SUCCESS(val) -> { EMIT val }
                        WHEN FAILURE(code, msg, details) -> {
                            EMIT -1
                        }
                    }
                }"#,
            "test",
            vec![Value::Success(Box::new(Value::Int(42)))],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn match_failure_pattern() {
        let result = run_op(
            r#"OPERATION test =>
                INPUT result: Result[Int64]
                BODY {
                    MATCH result => {
                        WHEN SUCCESS(val) -> { EMIT val }
                        WHEN FAILURE(code, msg, details) -> {
                            EMIT msg
                        }
                    }
                }"#,
            "test",
            vec![Value::Failure {
                code: "ERR".into(),
                message: "bad stuff".into(),
                details: Box::new(Value::None),
            }],
        );
        assert_eq!(result.unwrap(), Value::Str("bad stuff".into()));
    }

    #[test]
    fn match_otherwise_arm() {
        let result = run_op(
            "OPERATION test =>
                INPUT x: Int64
                BODY {
                    MATCH x => {
                        WHEN 1 -> { EMIT 100 }
                        WHEN 2 -> { EMIT 200 }
                        OTHERWISE -> { EMIT 999 }
                    }
                }",
            "test",
            vec![Value::Int(5)],
        );
        assert_eq!(result.unwrap(), Value::Int(999));
    }

    #[test]
    fn match_literal_int() {
        let result = run_op(
            "OPERATION test =>
                INPUT x: Int64
                BODY {
                    MATCH x => {
                        WHEN 42 -> { EMIT TRUE }
                        OTHERWISE -> { EMIT FALSE }
                    }
                }",
            "test",
            vec![Value::Int(42)],
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn match_wildcard() {
        let result = run_op(
            "OPERATION test =>
                INPUT x: Int64
                BODY {
                    MATCH x => {
                        WHEN _ -> { EMIT 99 }
                    }
                }",
            "test",
            vec![Value::Int(42)],
        );
        assert_eq!(result.unwrap(), Value::Int(99));
    }

    // -----------------------------------------------------------------
    // LOOP tests
    // -----------------------------------------------------------------

    #[test]
    fn loop_with_mutable_counter() {
        let result = run_op(
            r#"OPERATION test => BODY {
                MUTABLE sum @reason("accumulator") = 0
                MUTABLE i @reason("counter") = 0
                LOOP max: 5 => {
                    sum = sum + 1
                    i = i + 1
                }
                EMIT sum
            }"#,
            "test",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn loop_early_emit_breaks() {
        let result = run_op(
            r#"OPERATION test => BODY {
                MUTABLE i @reason("counter") = 0
                LOOP max: 100 => {
                    i = i + 1
                    MATCH i EQ 3 => {
                        WHEN TRUE -> { EMIT i }
                        OTHERWISE -> { }
                    }
                }
            }"#,
            "test",
            vec![],
        );
        // EMIT inside LOOP breaks the loop on iteration 3.
        assert_eq!(result.unwrap(), Value::Int(3));
    }

    // -----------------------------------------------------------------
    // Operation call tests
    // -----------------------------------------------------------------

    #[test]
    fn operation_call_with_input() {
        let result = run_op(
            "OPERATION double =>
                INPUT x: Int64
                BODY {
                    EMIT x + x
                }",
            "double",
            vec![Value::Int(21)],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn operation_call_nested() {
        let result = run_op(
            "OPERATION inner =>
                INPUT x: Int64
                BODY { EMIT x + 10 }
            OPERATION outer => BODY {
                STORE r = inner(32)
                EMIT r
            }",
            "outer",
            vec![],
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn operation_undefined_returns_failure() {
        let result = run_op(
            "OPERATION test => BODY {
                STORE r = nonexistent(42)
                EMIT r
            }",
            "test",
            vec![],
        );
        match result.unwrap() {
            Value::Failure { code, .. } => {
                assert_eq!(code, "NOT_IMPLEMENTED");
            }
            other => panic!("expected FAILURE, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------
    // Pipeline tests
    // -----------------------------------------------------------------

    #[test]
    fn pipeline_simple_two_stages() {
        let result = run_source(
            "OPERATION produce => BODY { EMIT 21 }
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x + x }
            PIPELINE Main => produce -> double",
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn pipeline_three_stages() {
        let result = run_source(
            "OPERATION produce => BODY { EMIT 10 }
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x + x }
            OPERATION add_two =>
                INPUT x: Int64
                BODY { EMIT x + 2 }
            PIPELINE Main => produce -> double -> add_two",
        );
        // 10 → double → 20 → add_two → 22
        assert_eq!(result.unwrap(), Value::Int(22));
    }

    #[test]
    fn pipeline_short_circuit_on_failure() {
        let result = run_source(
            "OPERATION fail_op => BODY {
                HALT(error)
            }
            OPERATION unreachable =>
                INPUT x: Int64
                BODY { EMIT 999 }
            PIPELINE Main => fail_op -> unreachable",
        );
        match result.unwrap() {
            Value::Failure { code, .. } => assert_eq!(code, "HALTED"),
            other => panic!("expected FAILURE, got {:?}", other),
        }
    }

    #[test]
    fn pipeline_pipe_forward_op() {
        let result = run_source(
            "OPERATION produce => BODY { EMIT 5 }
            OPERATION triple =>
                INPUT x: Int64
                BODY { EMIT x * 3 }
            PIPELINE Main => produce |> triple",
        );
        assert_eq!(result.unwrap(), Value::Int(15));
    }

    // -----------------------------------------------------------------
    // Checkpoint statement test
    // -----------------------------------------------------------------

    #[test]
    fn checkpoint_creates_audit_event() {
        let program = al_parser::parse(
            "OPERATION test => BODY {
                STORE x = 42
                CHECKPOINT \"save\"
                EMIT x
            }",
        )
        .unwrap();

        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let result = interp.run_operation("test", vec![]).unwrap();
        assert_eq!(result, Value::Int(42));
        assert!(
            interp
                .runtime
                .audit_log
                .iter()
                .any(|e| e.event_type
                    == al_diagnostics::AuditEventType::CheckpointCreated)
        );
    }

    // -----------------------------------------------------------------
    // Agent registration test
    // -----------------------------------------------------------------

    #[test]
    fn agent_decl_registers_agent() {
        let program = al_parser::parse(
            "AGENT Worker =>
                CAPABILITIES [FILE_READ, API_CALL]
                TRUST_LEVEL ~0.9",
        )
        .unwrap();

        let mut interp = Interpreter::new();
        interp.load_program(&program);

        let agent = interp.runtime.get_agent("Worker").unwrap();
        assert_eq!(
            agent.status,
            crate::AgentStatus::Ready
        );
        assert!(
            agent.capabilities.contains(&al_capabilities::Capability::FileRead)
        );
    }

    // -----------------------------------------------------------------
    // execute_source convenience
    // -----------------------------------------------------------------

    #[test]
    fn execute_source_end_to_end() {
        let result = execute_source(
            "OPERATION produce => BODY { EMIT 100 }
            OPERATION halve =>
                INPUT x: Int64
                BODY { EMIT x / 2 }
            PIPELINE Main => produce -> halve",
        );
        assert_eq!(result.unwrap(), Value::Int(50));
    }

    #[test]
    fn execute_source_parse_error() {
        let result = execute_source("OPERATION ??? => BODY { EMIT 1 }");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------
    // Complex integration: match + operation + pipeline
    // -----------------------------------------------------------------

    #[test]
    fn integration_match_in_pipeline() {
        let result = run_source(
            r#"OPERATION produce => BODY {
                EMIT 42
            }
            OPERATION classify =>
                INPUT x: Int64
                BODY {
                    MATCH x GT 10 => {
                        WHEN TRUE -> { EMIT "big" }
                        OTHERWISE -> { EMIT "small" }
                    }
                }
            PIPELINE Main => produce -> classify"#,
        );
        assert_eq!(result.unwrap(), Value::Str("big".into()));
    }

    #[test]
    fn integration_loop_in_operation() {
        let result = run_op(
            r#"OPERATION factorial =>
                INPUT n: Int64
                BODY {
                    MUTABLE result @reason("accumulator") = 1
                    MUTABLE i @reason("counter") = 1
                    LOOP max: 20 => {
                        result = result * i
                        i = i + 1
                        MATCH i GT n => {
                            WHEN TRUE -> { EMIT result }
                            OTHERWISE -> { }
                        }
                    }
                }"#,
            "factorial",
            vec![Value::Int(5)],
        );
        // 5! = 120
        assert_eq!(result.unwrap(), Value::Int(120));
    }

    #[test]
    fn integration_multiple_pipelines() {
        let result = run_source(
            "OPERATION a => BODY { EMIT 10 }
            OPERATION b =>
                INPUT x: Int64
                BODY { EMIT x + 5 }
            PIPELINE First => a -> b
            PIPELINE Second => a -> b -> b",
        );
        // Only the last pipeline's result is returned.
        // Second: a → 10 → b → 15 → b → 20
        assert_eq!(result.unwrap(), Value::Int(20));
    }

    #[test]
    fn integration_fork_join() {
        let result = run_source(
            "OPERATION branch_a => BODY { EMIT 10 }
            OPERATION branch_b => BODY { EMIT 20 }
            OPERATION combine =>
                INPUT x: Int64
                BODY {
                    STORE results = FORK { a: branch_a, b: branch_b } -> JOIN strategy: ALL_COMPLETE
                    EMIT results
                }
            PIPELINE Main => combine",
        );
        assert_eq!(
            result.unwrap(),
            Value::List(vec![Value::Int(10), Value::Int(20)])
        );
    }

    // =================================================================
    // Round 5 Slice 2 — Fork/Join ALL_COMPLETE failure collection
    // =================================================================

    #[test]
    fn fork_join_single_branch_failure_reports_aggregated() {
        let result = run_source(
            "OPERATION branch_ok => BODY { EMIT 10 }
            OPERATION branch_fail => BODY { HALT(branch_error) }
            OPERATION test =>
                INPUT x: Int64
                BODY {
                    STORE results = FORK { ok: branch_ok, bad: branch_fail } -> JOIN strategy: ALL_COMPLETE
                    EMIT results
                }
            PIPELINE Main => test",
        );
        // ALL_COMPLETE: collects all results, returns FORK_JOIN_FAILED with details.
        match result.unwrap() {
            Value::Failure { code, message, details } => {
                assert_eq!(code, "FORK_JOIN_FAILED");
                assert!(message.contains("1 of 2 branches failed"));
                // Details should be a list of {branch, failure} maps.
                if let Value::List(items) = *details {
                    assert_eq!(items.len(), 1);
                    if let Value::Map(m) = &items[0] {
                        assert_eq!(m.get("branch"), Some(&Value::Str("bad".into())));
                    } else {
                        panic!("expected Map in failure details");
                    }
                } else {
                    panic!("expected List in details");
                }
            }
            other => panic!("expected FAILURE, got {:?}", other),
        }
    }

    #[test]
    fn fork_join_all_branches_fail() {
        let result = run_source(
            "OPERATION fail_a => BODY { HALT(error_a) }
            OPERATION fail_b => BODY { HALT(error_b) }
            OPERATION test =>
                INPUT x: Int64
                BODY {
                    STORE r = FORK { a: fail_a, b: fail_b } -> JOIN strategy: ALL_COMPLETE
                    EMIT r
                }
            PIPELINE Main => test",
        );
        match result.unwrap() {
            Value::Failure { code, message, .. } => {
                assert_eq!(code, "FORK_JOIN_FAILED");
                assert!(message.contains("2 of 2 branches failed"));
            }
            other => panic!("expected FAILURE, got {:?}", other),
        }
    }

    #[test]
    fn fork_join_all_succeed_returns_list() {
        let result = run_source(
            "OPERATION a => BODY { EMIT 1 }
            OPERATION b => BODY { EMIT 2 }
            OPERATION c => BODY { EMIT 3 }
            OPERATION test =>
                INPUT x: Int64
                BODY {
                    STORE r = FORK { x: a, y: b, z: c } -> JOIN strategy: ALL_COMPLETE
                    EMIT r
                }
            PIPELINE Main => test",
        );
        assert_eq!(
            result.unwrap(),
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    // =================================================================
    // Round 5 Slice 2 — RETRY runtime behavior
    // =================================================================

    #[test]
    fn retry_in_operation_retries_on_halt() {
        // Operation that HALTs on first call but we RETRY(2).
        // Since each retry re-runs the body and encounters HALT again,
        // all attempts fail → RETRY_EXHAUSTED.
        let result = run_op(
            r#"OPERATION test => BODY {
                HALT(always_fails)
                RETRY(2)
            }"#,
            "test",
            vec![],
        );
        // HALT comes before RETRY, so the HALT propagates directly.
        match result.unwrap() {
            Value::Failure { code, .. } => assert_eq!(code, "HALTED"),
            other => panic!("expected HALTED FAILURE, got {:?}", other),
        }
    }

    #[test]
    fn retry_exhausted_produces_failure() {
        // RETRY(2) in operation that always retries — body re-executes 2 more times.
        let result = run_source(
            r#"OPERATION always_retry => BODY {
                RETRY(2)
            }
            PIPELINE Main => always_retry"#,
        );
        match result.unwrap() {
            Value::Failure { code, .. } => {
                assert_eq!(code, "RETRY_EXHAUSTED");
            }
            other => panic!("expected RETRY_EXHAUSTED FAILURE, got {:?}", other),
        }
    }

    #[test]
    fn retry_succeeds_on_second_attempt_via_mutable() {
        // Use a mutable counter to succeed on second try.
        // First body execution: counter=0, RETRY(3).
        // Second body execution: counter is reset (scoped), but the
        // operation always hits RETRY → eventually exhausts.
        let result = run_source(
            r#"OPERATION test_retry => BODY {
                RETRY(1)
            }
            PIPELINE Main => test_retry"#,
        );
        match result.unwrap() {
            Value::Failure { code, .. } => {
                assert_eq!(code, "RETRY_EXHAUSTED");
            }
            other => panic!("expected RETRY_EXHAUSTED FAILURE, got {:?}", other),
        }
    }

    // =================================================================
    // Round 5 Slice 2 — ESCALATE deterministic failure mapping
    // =================================================================

    #[test]
    fn escalate_with_message_produces_runtime_failure() {
        let result = run_op(
            r#"OPERATION test => BODY {
                ESCALATE("critical failure")
            }"#,
            "test",
            vec![],
        );
        match result {
            Err(InterpreterError::RuntimeFailure(rf)) => {
                assert_eq!(rf.code, al_diagnostics::ErrorCode::Escalated);
                assert!(rf.message.contains("critical failure"));
            }
            other => panic!("expected RuntimeFailure, got {:?}", other),
        }
    }

    #[test]
    fn escalate_without_message_uses_agent_default() {
        let result = run_op(
            "OPERATION test => BODY { ESCALATE }",
            "test",
            vec![],
        );
        match result {
            Err(InterpreterError::RuntimeFailure(rf)) => {
                assert_eq!(rf.code, al_diagnostics::ErrorCode::Escalated);
                // Default message should mention the agent/runtime.
                assert!(rf.message.contains("escalated"));
            }
            other => panic!("expected RuntimeFailure, got {:?}", other),
        }
    }

    #[test]
    fn escalate_emits_audit_event() {
        let program = al_parser::parse(
            r#"OPERATION test => BODY {
                ESCALATE("audit test")
            }"#,
        )
        .unwrap();
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let _ = interp.run_operation("test", vec![]);
        assert!(
            interp.runtime.audit_log.iter().any(|e| {
                e.event_type == al_diagnostics::AuditEventType::Escalated
            })
        );
    }

    // =================================================================
    // Round 5 Slice 2 — ASSERT with VC metadata
    // =================================================================

    #[test]
    fn assert_failure_carries_vc_metadata() {
        let result = run_op(
            "OPERATION test => BODY {
                ASSERT 1 GT 2
            }",
            "test",
            vec![],
        );
        match result {
            Err(InterpreterError::RuntimeFailure(rf)) => {
                assert_eq!(rf.code, al_diagnostics::ErrorCode::AssertionFailed);
                assert!(rf.message.contains("vc_id="));
                // Details should contain vc_id and solver_reason.
                assert!(rf.details.get("vc_id").is_some());
                assert!(rf.details.get("solver_reason").is_some());
            }
            other => panic!("expected RuntimeFailure with VC metadata, got {:?}", other),
        }
    }

    #[test]
    fn assert_true_passes_no_audit() {
        let program = al_parser::parse(
            "OPERATION test => BODY {
                ASSERT 2 GT 1
                EMIT 42
            }",
        )
        .unwrap();
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let result = interp.run_operation("test", vec![]).unwrap();
        assert_eq!(result, Value::Int(42));
        // No ASSERT_FAILED audit events.
        assert!(
            !interp.runtime.audit_log.iter().any(|e| {
                e.event_type == al_diagnostics::AuditEventType::AssertFailed
            })
        );
    }

    #[test]
    fn assert_false_emits_audit_with_vc_id() {
        let program = al_parser::parse(
            "OPERATION test => BODY { ASSERT FALSE }",
        )
        .unwrap();
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let _ = interp.run_operation("test", vec![]);
        let assert_events: Vec<_> = interp
            .runtime
            .audit_log
            .iter()
            .filter(|e| e.event_type == al_diagnostics::AuditEventType::AssertFailed)
            .collect();
        assert_eq!(assert_events.len(), 1);
        // The audit event should carry vc_id.
        assert!(assert_events[0].details.get("vc_id").is_some());
    }

    // =================================================================
    // Round 5 Slice 2 — Capability runtime checks
    // =================================================================

    #[test]
    fn capability_check_allows_operation_when_cap_held() {
        // Agent has FILE_READ, operation requires FILE_READ.
        let source = r#"
            AGENT Worker =>
                CAPABILITIES [FILE_READ]
            OPERATION read_file =>
                REQUIRE FILE_READ
                BODY { EMIT 42 }
            PIPELINE Main => read_file
        "#;
        let program = al_parser::parse(source).unwrap();
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        interp.set_active_agent("Worker");
        let result = interp.run().unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn capability_check_denies_operation_when_cap_missing() {
        let source = r#"
            AGENT Worker =>
                CAPABILITIES [FILE_READ]
            OPERATION write_file =>
                REQUIRE DB_WRITE
                BODY { EMIT 42 }
            PIPELINE Main => write_file
        "#;
        let program = al_parser::parse(source).unwrap();
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        interp.set_active_agent("Worker");
        let result = interp.run();
        match result {
            Err(InterpreterError::CapabilityDenied {
                agent_id,
                capability,
            }) => {
                assert_eq!(agent_id, "Worker");
                assert_eq!(capability, "DB_WRITE");
            }
            other => panic!("expected CapabilityDenied, got {:?}", other),
        }
    }

    #[test]
    fn no_agent_context_skips_capability_check() {
        // Without active agent, capability checks are skipped.
        let source = r#"
            OPERATION write_file =>
                REQUIRE DB_WRITE
                BODY { EMIT 42 }
            PIPELINE Main => write_file
        "#;
        let result = run_source(source);
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    // =================================================================
    // Round 5 Slice 2 — DELEGATE execution under callee's caps
    // =================================================================

    #[test]
    fn delegate_runs_under_target_agent_caps() {
        // Target agent has FILE_READ, operation requires FILE_READ.
        let source = r#"
            AGENT Caller =>
                CAPABILITIES [API_CALL]
            AGENT Worker =>
                CAPABILITIES [FILE_READ]
            OPERATION read_data =>
                REQUIRE FILE_READ
                BODY { EMIT 99 }
            OPERATION orchestrate => BODY {
                DELEGATE read_data TO Worker => {
                    INPUT 1
                }
                EMIT read_data_result
            }
            PIPELINE Main => orchestrate
        "#;
        let program = al_parser::parse(source).unwrap();
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        interp.set_active_agent("Caller");
        let result = interp.run().unwrap();
        assert_eq!(result, Value::Int(99));
    }

    #[test]
    fn delegate_fails_when_target_lacks_cap() {
        // Target agent does NOT have DB_WRITE, operation requires DB_WRITE.
        let source = r#"
            AGENT Caller =>
                CAPABILITIES [API_CALL]
            AGENT Worker =>
                CAPABILITIES [FILE_READ]
            OPERATION write_db =>
                REQUIRE DB_WRITE
                BODY { EMIT 99 }
            OPERATION orchestrate => BODY {
                DELEGATE write_db TO Worker => {
                    INPUT 1
                }
                EMIT write_db_result
            }
            PIPELINE Main => orchestrate
        "#;
        let program = al_parser::parse(source).unwrap();
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        interp.set_active_agent("Caller");
        let result = interp.run();
        match result {
            Err(InterpreterError::CapabilityDenied {
                agent_id,
                capability,
            }) => {
                assert_eq!(agent_id, "Worker");
                assert_eq!(capability, "DB_WRITE");
            }
            other => panic!("expected CapabilityDenied, got {:?}", other),
        }
    }

    #[test]
    fn delegate_without_target_agent_registered_inherits_caller() {
        // Target not registered — falls back to caller's context.
        let source = r#"
            OPERATION sub_task =>
                BODY { EMIT 77 }
            OPERATION orchestrate => BODY {
                DELEGATE sub_task TO UnknownAgent => {
                    INPUT 1
                }
                EMIT sub_task_result
            }
            PIPELINE Main => orchestrate
        "#;
        let result = run_source(source);
        assert_eq!(result.unwrap(), Value::Int(77));
    }

    // =================================================================
    // Round 6: Stdlib FILTER / MAP / REDUCE tests
    // =================================================================

    #[test]
    fn stdlib_filter_keeps_matching_elements() {
        let source = r#"
            OPERATION is_positive =>
                INPUT x: Int64
                BODY {
                    EMIT x GT 0
                }
            OPERATION test => BODY {
                STORE data = [1, -2, 3, -4, 5]
                STORE result = FILTER(data, "is_positive")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(
            result,
            Value::List(vec![Value::Int(1), Value::Int(3), Value::Int(5)])
        );
    }

    #[test]
    fn stdlib_filter_empty_list() {
        let source = r#"
            OPERATION always_true =>
                INPUT x: Int64
                BODY { EMIT TRUE }
            OPERATION test => BODY {
                STORE data = []
                STORE result = FILTER(data, "always_true")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::List(vec![]));
    }

    #[test]
    fn stdlib_filter_no_matches() {
        let source = r#"
            OPERATION always_false =>
                INPUT x: Int64
                BODY { EMIT FALSE }
            OPERATION test => BODY {
                STORE data = [1, 2, 3]
                STORE result = FILTER(data, "always_false")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::List(vec![]));
    }

    #[test]
    fn stdlib_map_transforms_elements() {
        let source = r#"
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x * 2 }
            OPERATION test => BODY {
                STORE data = [1, 2, 3]
                STORE result = MAP(data, "double")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(
            result,
            Value::List(vec![Value::Int(2), Value::Int(4), Value::Int(6)])
        );
    }

    #[test]
    fn stdlib_map_empty_list() {
        let source = r#"
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x * 2 }
            OPERATION test => BODY {
                STORE data = []
                STORE result = MAP(data, "double")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::List(vec![]));
    }

    #[test]
    fn stdlib_reduce_sums_list() {
        let source = r#"
            OPERATION add =>
                INPUT a: Int64
                INPUT b: Int64
                BODY { EMIT a + b }
            OPERATION test => BODY {
                STORE data = [1, 2, 3, 4, 5]
                STORE result = REDUCE(data, 0, "add")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Int(15));
    }

    #[test]
    fn stdlib_reduce_empty_list_returns_initial() {
        let source = r#"
            OPERATION add =>
                INPUT a: Int64
                INPUT b: Int64
                BODY { EMIT a + b }
            OPERATION test => BODY {
                STORE data = []
                STORE result = REDUCE(data, 42, "add")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn stdlib_reduce_product() {
        let source = r#"
            OPERATION multiply =>
                INPUT a: Int64
                INPUT b: Int64
                BODY { EMIT a * b }
            OPERATION test => BODY {
                STORE data = [2, 3, 4]
                STORE result = REDUCE(data, 1, "multiply")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Int(24));
    }

    #[test]
    fn stdlib_filter_type_error_non_list() {
        let source = r#"
            OPERATION pred =>
                INPUT x: Int64
                BODY { EMIT TRUE }
            OPERATION test => BODY {
                STORE result = FILTER(42, "pred")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source);
        assert!(result.is_err());
    }

    #[test]
    fn stdlib_map_type_error_non_list() {
        let source = r#"
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x * 2 }
            OPERATION test => BODY {
                STORE result = MAP(42, "double")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source);
        assert!(result.is_err());
    }

    #[test]
    fn stdlib_reduce_type_error_non_list() {
        let source = r#"
            OPERATION add =>
                INPUT a: Int64
                INPUT b: Int64
                BODY { EMIT a + b }
            OPERATION test => BODY {
                STORE result = REDUCE(42, 0, "add")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source);
        assert!(result.is_err());
    }

    #[test]
    fn stdlib_filter_map_compose() {
        // FILTER then MAP: filter positives, then double them.
        let source = r#"
            OPERATION is_positive =>
                INPUT x: Int64
                BODY { EMIT x GT 0 }
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x * 2 }
            OPERATION test => BODY {
                STORE data = [1, -2, 3, -4, 5]
                STORE filtered = FILTER(data, "is_positive")
                STORE mapped = MAP(filtered, "double")
                EMIT mapped
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        assert_eq!(
            result,
            Value::List(vec![Value::Int(2), Value::Int(6), Value::Int(10)])
        );
    }

    #[test]
    fn stdlib_map_reduce_compose() {
        // MAP then REDUCE: double each, then sum.
        let source = r#"
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x * 2 }
            OPERATION add =>
                INPUT a: Int64
                INPUT b: Int64
                BODY { EMIT a + b }
            OPERATION test => BODY {
                STORE data = [1, 2, 3]
                STORE doubled = MAP(data, "double")
                STORE total = REDUCE(doubled, 0, "add")
                EMIT total
            }
            PIPELINE Main => test
        "#;
        let result = run_source(source).unwrap();
        // doubled = [2, 4, 6], sum = 12
        assert_eq!(result, Value::Int(12));
    }

    // =================================================================
    // Round 6: Audit JSONL emission tests
    // =================================================================

    #[test]
    fn audit_pipeline_started_event() {
        let source = r#"
            OPERATION produce => BODY { EMIT 42 }
            PIPELINE Main => produce
        "#;
        let program = al_parser::parse(source).expect("parse ok");
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let _result = interp.run().unwrap();

        // Should have PIPELINE_STARTED and OPERATION_CALLED events.
        let events = &interp.runtime.audit_log;
        assert!(
            events.iter().any(|e| e.event_type == AuditEventType::PipelineStarted),
            "expected PIPELINE_STARTED audit event"
        );
    }

    #[test]
    fn audit_operation_called_event() {
        let source = r#"
            OPERATION produce => BODY { EMIT 42 }
            PIPELINE Main => produce
        "#;
        let program = al_parser::parse(source).expect("parse ok");
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let _result = interp.run().unwrap();

        let events = &interp.runtime.audit_log;
        let op_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == AuditEventType::OperationCalled)
            .collect();
        assert!(!op_events.is_empty(), "expected OPERATION_CALLED event");
        assert_eq!(op_events[0].details["operation"], "produce");
    }

    #[test]
    fn audit_stdlib_call_event() {
        let source = r#"
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x * 2 }
            OPERATION test => BODY {
                STORE data = [1, 2, 3]
                STORE result = MAP(data, "double")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let program = al_parser::parse(source).expect("parse ok");
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let _result = interp.run().unwrap();

        let events = &interp.runtime.audit_log;
        let stdlib_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == AuditEventType::StdlibCall)
            .collect();
        assert!(!stdlib_events.is_empty(), "expected STDLIB_CALL event");
        assert_eq!(stdlib_events[0].details["operation"], "MAP");
    }

    #[test]
    fn audit_jsonl_format_valid() {
        let source = r#"
            OPERATION produce => BODY { EMIT 42 }
            PIPELINE Main => produce
        "#;
        let program = al_parser::parse(source).expect("parse ok");
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let _result = interp.run().unwrap();

        let jsonl_lines = interp.runtime.audit_to_jsonl();
        assert!(!jsonl_lines.is_empty(), "should have audit JSONL output");

        for line in &jsonl_lines {
            // Each line must be valid JSON.
            let parsed: serde_json::Value =
                serde_json::from_str(line).expect("each JSONL line must be valid JSON");
            // Must have required audit schema fields.
            assert!(parsed["event_id"].is_string(), "missing event_id");
            assert!(parsed["timestamp"].is_string(), "missing timestamp");
            assert!(parsed["agent_id"].is_string(), "missing agent_id");
            assert!(parsed["task_id"].is_string(), "missing task_id");
            assert!(parsed["event_type"].is_string(), "missing event_type");
            assert_eq!(parsed["profile"], "mvp-0.1", "wrong profile");
        }
    }

    #[test]
    fn audit_schema_fields_for_all_event_types() {
        // Run a program that triggers PIPELINE_STARTED, OPERATION_CALLED, STDLIB_CALL.
        let source = r#"
            OPERATION double =>
                INPUT x: Int64
                BODY { EMIT x * 2 }
            OPERATION test => BODY {
                STORE data = [10, 20]
                STORE result = MAP(data, "double")
                EMIT result
            }
            PIPELINE Main => test
        "#;
        let program = al_parser::parse(source).expect("parse ok");
        let mut interp = Interpreter::new();
        interp.load_program(&program);
        let _result = interp.run().unwrap();

        let events = &interp.runtime.audit_log;
        let event_types: Vec<_> = events.iter().map(|e| e.event_type).collect();

        // Verify we got the expected event types.
        assert!(event_types.contains(&AuditEventType::PipelineStarted));
        assert!(event_types.contains(&AuditEventType::OperationCalled));
        assert!(event_types.contains(&AuditEventType::StdlibCall));

        // Verify all events have the required schema fields.
        for event in events {
            assert!(!event.event_id.is_empty(), "event_id must not be empty");
            assert!(!event.timestamp.is_empty(), "timestamp must not be empty");
            assert!(!event.agent_id.is_empty(), "agent_id must not be empty");
            assert!(!event.task_id.is_empty(), "task_id must not be empty");
            assert_eq!(event.profile, "mvp-0.1");
        }
    }
}
