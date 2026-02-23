//! # AgentLang Abstract Syntax Tree (MVP v0.1)
//!
//! This crate defines the complete set of AST node types that mirror the
//! grammar specified in `specs/GRAMMAR_MVP.ebnf`.  Every node carries a
//! [`Span`] so that later compiler stages can produce precise diagnostics.

use al_diagnostics::Span;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Span wrapper
// ---------------------------------------------------------------------------

/// A generic wrapper that pairs any AST node with its source location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Wrap a node together with its source span.
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

// ---------------------------------------------------------------------------
// Top-level: Program
// ---------------------------------------------------------------------------

/// A complete AgentLang source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub declarations: Vec<Spanned<Declaration>>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Declarations
// ---------------------------------------------------------------------------

/// Top-level declaration variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Declaration {
    /// `TYPE Name[T] = type_expr ;`
    TypeDecl {
        name: Spanned<String>,
        type_params: Vec<Spanned<String>>,
        ty: Spanned<TypeExpr>,
    },

    /// `SCHEMA Name => { fields }`
    SchemaDecl {
        name: Spanned<String>,
        fields: Vec<Spanned<FieldDecl>>,
    },

    /// `AGENT Name => { properties }`
    AgentDecl {
        name: Spanned<String>,
        properties: Vec<Spanned<AgentProperty>>,
    },

    /// `OPERATION Name => INPUT ... OUTPUT ... REQUIRE ... ENSURE ... INVARIANT ... BODY { }`
    OperationDecl {
        name: Spanned<String>,
        inputs: Vec<Spanned<Parameter>>,
        output: Option<Spanned<TypeExpr>>,
        requires: Vec<Spanned<Expr>>,
        ensures: Vec<Spanned<Expr>>,
        invariants: Vec<Spanned<Expr>>,
        body: Spanned<Block>,
    },

    /// `PIPELINE Name => chain ;`
    PipelineDecl {
        name: Spanned<String>,
        chain: Spanned<PipelineChain>,
    },
}

// ---------------------------------------------------------------------------
// Pipeline chain
// ---------------------------------------------------------------------------

/// A pipeline chain: an initial expression followed by zero or more
/// `pipe_operator pipe_stage` pairs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineChain {
    pub stages: Vec<PipelineStage>,
}

/// A single stage inside a pipeline chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineStage {
    /// The pipe operator that precedes this stage (`None` for the first stage).
    pub op: Option<Spanned<PipeOp>>,
    pub expr: Spanned<Expr>,
}

// ---------------------------------------------------------------------------
// Fields & Parameters
// ---------------------------------------------------------------------------

/// A field inside a `SCHEMA` or `STATE_SCHEMA` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDecl {
    pub name: Spanned<String>,
    pub ty: Spanned<TypeExpr>,
    pub constraint: Option<Spanned<ConstraintExpr>>,
}

/// A parameter in an `INPUT` clause: `name : type_expr`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: Spanned<String>,
    pub ty: Spanned<TypeExpr>,
}

// ---------------------------------------------------------------------------
// Agent properties
// ---------------------------------------------------------------------------

/// Properties that may appear inside an `AGENT` declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentProperty {
    /// `CAPABILITIES [cap1, cap2, ...]`
    Capabilities(Vec<Spanned<String>>),

    /// `DENY [cap1, cap2, ...]`
    Deny(Vec<Spanned<String>>),

    /// `TRUST_LEVEL ~0.95`
    TrustLevel(Spanned<f64>),

    /// `MAX_CONCURRENCY 4`
    MaxConcurrency(Spanned<i64>),

    /// `MEMORY_LIMIT 256MB`
    MemoryLimit(Spanned<String>),

    /// `TIMEOUT_DEFAULT 30s`
    TimeoutDefault(Spanned<String>),

    /// `ON_FAILURE failure_policy`
    OnFailure(Spanned<FailurePolicy>),

    /// `STATE_SCHEMA => { fields }`
    StateSchema(Vec<Spanned<FieldDecl>>),
}

// ---------------------------------------------------------------------------
// Statements & Block
// ---------------------------------------------------------------------------

/// A brace-delimited sequence of statements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub stmts: Vec<Spanned<Statement>>,
}

/// Statement variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// `STORE name [: ty] = expr ;`
    Store {
        name: Spanned<String>,
        ty: Option<Spanned<TypeExpr>>,
        value: Spanned<Expr>,
    },

    /// `MUTABLE name @reason("...") [: ty] = expr ;`
    Mutable {
        name: Spanned<String>,
        reason: Spanned<String>,
        ty: Option<Spanned<TypeExpr>>,
        value: Spanned<Expr>,
    },

    /// `name = expr ;`
    Assign {
        target: Spanned<String>,
        value: Spanned<Expr>,
    },

    /// `MATCH expr => { arms [OTHERWISE -> ...] }`
    Match {
        expr: Spanned<Expr>,
        arms: Vec<Spanned<MatchArm>>,
        otherwise: Option<Spanned<MatchBody>>,
    },

    /// `LOOP max: N => { body }`
    Loop {
        max_iters: Spanned<i64>,
        body: Spanned<Block>,
    },

    /// `EMIT [expr] ;`
    Emit {
        value: Option<Spanned<Expr>>,
    },

    /// `ASSERT expr ;`
    Assert {
        condition: Spanned<Expr>,
    },

    /// `RETRY(count [, args]) ;`
    Retry {
        count: Spanned<i64>,
        args: Vec<Spanned<Argument>>,
    },

    /// `ESCALATE [( expr )] ;`
    Escalate {
        message: Option<Spanned<Expr>>,
    },

    /// `CHECKPOINT ["label"] ;`
    Checkpoint {
        label: Option<Spanned<String>>,
    },

    /// `HALT(reason [, expr]) ;`
    Halt {
        reason: Spanned<String>,
        value: Option<Spanned<Expr>>,
    },

    /// `DELEGATE task TO target => { clauses }`
    Delegate {
        task: Spanned<String>,
        target: Spanned<String>,
        clauses: Vec<Spanned<DelegateClause>>,
    },

    /// A bare expression used as a statement: `expr ;`
    Expr {
        expr: Spanned<Expr>,
    },
}

// ---------------------------------------------------------------------------
// Match
// ---------------------------------------------------------------------------

/// A single arm inside a `MATCH` expression.
///
/// ```text
/// WHEN pattern -> match_body
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Spanned<Pattern>,
    pub body: Spanned<MatchBody>,
}

/// The right-hand side of a match arm: either a block or a single expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchBody {
    Block(Spanned<Block>),
    Expr(Spanned<Expr>),
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

/// Pattern variants used in `MATCH` arms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    /// `_` — matches anything.
    Wildcard,

    /// A literal value (`42`, `"hello"`, `TRUE`, etc.).
    Literal(Literal),

    /// `SUCCESS(inner_pattern)`
    Success(Box<Spanned<Pattern>>),

    /// `FAILURE(error_code, msg_pattern, details_pattern)`
    Failure {
        code: Spanned<String>,
        msg_pat: Box<Spanned<Pattern>>,
        details_pat: Box<Spanned<Pattern>>,
    },

    /// `Name(arg_patterns...)`
    Constructor {
        name: Spanned<String>,
        args: Vec<Spanned<Pattern>>,
    },

    /// A bare identifier used as a binding or enum variant reference.
    Identifier(String),
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// Expression variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// A literal value.
    Literal(Literal),

    /// A simple identifier reference.
    Identifier(String),

    /// `left op right`
    BinaryOp {
        left: Box<Spanned<Expr>>,
        op: Spanned<BinaryOp>,
        right: Box<Spanned<Expr>>,
    },

    /// `op operand`  (prefix unary)
    UnaryOp {
        op: Spanned<UnaryOp>,
        operand: Box<Spanned<Expr>>,
    },

    /// `func(args)`
    Call {
        func: Box<Spanned<Expr>>,
        args: Vec<Spanned<Argument>>,
    },

    /// `object.field`
    Member {
        object: Box<Spanned<Expr>>,
        field: Spanned<String>,
    },

    /// `expr?`  — confidence query postfix.
    Confidence {
        expr: Box<Spanned<Expr>>,
    },

    /// `start..end`
    Range {
        start: Box<Spanned<Expr>>,
        end: Box<Spanned<Expr>>,
    },

    /// `left -> right`  or  `left |> right`
    Pipeline {
        left: Box<Spanned<Expr>>,
        op: Spanned<PipeOp>,
        right: Box<Spanned<Expr>>,
    },

    /// `FORK { branches } -> JOIN strategy: ALL_COMPLETE`
    Fork {
        branches: Vec<Spanned<ForkBranch>>,
        join: Spanned<JoinStrategy>,
    },

    /// `RESUME(expr)`
    Resume {
        expr: Box<Spanned<Expr>>,
    },

    /// `[a, b, c]`
    List {
        elements: Vec<Spanned<Expr>>,
    },

    /// `{ "key": value, ... }`  or  `{ ident: value, ... }`
    Map {
        items: Vec<Spanned<MapItem>>,
    },

    /// Parenthesised expression `( expr )`.
    Paren {
        inner: Box<Spanned<Expr>>,
    },
}

// ---------------------------------------------------------------------------
// Map items
// ---------------------------------------------------------------------------

/// A single key-value pair inside a map literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapItem {
    pub key: Spanned<MapKey>,
    pub value: Spanned<Expr>,
}

/// The key of a map item: either a string literal or an identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MapKey {
    String(String),
    Identifier(String),
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

/// Binary infix operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    // Logical
    And,
    Or,
}

/// Prefix unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnaryOp {
    /// `NOT`
    Not,
    /// `-` (arithmetic negation)
    Neg,
}

/// Pipeline / pipe operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipeOp {
    /// `->`
    Arrow,
    /// `|>`
    PipeForward,
}

// ---------------------------------------------------------------------------
// Fork / Join
// ---------------------------------------------------------------------------

/// A named branch inside a `FORK` expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForkBranch {
    pub name: Spanned<String>,
    pub chain: Spanned<PipelineChain>,
}

/// Join strategies for `FORK` expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JoinStrategy {
    /// `ALL_COMPLETE` — wait for every branch.
    AllComplete,
}

// ---------------------------------------------------------------------------
// Arguments
// ---------------------------------------------------------------------------

/// A positional or named argument: `[name:] expr`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Argument {
    /// `Some("param_name")` for named arguments, `None` for positional.
    pub name: Option<Spanned<String>>,
    pub value: Spanned<Expr>,
}

// ---------------------------------------------------------------------------
// Delegate clauses
// ---------------------------------------------------------------------------

/// Clauses inside a `DELEGATE ... TO ... => { }` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DelegateClause {
    /// `INPUT expr ;`
    Input(Spanned<Expr>),

    /// `TIMEOUT 30s ;`
    Timeout(Spanned<String>),

    /// `ON_TIMEOUT failure_policy ;`
    OnTimeout(Spanned<FailurePolicy>),

    /// `SHARED_CONTEXT [ident, ident, ...] ;`
    SharedContext(Vec<Spanned<String>>),

    /// `ISOLATION { rule, ... }`
    Isolation(Vec<Spanned<IsolationRule>>),
}

/// A single `key : value` rule inside an `ISOLATION` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsolationRule {
    pub key: Spanned<String>,
    pub value: Spanned<String>,
}

// ---------------------------------------------------------------------------
// Failure policy
// ---------------------------------------------------------------------------

/// A chain of recovery steps: `RETRY(3) -> REASSIGN(backup) -> ESCALATE`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailurePolicy {
    pub steps: Vec<Spanned<PolicyStep>>,
}

/// Individual steps in a failure-recovery pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyStep {
    /// `RETRY(count [, args])`
    Retry {
        count: Spanned<i64>,
        args: Vec<Spanned<Argument>>,
    },

    /// `REASSIGN(target_agent)`
    Reassign(Spanned<String>),

    /// `ESCALATE [( message )]`
    Escalate(Option<Spanned<Expr>>),

    /// `ABORT`
    Abort,
}

// ---------------------------------------------------------------------------
// Type expressions
// ---------------------------------------------------------------------------

/// Type expression variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeExpr {
    /// A named type, optionally with type parameters:
    /// `Int64`, `List[Int64]`, `Result[T, E]`.
    Named {
        name: Spanned<String>,
        params: Vec<Spanned<TypeExpr>>,
    },

    /// An inline record type: `{ name: Str, age: Int64 }`.
    Record {
        fields: Vec<Spanned<FieldType>>,
    },

    /// A union type: `Success | Failure`.
    Union {
        types: Vec<Spanned<TypeExpr>>,
    },

    /// A constrained type: `Int64 :: range(0, 100)`.
    Constrained {
        ty: Box<Spanned<TypeExpr>>,
        constraint: Spanned<ConstraintExpr>,
    },
}

/// A field inside an inline record type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldType {
    pub name: Spanned<String>,
    pub ty: Spanned<TypeExpr>,
}

// ---------------------------------------------------------------------------
// Constraints
// ---------------------------------------------------------------------------

/// A constraint annotation: `range(0, 100)`, `max_length(255)`, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstraintExpr {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<Argument>>,
}

// ---------------------------------------------------------------------------
// Literals
// ---------------------------------------------------------------------------

/// Literal value variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    /// `42`, `-7`, `0xFF`, `0b1010`
    Integer(i64),
    /// `3.14`, `1.0e-10`
    Float(f64),
    /// `"hello"`
    String(String),
    /// `TRUE` / `FALSE`
    Bool(bool),
    /// `NONE`
    None,
    /// `5s`, `100ms`, `2m`, `1h`
    Duration(String),
    /// `256KB`, `1MB`, `4GB`
    Size(String),
    /// `~0.95`
    Confidence(f64),
    /// `SHA256:a3f8...`
    Hash(String),
}
