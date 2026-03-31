//! AgentLang Verification Condition (VC) generation and solving.
//!
//! MVP v0.1 includes:
//! - VC generation from `REQUIRE`, `ENSURE`, and explicit `ASSERT`.
//! - A configurable stub solver (`Valid`/`Invalid`/`Unknown`).
//! - Unknown-result plumbing to inject synthetic HIR `ASSERT` nodes.
//! - Invalid-result diagnostics (`VC_INVALID`).
//!
//! Phase 1 additions:
//! - `Solver` trait abstracting over solver backends.
//! - `SmtExpr` intermediate representation for solver-agnostic translation.
//! - `SmtTranslator` converting AST expressions to `SmtExpr`.
//! - `SimpleSolver` proving common patterns from premises.
//! - Feature-gated `Z3Solver` for full SMT power (future).

pub mod simple_solver;
pub mod smt;

use al_ast::{Declaration, Expr, MatchBody, Statement};
use al_diagnostics::{Diagnostic, DiagnosticSink, ErrorCode, Span};
use al_hir::{HirDeclaration, HirExpr, HirExprKind, HirMeta, HirProgram, HirStatement};
use std::collections::HashMap;

/// Result of attempting to verify a condition.
#[derive(Debug, Clone, PartialEq)]
pub enum VcResult {
    /// The condition is provably valid.
    Valid,
    /// The condition is provably invalid.
    Invalid { counterexample: String },
    /// The solver could not determine validity.
    Unknown { reason: String },
}

/// The source location/origin for a generated VC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VcOrigin {
    Require,
    Ensure,
    InvariantLoopEntry,
    InvariantIterationBoundary,
    Assert { synthetic: bool },
}

/// A verification condition extracted from the program.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationCondition {
    pub vc_id: String,
    pub operation: String,
    pub origin: VcOrigin,
    pub description: String,
    pub span: Span,
    pub result: Option<VcResult>,
    /// The expression AST for this VC (for solver translation).
    pub expr: Option<Expr>,
    /// Premise expressions (REQUIRE clauses) available when solving this VC.
    pub premises: Vec<Expr>,
}

impl VerificationCondition {
    fn new(
        vc_id: String,
        operation: String,
        origin: VcOrigin,
        description: String,
        span: Span,
    ) -> Self {
        Self {
            vc_id,
            operation,
            origin,
            description,
            span,
            result: None,
            expr: None,
            premises: Vec::new(),
        }
    }

    fn with_expr(mut self, expr: Expr) -> Self {
        self.expr = Some(expr);
        self
    }

    fn with_premises(mut self, premises: Vec<Expr>) -> Self {
        self.premises = premises;
        self
    }
}

/// A runtime assertion rewrite generated for `Unknown` VC results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticAssertRewrite {
    pub operation: String,
    pub vc_id: String,
    pub solver_reason: String,
}

// ---------------------------------------------------------------------------
// Solver trait
// ---------------------------------------------------------------------------

/// Configuration for solver execution.
#[derive(Debug, Clone)]
pub struct SolverConfig {
    /// Timeout in milliseconds per VC. `0` means no timeout.
    pub timeout_ms: u64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self { timeout_ms: 5000 }
    }
}

/// Abstract solver backend. Implement this to add new solver backends.
pub trait Solver: Send + Sync {
    /// Attempt to solve a verification condition.
    fn solve(&self, vc: &mut VerificationCondition, config: &SolverConfig) -> VcResult;

    /// Human-readable name of this solver backend.
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// VC Generator
// ---------------------------------------------------------------------------

/// Generates VCs from an AST program.
pub struct VcGenerator {
    next_id: u64,
}

impl VcGenerator {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    pub fn generate_program(&mut self, program: &al_ast::Program) -> Vec<VerificationCondition> {
        let mut vcs = Vec::new();
        for decl in &program.declarations {
            if let Declaration::OperationDecl {
                name,
                requires,
                ensures,
                invariants,
                body,
                ..
            } = &decl.node
            {
                // Collect REQUIRE expressions as premises for ENSURE/ASSERT VCs.
                let premises: Vec<Expr> = requires.iter().map(|r| r.node.clone()).collect();

                for req in requires {
                    // REQUIRE VCs have no premises (they ARE the premises).
                    // We mark them as Valid by convention — they are axioms
                    // from the caller's obligation.
                    let vc = self
                        .make_vc(name.node.as_str(), VcOrigin::Require, req)
                        .with_expr(req.node.clone());
                    vcs.push(vc);
                }
                for ens in ensures {
                    let vc = self
                        .make_vc(name.node.as_str(), VcOrigin::Ensure, ens)
                        .with_expr(ens.node.clone())
                        .with_premises(premises.clone());
                    vcs.push(vc);
                }
                for invariant in invariants {
                    let vc_entry = self
                        .make_vc(name.node.as_str(), VcOrigin::InvariantLoopEntry, invariant)
                        .with_expr(invariant.node.clone())
                        .with_premises(premises.clone());
                    let vc_iter = self
                        .make_vc(
                            name.node.as_str(),
                            VcOrigin::InvariantIterationBoundary,
                            invariant,
                        )
                        .with_expr(invariant.node.clone())
                        .with_premises(premises.clone());
                    vcs.push(vc_entry);
                    vcs.push(vc_iter);
                }
                for stmt in &body.node.stmts {
                    self.collect_assert_vcs(
                        name.node.as_str(),
                        &stmt.node,
                        stmt.span,
                        &premises,
                        &mut vcs,
                    );
                }
            }
        }
        vcs
    }

    fn collect_assert_vcs(
        &mut self,
        operation: &str,
        stmt: &Statement,
        stmt_span: Span,
        premises: &[Expr],
        out: &mut Vec<VerificationCondition>,
    ) {
        match stmt {
            Statement::Assert { condition } => {
                let vc = self
                    .make_vc(operation, VcOrigin::Assert { synthetic: false }, condition)
                    .with_expr(condition.node.clone())
                    .with_premises(premises.to_vec());
                out.push(vc);
            }
            Statement::Match {
                arms, otherwise, ..
            } => {
                for arm in arms {
                    if let MatchBody::Block(block) = &arm.node.body.node {
                        for nested in &block.node.stmts {
                            self.collect_assert_vcs(
                                operation,
                                &nested.node,
                                nested.span,
                                premises,
                                out,
                            );
                        }
                    }
                }
                if let Some(otherwise) = otherwise {
                    if let MatchBody::Block(block) = &otherwise.node {
                        for nested in &block.node.stmts {
                            self.collect_assert_vcs(
                                operation,
                                &nested.node,
                                nested.span,
                                premises,
                                out,
                            );
                        }
                    }
                }
            }
            Statement::Loop { body, .. } => {
                for nested in &body.node.stmts {
                    self.collect_assert_vcs(operation, &nested.node, nested.span, premises, out);
                }
            }
            _ => {
                let _ = stmt_span;
            }
        }
    }

    fn make_vc(
        &mut self,
        operation: &str,
        origin: VcOrigin,
        expr: &al_ast::Spanned<Expr>,
    ) -> VerificationCondition {
        let vc_id = format!("vc_{:06}", self.next_id);
        self.next_id += 1;
        let description = format!("{:?}", expr.node);
        VerificationCondition::new(vc_id, operation.to_string(), origin, description, expr.span)
    }
}

impl Default for VcGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Stub Solver (MVP legacy)
// ---------------------------------------------------------------------------

/// Stub solver mode for MVP.
#[derive(Debug, Clone, PartialEq)]
pub enum StubSolverMode {
    AlwaysValid,
    AlwaysInvalid { counterexample: String },
    AlwaysUnknown { reason: String },
}

/// Configurable solver configuration for tests and MVP compilation.
#[derive(Debug, Clone)]
pub struct StubSolverConfig {
    pub default_mode: StubSolverMode,
    pub per_vc: HashMap<String, StubSolverMode>,
}

impl Default for StubSolverConfig {
    fn default() -> Self {
        Self {
            default_mode: StubSolverMode::AlwaysUnknown {
                reason: "MVP stub solver: no SMT backend configured".to_string(),
            },
            per_vc: HashMap::new(),
        }
    }
}

/// MVP stub solver with configurable outputs.
#[derive(Debug, Clone)]
pub struct StubSolver {
    config: StubSolverConfig,
}

impl StubSolver {
    pub fn new(config: StubSolverConfig) -> Self {
        Self { config }
    }

    /// Legacy solve method for backward compatibility.
    pub fn solve_legacy<'a>(&self, vc: &'a mut VerificationCondition) -> &'a VcResult {
        let mode = self
            .config
            .per_vc
            .get(&vc.vc_id)
            .unwrap_or(&self.config.default_mode);
        let result = match mode {
            StubSolverMode::AlwaysValid => VcResult::Valid,
            StubSolverMode::AlwaysInvalid { counterexample } => VcResult::Invalid {
                counterexample: counterexample.clone(),
            },
            StubSolverMode::AlwaysUnknown { reason } => VcResult::Unknown {
                reason: reason.clone(),
            },
        };
        vc.result = Some(result);
        vc.result.as_ref().expect("solver always sets result")
    }
}

impl Default for StubSolver {
    fn default() -> Self {
        Self::new(StubSolverConfig::default())
    }
}

impl Solver for StubSolver {
    fn solve(&self, vc: &mut VerificationCondition, _config: &SolverConfig) -> VcResult {
        let mode = self
            .config
            .per_vc
            .get(&vc.vc_id)
            .unwrap_or(&self.config.default_mode);
        match mode {
            StubSolverMode::AlwaysValid => VcResult::Valid,
            StubSolverMode::AlwaysInvalid { counterexample } => VcResult::Invalid {
                counterexample: counterexample.clone(),
            },
            StubSolverMode::AlwaysUnknown { reason } => VcResult::Unknown {
                reason: reason.clone(),
            },
        }
    }

    fn name(&self) -> &str {
        "stub"
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Check if a VC result requires runtime assertion insertion.
pub fn needs_runtime_assert(result: &VcResult) -> bool {
    matches!(result, VcResult::Unknown { .. })
}

/// Check if a VC result is a compile-time error.
pub fn is_compile_error(result: &VcResult) -> bool {
    matches!(result, VcResult::Invalid { .. })
}

/// Get the error code for an invalid VC.
pub fn error_code_for_result(result: &VcResult) -> Option<ErrorCode> {
    match result {
        VcResult::Invalid { .. } => Some(ErrorCode::VcInvalid),
        _ => None,
    }
}

/// Convert solved VC results into diagnostics and synthetic-assert rewrites.
pub fn apply_vc_results(
    vcs: &[VerificationCondition],
    hir: &mut HirProgram,
    sink: &mut DiagnosticSink,
) -> Vec<SyntheticAssertRewrite> {
    let mut rewrites = Vec::new();
    for vc in vcs {
        match &vc.result {
            Some(VcResult::Invalid { counterexample }) => {
                sink.emit(Diagnostic::error(
                    ErrorCode::VcInvalid,
                    format!(
                        "Verification condition {} is invalid in operation '{}': {}",
                        vc.vc_id, vc.operation, counterexample
                    ),
                    vc.span,
                ));
            }
            Some(VcResult::Unknown { reason }) => {
                rewrites.push(SyntheticAssertRewrite {
                    operation: vc.operation.clone(),
                    vc_id: vc.vc_id.clone(),
                    solver_reason: reason.clone(),
                });
            }
            _ => {}
        }
    }

    for rewrite in &rewrites {
        inject_synthetic_assert(hir, rewrite);
    }

    rewrites
}

fn inject_synthetic_assert(hir: &mut HirProgram, rewrite: &SyntheticAssertRewrite) {
    for decl in &mut hir.declarations {
        if let HirDeclaration::Operation { name, body, .. } = decl {
            if name == &rewrite.operation {
                body.push(HirStatement::Assert {
                    condition: HirExpr {
                        kind: HirExprKind::Other,
                        meta: HirMeta::synthetic(Span::dummy()),
                    },
                    meta: HirMeta::synthetic(Span::dummy()),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> al_ast::Program {
        al_parser::parse(source).expect("source should parse")
    }

    #[test]
    fn generates_vcs_from_require_ensure_assert_with_unique_ids() {
        let source = r#"
OPERATION Verify =>
  INPUT x: Int64
  REQUIRE x GT 0
  ENSURE x GT 0
  BODY {
    ASSERT x GT 0
    EMIT x
  }
"#;
        let program = parse(source);
        let mut generator = VcGenerator::new();
        let vcs = generator.generate_program(&program);
        assert_eq!(vcs.len(), 3);
        assert_eq!(vcs[0].origin, VcOrigin::Require);
        assert_eq!(vcs[1].origin, VcOrigin::Ensure);
        assert_eq!(vcs[2].origin, VcOrigin::Assert { synthetic: false });
        assert_ne!(vcs[0].vc_id, vcs[1].vc_id);
        assert_ne!(vcs[1].vc_id, vcs[2].vc_id);
    }

    #[test]
    fn generates_two_vcs_per_invariant_for_entry_and_iteration_boundary() {
        let source = r#"
OPERATION LoopGuarded =>
  INPUT x: Int64
  INVARIANT x GTE 0
  BODY {
    EMIT x
  }
"#;
        let program = parse(source);
        let mut generator = VcGenerator::new();
        let vcs = generator.generate_program(&program);

        assert_eq!(vcs.len(), 2);
        assert_eq!(vcs[0].origin, VcOrigin::InvariantLoopEntry);
        assert_eq!(vcs[1].origin, VcOrigin::InvariantIterationBoundary);
        assert_ne!(vcs[0].vc_id, vcs[1].vc_id);
    }

    #[test]
    fn stub_solver_implements_trait() {
        let solver = StubSolver::default();
        assert_eq!(solver.name(), "stub");
        let config = SolverConfig::default();

        let mut vc = VerificationCondition::new(
            "vc_000001".to_string(),
            "Test".to_string(),
            VcOrigin::Require,
            "x > 0".to_string(),
            Span::dummy(),
        );

        let result = solver.solve(&mut vc, &config);
        assert!(matches!(result, VcResult::Unknown { .. }));
    }

    #[test]
    fn stub_solver_can_return_valid_invalid_unknown() {
        let mut vc = VerificationCondition::new(
            "vc_000001".to_string(),
            "Verify".to_string(),
            VcOrigin::Require,
            "x > 0".to_string(),
            Span::dummy(),
        );

        let valid_solver = StubSolver::new(StubSolverConfig {
            default_mode: StubSolverMode::AlwaysValid,
            per_vc: HashMap::new(),
        });
        assert_eq!(valid_solver.solve_legacy(&mut vc), &VcResult::Valid);

        let invalid_solver = StubSolver::new(StubSolverConfig {
            default_mode: StubSolverMode::AlwaysInvalid {
                counterexample: "x = -1".to_string(),
            },
            per_vc: HashMap::new(),
        });
        assert!(matches!(
            invalid_solver.solve_legacy(&mut vc),
            VcResult::Invalid { .. }
        ));

        let unknown_solver = StubSolver::default();
        assert!(matches!(
            unknown_solver.solve_legacy(&mut vc),
            VcResult::Unknown { .. }
        ));
    }

    #[test]
    fn unknown_results_inject_synthetic_assert() {
        let source = r#"
OPERATION Verify =>
  INPUT x: Int64
  REQUIRE x GT 0
  BODY { EMIT x }
"#;
        let program = parse(source);
        let mut generator = VcGenerator::new();
        let mut vcs = generator.generate_program(&program);
        let solver = StubSolver::default();
        for vc in &mut vcs {
            let _ = solver.solve_legacy(vc);
        }

        let mut hir = al_hir::lower_program(&program);
        let mut sink = DiagnosticSink::new();
        let rewrites = apply_vc_results(&vcs, &mut hir, &mut sink);
        assert_eq!(rewrites.len(), 1);
        assert_eq!(rewrites[0].operation, "Verify");
        assert!(!sink.has_errors());

        let op = hir
            .declarations
            .iter()
            .find(|d| matches!(d, HirDeclaration::Operation { name, .. } if name == "Verify"))
            .expect("operation present");
        if let HirDeclaration::Operation { body, .. } = op {
            let last = body.last().expect("synthetic assert added");
            match last {
                HirStatement::Assert { meta, .. } => assert!(meta.synthetic),
                _ => panic!("expected synthetic assert"),
            }
        }
    }

    #[test]
    fn invalid_results_emit_vc_invalid_diagnostic() {
        let source = r#"
OPERATION Verify =>
  INPUT x: Int64
  REQUIRE x GT 0
  BODY { EMIT x }
"#;
        let program = parse(source);
        let mut generator = VcGenerator::new();
        let mut vcs = generator.generate_program(&program);
        let solver = StubSolver::new(StubSolverConfig {
            default_mode: StubSolverMode::AlwaysInvalid {
                counterexample: "x = -1".to_string(),
            },
            per_vc: HashMap::new(),
        });
        for vc in &mut vcs {
            let _ = solver.solve_legacy(vc);
        }

        let mut hir = al_hir::lower_program(&program);
        let mut sink = DiagnosticSink::new();
        let rewrites = apply_vc_results(&vcs, &mut hir, &mut sink);
        assert!(rewrites.is_empty());
        assert!(sink.has_errors());
        assert!(sink
            .errors()
            .iter()
            .any(|d| { d.code == al_diagnostics::DiagnosticCode::Error(ErrorCode::VcInvalid) }));
    }

    #[test]
    fn vc_generator_captures_expr_and_premises() {
        let source = r#"
OPERATION Verify =>
  INPUT x: Int64
  REQUIRE x GT 0
  ENSURE x GT 0
  BODY {
    ASSERT x GT 0
    EMIT x
  }
"#;
        let program = parse(source);
        let mut generator = VcGenerator::new();
        let vcs = generator.generate_program(&program);

        // REQUIRE VC has no premises (it is the premise)
        assert!(vcs[0].expr.is_some());
        assert!(vcs[0].premises.is_empty());

        // ENSURE VC has the REQUIRE as a premise
        assert!(vcs[1].expr.is_some());
        assert_eq!(vcs[1].premises.len(), 1);

        // ASSERT VC also has the REQUIRE as a premise
        assert!(vcs[2].expr.is_some());
        assert_eq!(vcs[2].premises.len(), 1);
    }
}
