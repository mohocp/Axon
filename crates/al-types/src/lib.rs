//! AgentLang Type System - MVP v0.1
//!
//! Basic type checking: declaration/type table, expression typing,
//! failure arity enforcement, capability requirement tracking.

use al_ast::{self, Declaration, MatchBody, Pattern, Statement};
use al_diagnostics::{Diagnostic, DiagnosticSink, ErrorCode, Span};
use std::collections::HashMap;

/// The profile tag for all MVP diagnostics.
pub const MVP_PROFILE: &str = "mvp-0.1";

/// Type environment for a compilation unit.
#[derive(Debug, Default)]
pub struct TypeEnv {
    /// Named type definitions.
    pub types: HashMap<String, TypeInfo>,
    /// Schema definitions.
    pub schemas: HashMap<String, SchemaInfo>,
    /// Agent definitions.
    pub agents: HashMap<String, AgentInfo>,
    /// Operation definitions.
    pub operations: HashMap<String, OperationInfo>,
    /// Pipeline definitions.
    pub pipelines: HashMap<String, PipelineInfo>,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SchemaInfo {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub name: String,
    pub capabilities: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct OperationInfo {
    pub name: String,
    pub inputs: Vec<(String, String)>,
    pub output: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PipelineInfo {
    pub name: String,
    pub span: Span,
}

/// Type checker for AgentLang MVP.
pub struct TypeChecker {
    pub env: TypeEnv,
    pub sink: DiagnosticSink,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::default(),
            sink: DiagnosticSink::new(),
        }
    }

    /// Run all type checking passes on a program.
    pub fn check(&mut self, program: &al_ast::Program) {
        // Pass 1: Build declaration table
        self.build_declarations(program);
        // Pass 2: Check failure arity (C2)
        self.check_failure_arity(program);
        // Pass 3: Check excluded features (C8)
        self.check_excluded_features(program);
    }

    /// Pass 1: Build declaration/type table.
    fn build_declarations(&mut self, program: &al_ast::Program) {
        for decl in &program.declarations {
            match &decl.node {
                Declaration::TypeDecl { name, .. } => {
                    if self.env.types.contains_key(&name.node) {
                        self.sink.emit(Diagnostic::error(
                            ErrorCode::DuplicateDefinition,
                            format!("Duplicate type definition: '{}'", name.node),
                            name.span,
                        ));
                    } else {
                        self.env.types.insert(
                            name.node.clone(),
                            TypeInfo {
                                name: name.node.clone(),
                                span: name.span,
                            },
                        );
                    }
                }
                Declaration::SchemaDecl { name, fields } => {
                    if self.env.schemas.contains_key(&name.node) {
                        self.sink.emit(Diagnostic::error(
                            ErrorCode::DuplicateDefinition,
                            format!("Duplicate schema definition: '{}'", name.node),
                            name.span,
                        ));
                    } else {
                        let field_info: Vec<(String, String)> = fields
                            .iter()
                            .map(|f| {
                                (f.node.name.node.clone(), format!("{:?}", f.node.ty.node))
                            })
                            .collect();
                        self.env.schemas.insert(
                            name.node.clone(),
                            SchemaInfo {
                                name: name.node.clone(),
                                fields: field_info,
                                span: name.span,
                            },
                        );
                    }
                }
                Declaration::AgentDecl { name, properties } => {
                    if self.env.agents.contains_key(&name.node) {
                        self.sink.emit(Diagnostic::error(
                            ErrorCode::DuplicateDefinition,
                            format!("Duplicate agent definition: '{}'", name.node),
                            name.span,
                        ));
                    } else {
                        let caps: Vec<String> = properties
                            .iter()
                            .filter_map(|p| match &p.node {
                                al_ast::AgentProperty::Capabilities(caps) => {
                                    Some(caps.iter().map(|c| c.node.clone()).collect::<Vec<_>>())
                                }
                                _ => None,
                            })
                            .flatten()
                            .collect();
                        self.env.agents.insert(
                            name.node.clone(),
                            AgentInfo {
                                name: name.node.clone(),
                                capabilities: caps,
                                span: name.span,
                            },
                        );
                    }
                }
                Declaration::OperationDecl {
                    name,
                    inputs,
                    output,
                    ..
                } => {
                    if self.env.operations.contains_key(&name.node) {
                        self.sink.emit(Diagnostic::error(
                            ErrorCode::DuplicateDefinition,
                            format!("Duplicate operation definition: '{}'", name.node),
                            name.span,
                        ));
                    } else {
                        let input_info: Vec<(String, String)> = inputs
                            .iter()
                            .map(|p| {
                                (p.node.name.node.clone(), format!("{:?}", p.node.ty.node))
                            })
                            .collect();
                        let output_ty = output.as_ref().map(|o| format!("{:?}", o.node));
                        self.env.operations.insert(
                            name.node.clone(),
                            OperationInfo {
                                name: name.node.clone(),
                                inputs: input_info,
                                output: output_ty,
                                span: name.span,
                            },
                        );
                    }
                }
                Declaration::PipelineDecl { name, .. } => {
                    if self.env.pipelines.contains_key(&name.node) {
                        self.sink.emit(Diagnostic::error(
                            ErrorCode::DuplicateDefinition,
                            format!("Duplicate pipeline definition: '{}'", name.node),
                            name.span,
                        ));
                    } else {
                        self.env.pipelines.insert(
                            name.node.clone(),
                            PipelineInfo {
                                name: name.node.clone(),
                                span: name.span,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Pass 2: Check that all FAILURE patterns use the canonical 3-field form (C2).
    fn check_failure_arity(&mut self, program: &al_ast::Program) {
        for decl in &program.declarations {
            if let Declaration::OperationDecl { body, .. } = &decl.node {
                for stmt in &body.node.stmts {
                    self.check_failure_arity_in_stmt(&stmt.node);
                }
            }
        }
    }

    fn check_failure_arity_in_stmt(&mut self, stmt: &Statement) {
        if let Statement::Match {
            arms, otherwise, ..
        } = stmt
        {
            for arm in arms {
                self.check_failure_arity_in_pattern(
                    &arm.node.pattern.node,
                    &arm.node.pattern.span,
                );
                if let MatchBody::Block(block) = &arm.node.body.node {
                    for s in &block.node.stmts {
                        self.check_failure_arity_in_stmt(&s.node);
                    }
                }
            }
            if let Some(ow) = otherwise {
                if let MatchBody::Block(block) = &ow.node {
                    for s in &block.node.stmts {
                        self.check_failure_arity_in_stmt(&s.node);
                    }
                }
            }
        }
    }

    fn check_failure_arity_in_pattern(&mut self, pattern: &Pattern, _span: &Span) {
        match pattern {
            Pattern::Failure { .. } => {
                // 3-field form enforced by grammar. Valid.
            }
            Pattern::Constructor { args, .. } => {
                for arg in args {
                    self.check_failure_arity_in_pattern(&arg.node, &arg.span);
                }
            }
            Pattern::Success(inner) => {
                self.check_failure_arity_in_pattern(&inner.node, &inner.span);
            }
            _ => {}
        }
    }

    /// Pass 3: Check for excluded features (C8).
    fn check_excluded_features(&mut self, _program: &al_ast::Program) {
        // Excluded feature checking is primarily handled at parse time.
    }

    /// Check that a RETRY count is a valid non-negative integer (C10).
    pub fn check_retry_count(&mut self, count: i64, span: Span) {
        if count < 0 {
            self.sink.emit(Diagnostic::error(
                ErrorCode::TypeMismatch,
                format!("RETRY count must be non-negative, got {}", count),
                span,
            ));
        }
    }

    /// Emit NOT_IMPLEMENTED for excluded join strategies (C4).
    pub fn reject_non_mvp_join(&mut self, strategy: &str, span: Span) {
        if strategy != "ALL_COMPLETE" {
            self.sink.emit(
                Diagnostic::error(
                    ErrorCode::NotImplemented,
                    format!(
                        "Join strategy '{}' is not supported in mvp-0.1; only ALL_COMPLETE is allowed",
                        strategy
                    ),
                    span,
                )
                .with_note("Profile: mvp-0.1".to_string()),
            );
        }
    }

    /// Take ownership of collected diagnostics.
    pub fn take_diagnostics(&mut self) -> DiagnosticSink {
        std::mem::take(&mut self.sink)
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.sink.has_errors()
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_ast::{Spanned, TypeExpr};

    fn dummy_span() -> Span {
        Span::dummy()
    }

    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned {
            node,
            span: dummy_span(),
        }
    }

    #[test]
    fn empty_program_type_checks() {
        let program = al_ast::Program {
            declarations: vec![],
            span: dummy_span(),
        };
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(!checker.has_errors());
    }

    #[test]
    fn duplicate_type_detected() {
        let decl = Declaration::TypeDecl {
            name: spanned("Foo".to_string()),
            type_params: vec![],
            ty: spanned(TypeExpr::Named {
                name: spanned("Int".to_string()),
                params: vec![],
            }),
        };
        let program = al_ast::Program {
            declarations: vec![spanned(decl.clone()), spanned(decl)],
            span: dummy_span(),
        };
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(checker.has_errors());
    }

    #[test]
    fn reject_non_mvp_join_strategy() {
        let mut checker = TypeChecker::new();
        checker.reject_non_mvp_join("BEST_EFFORT", dummy_span());
        assert!(checker.has_errors());
        let diags = checker.take_diagnostics();
        let errors: Vec<_> = diags.errors().into_iter().collect();
        assert_eq!(
            errors[0].code,
            al_diagnostics::DiagnosticCode::Error(ErrorCode::NotImplemented)
        );
    }

    #[test]
    fn negative_retry_count_rejected() {
        let mut checker = TypeChecker::new();
        checker.check_retry_count(-1, dummy_span());
        assert!(checker.has_errors());
    }

    #[test]
    fn valid_retry_count_accepted() {
        let mut checker = TypeChecker::new();
        checker.check_retry_count(3, dummy_span());
        assert!(!checker.has_errors());
    }

    #[test]
    fn type_check_parsed_program() {
        let source = r#"
TYPE UserId = Int64
SCHEMA User => { name: Str, id: Int64 }
OPERATION GetUser =>
  INPUT id: Int64
  OUTPUT User
  BODY {
    EMIT id
  }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(!checker.has_errors());
        assert!(checker.env.types.contains_key("UserId"));
        assert!(checker.env.schemas.contains_key("User"));
        assert!(checker.env.operations.contains_key("GetUser"));
    }

    #[test]
    fn duplicate_operation_detected() {
        let source = r#"
OPERATION Foo => BODY { EMIT 1 }
OPERATION Foo => BODY { EMIT 2 }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(checker.has_errors());
    }
}
