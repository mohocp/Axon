//! Simple algebraic solver for common verification patterns.
//!
//! Proves verification conditions without an external SMT solver by:
//! 1. Translating REQUIRE premises and the VC goal to `SmtExpr`.
//! 2. Checking if the goal is a direct consequence of premises (structural match).
//! 3. Evaluating constant expressions directly.
//! 4. Applying simple arithmetic reasoning (monotonicity, transitivity).
//! 5. Returning `Unknown` for anything it can't handle (safe fallback).

use crate::smt::{SmtExpr, SmtTranslator};
use crate::{Solver, SolverConfig, VcOrigin, VcResult, VerificationCondition};

/// A lightweight solver that proves common patterns without an external SMT backend.
///
/// This solver handles:
/// - Constant expression evaluation (`3 GT 2` → Valid)
/// - Premise matching (`REQUIRE x GT 0` + `ENSURE x GT 0` → Valid)
/// - REQUIRE VCs are axioms (always Valid)
/// - Simple monotonicity (`x GT 0` implies `x + 1 GT 0`)
/// - Boolean tautologies (`TRUE`, `NOT FALSE`)
/// - Conjunction splitting and implication
///
/// For anything it can't prove, it returns `Unknown` (fail-safe).
pub struct SimpleSolver;

impl SimpleSolver {
    pub fn new() -> Self {
        SimpleSolver
    }
}

impl Default for SimpleSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Solver for SimpleSolver {
    fn solve(&self, vc: &mut VerificationCondition, _config: &SolverConfig) -> VcResult {
        // REQUIRE VCs are axioms — they represent the caller's obligation.
        // They are always Valid from the operation's perspective.
        if vc.origin == VcOrigin::Require {
            return VcResult::Valid;
        }

        // If we don't have an expression to analyze, fall back to Unknown.
        let goal_expr = match &vc.expr {
            Some(expr) => expr.clone(),
            None => {
                return VcResult::Unknown {
                    reason: "no expression available for analysis".to_string(),
                };
            }
        };

        let goal = SmtTranslator::translate(&goal_expr);

        // If the goal itself is opaque, we can't reason about it.
        if goal.is_unknown() {
            return VcResult::Unknown {
                reason: "expression contains constructs not supported by simple solver".to_string(),
            };
        }

        // Translate premises.
        let premises: Vec<SmtExpr> = vc
            .premises
            .iter()
            .map(SmtTranslator::translate)
            .filter(|p| !p.is_unknown())
            .collect();

        // Try proving strategies in order of simplicity.
        try_constant_eval(&goal)
            .or_else(|| try_tautology(&goal))
            .or_else(|| try_premise_match(&premises, &goal))
            .or_else(|| try_monotonicity(&premises, &goal))
            .or_else(|| try_transitivity(&premises, &goal))
            .or_else(|| try_strengthen(&premises, &goal))
            .unwrap_or_else(|| VcResult::Unknown {
                reason: "simple solver: could not prove or disprove".to_string(),
            })
    }

    fn name(&self) -> &str {
        "simple"
    }
}

// ---------------------------------------------------------------------------
// Proving strategies
// ---------------------------------------------------------------------------

/// Evaluate a boolean expression with only constants.
fn try_constant_eval(goal: &SmtExpr) -> Option<VcResult> {
    match eval_bool(goal) {
        Some(true) => Some(VcResult::Valid),
        Some(false) => Some(VcResult::Invalid {
            counterexample: "constant expression evaluates to false".to_string(),
        }),
        None => None,
    }
}

/// Check for boolean tautologies.
fn try_tautology(goal: &SmtExpr) -> Option<VcResult> {
    match goal {
        SmtExpr::BoolLit(true) => Some(VcResult::Valid),
        SmtExpr::BoolLit(false) => Some(VcResult::Invalid {
            counterexample: "goal is FALSE".to_string(),
        }),
        SmtExpr::Not(inner) => match inner.as_ref() {
            SmtExpr::BoolLit(false) => Some(VcResult::Valid),
            SmtExpr::BoolLit(true) => Some(VcResult::Invalid {
                counterexample: "NOT TRUE is FALSE".to_string(),
            }),
            _ => None,
        },
        _ => None,
    }
}

/// Check if the goal is structurally identical to any premise.
fn try_premise_match(premises: &[SmtExpr], goal: &SmtExpr) -> Option<VcResult> {
    for premise in premises {
        if premise.structurally_eq(goal) {
            return Some(VcResult::Valid);
        }
    }

    // Check if goal is a conjunction and both parts are in premises.
    if let SmtExpr::And(left, right) = goal {
        let left_proved = premises.iter().any(|p| p.structurally_eq(left));
        let right_proved = premises.iter().any(|p| p.structurally_eq(right));
        if left_proved && right_proved {
            return Some(VcResult::Valid);
        }
    }

    // Check if any premise is a conjunction containing the goal.
    for premise in premises {
        if let SmtExpr::And(left, right) = premise {
            if left.structurally_eq(goal) || right.structurally_eq(goal) {
                return Some(VcResult::Valid);
            }
        }
    }

    None
}

/// Monotonicity reasoning: if `x GT a` is a premise and goal is `x + k GT a`
/// where k > 0, then the goal follows (for GT, GTE).
fn try_monotonicity(premises: &[SmtExpr], goal: &SmtExpr) -> Option<VcResult> {
    match goal {
        // Goal: (x + k) GT c  |  Premise: x GT c  |  k > 0 → Valid
        SmtExpr::Gt(left, right) => {
            if let SmtExpr::Add(var, offset) = left.as_ref() {
                if let Some(k) = extract_int(offset) {
                    if k > 0 {
                        // We need premise: var GT right or var GTE right
                        let needed_gt = SmtExpr::Gt(var.clone(), right.clone());
                        let needed_gte = SmtExpr::Gte(var.clone(), right.clone());
                        if premises.iter().any(|p| {
                            p.structurally_eq(&needed_gt) || p.structurally_eq(&needed_gte)
                        }) {
                            return Some(VcResult::Valid);
                        }
                    }
                }
            }
            // Goal: x GT c  |  Premise: x GT d where d >= c → Valid
            if let (Some(var_name), Some(c)) = (extract_var_name(left), extract_int(right)) {
                for premise in premises {
                    if let SmtExpr::Gt(pvar, pval) = premise {
                        if let (Some(pn), Some(d)) = (extract_var_name(pvar), extract_int(pval)) {
                            if pn == var_name && d >= c {
                                return Some(VcResult::Valid);
                            }
                        }
                    }
                    if let SmtExpr::Gte(pvar, pval) = premise {
                        if let (Some(pn), Some(d)) = (extract_var_name(pvar), extract_int(pval)) {
                            if pn == var_name && d > c {
                                return Some(VcResult::Valid);
                            }
                        }
                    }
                }
            }
            None
        }
        SmtExpr::Gte(left, right) => {
            // Goal: (x + k) GTE c  |  Premise: x GTE c  |  k >= 0 → Valid
            if let SmtExpr::Add(var, offset) = left.as_ref() {
                if let Some(k) = extract_int(offset) {
                    if k >= 0 {
                        let needed_gte = SmtExpr::Gte(var.clone(), right.clone());
                        let needed_gt = SmtExpr::Gt(var.clone(), right.clone());
                        if premises.iter().any(|p| {
                            p.structurally_eq(&needed_gte) || p.structurally_eq(&needed_gt)
                        }) {
                            return Some(VcResult::Valid);
                        }
                    }
                }
            }
            // Goal: x GTE c  |  Premise: x GT c or x GTE d where d >= c → Valid
            if let (Some(var_name), Some(c)) = (extract_var_name(left), extract_int(right)) {
                for premise in premises {
                    match premise {
                        SmtExpr::Gt(pvar, pval) => {
                            if let (Some(pn), Some(d)) = (extract_var_name(pvar), extract_int(pval))
                            {
                                // x > d and d >= c implies x >= c
                                if pn == var_name && d >= c {
                                    return Some(VcResult::Valid);
                                }
                            }
                        }
                        SmtExpr::Gte(pvar, pval) => {
                            if let (Some(pn), Some(d)) = (extract_var_name(pvar), extract_int(pval))
                            {
                                if pn == var_name && d >= c {
                                    return Some(VcResult::Valid);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Transitivity reasoning: if premises include `x GT a` and `a GT b`,
/// then `x GT b` follows.
fn try_transitivity(premises: &[SmtExpr], goal: &SmtExpr) -> Option<VcResult> {
    // Goal: x GT c
    // If any premise says x GT d and d >= c, that's monotonicity (handled above).
    // If premise says x GT y and another says y GT c, that's transitivity.
    match goal {
        SmtExpr::Gt(goal_left, goal_right) => {
            // Find premise: goal_left GT something
            for p1 in premises {
                if let SmtExpr::Gt(p1_left, p1_right) = p1 {
                    if p1_left.structurally_eq(goal_left) {
                        // We have: goal_left GT p1_right
                        // We need: p1_right GTE goal_right
                        let needed = SmtExpr::Gte(p1_right.clone(), goal_right.clone());
                        let needed_gt = SmtExpr::Gt(p1_right.clone(), goal_right.clone());
                        let needed_eq = SmtExpr::Eq(p1_right.clone(), goal_right.clone());
                        if premises.iter().any(|p| {
                            p.structurally_eq(&needed)
                                || p.structurally_eq(&needed_gt)
                                || p.structurally_eq(&needed_eq)
                        }) {
                            return Some(VcResult::Valid);
                        }
                        // Or p1_right == goal_right structurally
                        if p1_right.structurally_eq(goal_right) {
                            return Some(VcResult::Valid);
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Strengthening: GT implies GTE, and specific numeric reasoning.
fn try_strengthen(premises: &[SmtExpr], goal: &SmtExpr) -> Option<VcResult> {
    match goal {
        // If goal is x GTE c and premise is x GT c, that's valid (GT implies GTE).
        SmtExpr::Gte(left, right) => {
            let stronger = SmtExpr::Gt(left.clone(), right.clone());
            if premises.iter().any(|p| p.structurally_eq(&stronger)) {
                return Some(VcResult::Valid);
            }
            None
        }
        // Goal: x NEQ c  |  Premise: x GT c → Valid (if x > c then x ≠ c)
        SmtExpr::Neq(left, right) => {
            let gt = SmtExpr::Gt(left.clone(), right.clone());
            let lt = SmtExpr::Lt(left.clone(), right.clone());
            if premises
                .iter()
                .any(|p| p.structurally_eq(&gt) || p.structurally_eq(&lt))
            {
                return Some(VcResult::Valid);
            }
            None
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Try to evaluate a boolean expression with only constants.
fn eval_bool(expr: &SmtExpr) -> Option<bool> {
    match expr {
        SmtExpr::BoolLit(v) => Some(*v),
        SmtExpr::Not(inner) => eval_bool(inner).map(|v| !v),
        SmtExpr::And(l, r) => {
            let lv = eval_bool(l)?;
            let rv = eval_bool(r)?;
            Some(lv && rv)
        }
        SmtExpr::Or(l, r) => {
            let lv = eval_bool(l)?;
            let rv = eval_bool(r)?;
            Some(lv || rv)
        }
        SmtExpr::Gt(l, r) => {
            let lv = eval_int(l)?;
            let rv = eval_int(r)?;
            Some(lv > rv)
        }
        SmtExpr::Gte(l, r) => {
            let lv = eval_int(l)?;
            let rv = eval_int(r)?;
            Some(lv >= rv)
        }
        SmtExpr::Lt(l, r) => {
            let lv = eval_int(l)?;
            let rv = eval_int(r)?;
            Some(lv < rv)
        }
        SmtExpr::Lte(l, r) => {
            let lv = eval_int(l)?;
            let rv = eval_int(r)?;
            Some(lv <= rv)
        }
        SmtExpr::Eq(l, r) => {
            let li = eval_int(l);
            let ri = eval_int(r);
            if let (Some(lv), Some(rv)) = (li, ri) {
                return Some(lv == rv);
            }
            let lb = eval_bool(l);
            let rb = eval_bool(r);
            if let (Some(lv), Some(rv)) = (lb, rb) {
                return Some(lv == rv);
            }
            None
        }
        SmtExpr::Neq(l, r) => {
            let li = eval_int(l);
            let ri = eval_int(r);
            if let (Some(lv), Some(rv)) = (li, ri) {
                return Some(lv != rv);
            }
            None
        }
        _ => None,
    }
}

/// Try to evaluate an integer expression with only constants.
fn eval_int(expr: &SmtExpr) -> Option<i64> {
    match expr {
        SmtExpr::IntLit(v) => Some(*v),
        SmtExpr::Add(l, r) => Some(eval_int(l)? + eval_int(r)?),
        SmtExpr::Sub(l, r) => Some(eval_int(l)? - eval_int(r)?),
        SmtExpr::Mul(l, r) => Some(eval_int(l)? * eval_int(r)?),
        SmtExpr::Div(l, r) => {
            let rv = eval_int(r)?;
            if rv == 0 {
                return None;
            }
            Some(eval_int(l)? / rv)
        }
        SmtExpr::Mod(l, r) => {
            let rv = eval_int(r)?;
            if rv == 0 {
                return None;
            }
            Some(eval_int(l)? % rv)
        }
        SmtExpr::Neg(inner) => Some(-eval_int(inner)?),
        _ => None,
    }
}

/// Extract integer value from an SmtExpr if it's a literal.
fn extract_int(expr: &SmtExpr) -> Option<i64> {
    match expr {
        SmtExpr::IntLit(v) => Some(*v),
        _ => None,
    }
}

/// Extract variable name from an SmtExpr if it's a simple variable.
fn extract_var_name(expr: &SmtExpr) -> Option<&str> {
    match expr {
        SmtExpr::Var { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SolverConfig, VcOrigin, VerificationCondition};
    use al_ast::{BinaryOp, Expr, Literal, Spanned};
    use al_diagnostics::Span;

    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned {
            node,
            span: Span::dummy(),
        }
    }

    fn make_vc(origin: VcOrigin, expr: Expr, premises: Vec<Expr>) -> VerificationCondition {
        VerificationCondition {
            vc_id: "vc_test".to_string(),
            operation: "Test".to_string(),
            origin,
            description: format!("{:?}", expr),
            span: Span::dummy(),
            result: None,
            expr: Some(expr),
            premises,
        }
    }

    fn binop(left: Expr, op: BinaryOp, right: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(spanned(left)),
            op: spanned(op),
            right: Box::new(spanned(right)),
        }
    }

    fn ident(name: &str) -> Expr {
        Expr::Identifier(name.to_string())
    }

    fn int(v: i64) -> Expr {
        Expr::Literal(Literal::Integer(v))
    }

    #[test]
    fn require_always_valid() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        let expr = binop(ident("x"), BinaryOp::Gt, int(0));
        let mut vc = make_vc(VcOrigin::Require, expr, vec![]);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn constant_true_is_valid() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // 3 GT 2 → true
        let expr = binop(int(3), BinaryOp::Gt, int(2));
        let mut vc = make_vc(VcOrigin::Assert { synthetic: false }, expr, vec![]);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn constant_false_is_invalid() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // 2 GT 3 → false
        let expr = binop(int(2), BinaryOp::Gt, int(3));
        let mut vc = make_vc(VcOrigin::Assert { synthetic: false }, expr, vec![]);
        let result = solver.solve(&mut vc, &config);
        assert!(matches!(result, VcResult::Invalid { .. }));
    }

    #[test]
    fn premise_match_proves_ensure() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // REQUIRE x GT 0, ENSURE x GT 0 → same expression, Valid
        let expr = binop(ident("x"), BinaryOp::Gt, int(0));
        let premises = vec![expr.clone()];
        let mut vc = make_vc(VcOrigin::Ensure, expr, premises);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn premise_match_proves_assert() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // REQUIRE x GT 0, ASSERT x GT 0 → Valid
        let expr = binop(ident("x"), BinaryOp::Gt, int(0));
        let premises = vec![expr.clone()];
        let mut vc = make_vc(VcOrigin::Assert { synthetic: false }, expr, premises);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn monotonicity_addition() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // REQUIRE x GT 0 → ENSURE (x + 1) GT 0 → Valid (monotonicity)
        let premise = binop(ident("x"), BinaryOp::Gt, int(0));
        let goal = binop(
            binop(ident("x"), BinaryOp::Add, int(1)),
            BinaryOp::Gt,
            int(0),
        );
        let mut vc = make_vc(VcOrigin::Ensure, goal, vec![premise]);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn gt_implies_gte() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // REQUIRE x GT 0 → ENSURE x GTE 0 → Valid (GT implies GTE)
        let premise = binop(ident("x"), BinaryOp::Gt, int(0));
        let goal = binop(ident("x"), BinaryOp::Gte, int(0));
        let mut vc = make_vc(VcOrigin::Ensure, goal, vec![premise]);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn gt_implies_neq() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // REQUIRE x GT 0 → ENSURE x NEQ 0 → Valid
        let premise = binop(ident("x"), BinaryOp::Gt, int(0));
        let goal = binop(ident("x"), BinaryOp::Neq, int(0));
        let mut vc = make_vc(VcOrigin::Ensure, goal, vec![premise]);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn unknown_for_unprovable() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // No premises, x GT 0 → Unknown
        let goal = binop(ident("x"), BinaryOp::Gt, int(0));
        let mut vc = make_vc(VcOrigin::Ensure, goal, vec![]);
        let result = solver.solve(&mut vc, &config);
        assert!(matches!(result, VcResult::Unknown { .. }));
    }

    #[test]
    fn constant_arithmetic_in_comparison() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // (10 + 5) GT 14 → 15 > 14 → true → Valid
        let goal = binop(binop(int(10), BinaryOp::Add, int(5)), BinaryOp::Gt, int(14));
        let mut vc = make_vc(VcOrigin::Assert { synthetic: false }, goal, vec![]);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn conjunction_from_separate_premises() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // REQUIRE x GT 0, REQUIRE x LT 100
        // ENSURE (x GT 0) AND (x LT 100) → Valid (both parts in premises)
        let p1 = binop(ident("x"), BinaryOp::Gt, int(0));
        let p2 = binop(ident("x"), BinaryOp::Lt, int(100));
        let goal = binop(
            binop(ident("x"), BinaryOp::Gt, int(0)),
            BinaryOp::And,
            binop(ident("x"), BinaryOp::Lt, int(100)),
        );
        let mut vc = make_vc(VcOrigin::Ensure, goal, vec![p1, p2]);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn stronger_premise_proves_weaker_goal() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        // REQUIRE x GT 10 → ENSURE x GT 5 → Valid (10 >= 5)
        let premise = binop(ident("x"), BinaryOp::Gt, int(10));
        let goal = binop(ident("x"), BinaryOp::Gt, int(5));
        let mut vc = make_vc(VcOrigin::Ensure, goal, vec![premise]);
        let result = solver.solve(&mut vc, &config);
        assert_eq!(result, VcResult::Valid);
    }

    #[test]
    fn solver_name() {
        let solver = SimpleSolver::new();
        assert_eq!(solver.name(), "simple");
    }

    #[test]
    fn no_expr_returns_unknown() {
        let solver = SimpleSolver::new();
        let config = SolverConfig::default();
        let mut vc = VerificationCondition {
            vc_id: "vc_test".to_string(),
            operation: "Test".to_string(),
            origin: VcOrigin::Ensure,
            description: "test".to_string(),
            span: Span::dummy(),
            result: None,
            expr: None,
            premises: vec![],
        };
        let result = solver.solve(&mut vc, &config);
        assert!(matches!(result, VcResult::Unknown { .. }));
    }
}
