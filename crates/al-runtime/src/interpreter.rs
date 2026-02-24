//! # AgentLang Interpreter — Round 5 MVP
//!
//! Tree-walking interpreter that executes AgentLang programs end-to-end.
//!
//! Implements:
//! - **Statement interpreter**: STORE, MUTABLE, ASSIGN, MATCH, LOOP, EMIT, HALT
//! - **Expression evaluator**: literals, identifiers, binary/unary ops, member
//!   access, list/map constructors, operation calls
//! - **Pattern matching**: wildcard, literal, SUCCESS/FAILURE destructuring,
//!   identifier binding
//! - **Pipeline execution**: output threading with short-circuit on FAILURE

use std::collections::{BTreeMap, HashMap, HashSet};

use al_ast::*;
use al_diagnostics::RuntimeFailure;

use crate::{Runtime, Value};

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
}

/// Result of executing a single statement.
enum StmtResult {
    /// Continue to the next statement.
    Continue,
    /// An EMIT was executed; propagate the value upward.
    Emit(Value),
    /// A HALT was executed.
    Halt { reason: String, value: Value },
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
        }
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
                    name, inputs, body, ..
                } => {
                    let param_names: Vec<String> =
                        inputs.iter().map(|p| p.node.name.node.clone()).collect();
                    self.operations.insert(
                        name.node.clone(),
                        OperationDef {
                            name: name.node.clone(),
                            inputs: param_names,
                            body: body.clone(),
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

        // Save caller state.
        let saved_regs = self.runtime.registers.clone();
        let saved_mutables = self.mutables.clone();

        // Bind positional inputs.
        for (i, param) in op.inputs.iter().enumerate() {
            if let Some(arg) = args.get(i) {
                self.runtime.reg_set(param.clone(), arg.clone());
            }
        }
        // If there is a threaded input but no declared parameters, bind as `_input`.
        if !args.is_empty() && op.inputs.is_empty() {
            self.runtime.reg_set("_input", args[0].clone());
        }

        // Execute the operation body.
        let result = self.exec_block(&op.body);

        // Restore caller state.
        self.runtime.registers = saved_regs;
        self.mutables = saved_mutables;

        match result {
            Ok(Some(val)) => Ok(val),
            Ok(None) => Ok(Value::None),
            Err(InterpreterError::Halted { reason, value }) => {
                // HALT inside an operation produces a FAILURE value (not a crash).
                Ok(Value::Failure {
                    code: "HALTED".to_string(),
                    message: reason,
                    details: Box::new(value),
                })
            }
            Err(e) => Err(e),
        }
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

            // ----- ASSERT ---------------------------------------------------
            Statement::Assert { condition } => {
                let val = self.eval_expr(condition)?;
                match val {
                    Value::Bool(true) => Ok(StmtResult::Continue),
                    Value::Bool(false) => {
                        self.runtime
                            .execute_assert(false, "runtime", "assertion failed")?;
                        unreachable!()
                    }
                    _ => Err(InterpreterError::TypeError(
                        "ASSERT condition must be boolean".to_string(),
                    )),
                }
            }

            // ----- RETRY ----------------------------------------------------
            Statement::Retry { .. } => {
                // MVP: RETRY as a standalone statement is a no-op marker.
                // Real retry semantics are handled at the pipeline/operation level.
                Ok(StmtResult::Continue)
            }

            // ----- ESCALATE -------------------------------------------------
            Statement::Escalate { message } => {
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
                let failure = self.runtime.execute_escalate(msg, "runtime");
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

            // ----- DELEGATE --------------------------------------------------
            Statement::Delegate {
                task,
                target: _,
                clauses,
            } => {
                let mut input_val = Value::None;
                for clause in clauses {
                    if let DelegateClause::Input(expr) = &clause.node {
                        input_val = self.eval_expr(expr)?;
                    }
                }
                let result = self.call_operation(&task.node, vec![input_val])?;
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
                for (_name, chain) in &branch_data {
                    let result = self.exec_pipeline_chain(chain, Value::None)?;
                    if let Value::Failure { .. } = &result {
                        return Ok(result);
                    }
                    results.push(result);
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
}
