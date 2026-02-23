//! AgentLang Verification Condition (VC) generation and solving.
//!
//! MVP v0.1: Stub solver that returns Unknown for all VCs,
//! triggering runtime ASSERT insertion per the SMT Unknown policy.

use al_diagnostics::{ErrorCode, Span};

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

/// A verification condition extracted from the program.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationCondition {
    pub id: String,
    pub description: String,
    pub span: Span,
    pub result: Option<VcResult>,
}

impl VerificationCondition {
    pub fn new(id: impl Into<String>, description: impl Into<String>, span: Span) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            span,
            result: None,
        }
    }
}

/// MVP stub solver: returns Unknown for all VCs.
pub struct MvpSolver;

impl MvpSolver {
    pub fn new() -> Self {
        Self
    }

    /// Attempt to solve a verification condition.
    /// In MVP, always returns Unknown (triggering runtime ASSERT insertion).
    pub fn solve<'a>(&self, vc: &'a mut VerificationCondition) -> &'a VcResult {
        let result = VcResult::Unknown {
            reason: "MVP stub solver: no SMT backend configured".to_string(),
        };
        vc.result = Some(result);
        vc.result.as_ref().unwrap()
    }
}

impl Default for MvpSolver {
    fn default() -> Self {
        Self::new()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvp_solver_returns_unknown() {
        let solver = MvpSolver::new();
        let mut vc = VerificationCondition::new("vc_1", "test condition", Span::dummy());
        let result = solver.solve(&mut vc);
        assert!(matches!(result, VcResult::Unknown { .. }));
    }

    #[test]
    fn unknown_needs_runtime_assert() {
        let result = VcResult::Unknown { reason: "test".into() };
        assert!(needs_runtime_assert(&result));
        assert!(!is_compile_error(&result));
    }

    #[test]
    fn invalid_is_compile_error() {
        let result = VcResult::Invalid { counterexample: "x=0".into() };
        assert!(!needs_runtime_assert(&result));
        assert!(is_compile_error(&result));
        assert_eq!(error_code_for_result(&result), Some(ErrorCode::VcInvalid));
    }

    #[test]
    fn valid_needs_nothing() {
        let result = VcResult::Valid;
        assert!(!needs_runtime_assert(&result));
        assert!(!is_compile_error(&result));
        assert_eq!(error_code_for_result(&result), None);
    }
}
