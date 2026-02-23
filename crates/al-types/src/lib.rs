//! AgentLang Type System - MVP v0.1
//!
//! Basic type checking: declaration/type table, expression typing,
//! failure arity enforcement, capability requirement tracking.

use al_ast::{self, Declaration, Expr, MatchBody, Pattern, Statement, TypeExpr};
use al_diagnostics::{Diagnostic, DiagnosticSink, ErrorCode, Span, WarningCode};
use std::collections::{HashMap, HashSet};

/// The profile tag for all MVP diagnostics.
pub const MVP_PROFILE: &str = "mvp-0.1";

/// Built-in type names that are always available without explicit definition.
const BUILTIN_TYPES: &[&str] = &[
    "Int64", "Float64", "Str", "Bool",
    "List", "Map", "Set",
    "Result", "Option",
    "Duration", "Size", "Confidence", "Hash",
    "Record", "Any", "Unit", "Void",
    "Int", "Float", "String", "Bytes",
];

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
        // Pass 4: Resolve type references (P1)
        self.resolve_type_references(program);
        // Pass 5: Validate REQUIRE clause expressions (P2)
        self.check_require_clauses(program);
        // Pass 6: Resolve pipeline/fork references (P2)
        self.resolve_pipeline_fork_references(program);
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

    // ── Pass 4: Type reference resolution ─────────────────────────────

    /// Check if a type name is known (built-in, user-defined type, or schema).
    fn is_type_defined(&self, name: &str) -> bool {
        BUILTIN_TYPES.contains(&name)
            || self.env.types.contains_key(name)
            || self.env.schemas.contains_key(name)
    }

    /// Pass 4: Resolve type references — verify all referenced type names
    /// are either built-in or declared in the program.
    fn resolve_type_references(&mut self, program: &al_ast::Program) {
        for decl in &program.declarations {
            match &decl.node {
                Declaration::TypeDecl {
                    ty, type_params, ..
                } => {
                    let params: HashSet<String> =
                        type_params.iter().map(|p| p.node.clone()).collect();
                    self.check_type_expr(ty, &params);
                }
                Declaration::SchemaDecl { fields, .. } => {
                    let empty = HashSet::new();
                    for field in fields {
                        self.check_type_expr(&field.node.ty, &empty);
                    }
                }
                Declaration::OperationDecl {
                    inputs, output, ..
                } => {
                    let empty = HashSet::new();
                    for input in inputs {
                        self.check_type_expr(&input.node.ty, &empty);
                    }
                    if let Some(out) = output {
                        self.check_type_expr(out, &empty);
                    }
                }
                _ => {}
            }
        }
    }

    /// Recursively check a type expression for undefined type references.
    fn check_type_expr(
        &mut self,
        ty: &al_ast::Spanned<TypeExpr>,
        type_params: &HashSet<String>,
    ) {
        match &ty.node {
            TypeExpr::Named { name, params } => {
                if !type_params.contains(&name.node) && !self.is_type_defined(&name.node) {
                    self.sink.emit(Diagnostic::error(
                        ErrorCode::UnknownIdentifier,
                        format!("Undefined type: '{}'", name.node),
                        name.span,
                    ));
                }
                for param in params {
                    self.check_type_expr(param, type_params);
                }
            }
            TypeExpr::Union { types } => {
                for t in types {
                    self.check_type_expr(t, type_params);
                }
            }
            TypeExpr::Constrained { ty: inner, .. } => {
                self.check_type_expr(inner, type_params);
            }
            TypeExpr::Record { fields } => {
                for field in fields {
                    self.check_type_expr(&field.node.ty, type_params);
                }
            }
        }
    }

    // ── Pass 5: REQUIRE clause validation ────────────────────────────

    /// Pass 5: Validate REQUIRE clause expressions — check that identifiers
    /// in REQUIRE conditions reference operation inputs or known names.
    fn check_require_clauses(&mut self, program: &al_ast::Program) {
        for decl in &program.declarations {
            if let Declaration::OperationDecl {
                requires, inputs, ..
            } = &decl.node
            {
                let input_names: HashSet<String> =
                    inputs.iter().map(|i| i.node.name.node.clone()).collect();
                for req in requires {
                    self.check_require_expr(req, &input_names);
                }
            }
        }
    }

    /// Walk a REQUIRE expression and verify identifiers are in scope.
    fn check_require_expr(
        &mut self,
        expr: &al_ast::Spanned<Expr>,
        known_names: &HashSet<String>,
    ) {
        match &expr.node {
            Expr::Identifier(name) => {
                // Only flag top-level identifiers that are not inputs.
                // We don't flag calls or member bases — they may reference
                // stdlib or schemas which we don't fully resolve yet.
                if !known_names.contains(name) {
                    self.sink.emit(Diagnostic::error(
                        ErrorCode::UnknownIdentifier,
                        format!(
                            "Unknown identifier '{}' in REQUIRE clause (not an operation input)",
                            name
                        ),
                        expr.span,
                    ));
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_require_expr(left, known_names);
                self.check_require_expr(right, known_names);
            }
            Expr::UnaryOp { operand, .. } => {
                self.check_require_expr(operand, known_names);
            }
            Expr::Member { object, .. } => {
                self.check_require_expr(object, known_names);
            }
            Expr::Call { func, args, .. } => {
                // Don't check function name — it may be a stdlib function
                if let Expr::Identifier(_) = &func.node {
                    // OK — function calls are allowed
                } else {
                    self.check_require_expr(func, known_names);
                }
                for arg in args {
                    self.check_require_expr(&arg.node.value, known_names);
                }
            }
            Expr::Literal(_) => {} // always valid
            Expr::Paren { inner } => {
                self.check_require_expr(inner, known_names);
            }
            _ => {} // other expression types OK for now
        }
    }

    // ── Pass 6: Pipeline/fork reference resolution ───────────────────

    /// Pass 6: Resolve pipeline stage and fork branch references.
    /// Emits warnings (not errors) for unresolved references since stages
    /// may reference stdlib functions not in the compilation unit.
    fn resolve_pipeline_fork_references(&mut self, program: &al_ast::Program) {
        for decl in &program.declarations {
            match &decl.node {
                Declaration::PipelineDecl { chain, .. } => {
                    for stage in &chain.node.stages {
                        self.check_pipeline_stage_ref(&stage.expr);
                    }
                }
                Declaration::OperationDecl { body, .. } => {
                    for stmt in &body.node.stmts {
                        self.check_fork_refs_in_stmt(&stmt.node);
                    }
                }
                _ => {}
            }
        }
    }

    /// Warn if a pipeline stage identifier doesn't reference a defined operation.
    fn check_pipeline_stage_ref(&mut self, expr: &al_ast::Spanned<Expr>) {
        if let Expr::Identifier(name) = &expr.node {
            if !self.env.operations.contains_key(name) {
                self.sink.emit(Diagnostic::warning(
                    WarningCode::UnresolvedReference,
                    format!(
                        "Pipeline stage '{}' does not reference a defined operation",
                        name
                    ),
                    expr.span,
                ));
            }
        }
    }

    /// Walk statements looking for FORK expressions and check branch refs.
    fn check_fork_refs_in_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Store { value, .. } => {
                self.check_fork_refs_in_expr(&value.node, value.span);
            }
            Statement::Match {
                arms, otherwise, ..
            } => {
                for arm in arms {
                    match &arm.node.body.node {
                        MatchBody::Block(block) => {
                            for s in &block.node.stmts {
                                self.check_fork_refs_in_stmt(&s.node);
                            }
                        }
                        MatchBody::Expr(expr) => {
                            self.check_fork_refs_in_expr(&expr.node, expr.span);
                        }
                    }
                }
                if let Some(ow) = otherwise {
                    match &ow.node {
                        MatchBody::Block(block) => {
                            for s in &block.node.stmts {
                                self.check_fork_refs_in_stmt(&s.node);
                            }
                        }
                        MatchBody::Expr(expr) => {
                            self.check_fork_refs_in_expr(&expr.node, expr.span);
                        }
                    }
                }
            }
            Statement::Loop { body, .. } => {
                for s in &body.node.stmts {
                    self.check_fork_refs_in_stmt(&s.node);
                }
            }
            _ => {}
        }
    }

    /// Check a FORK expression for unresolved branch references.
    fn check_fork_refs_in_expr(&mut self, expr: &Expr, _span: Span) {
        if let Expr::Fork { branches, .. } = expr {
            for branch in branches {
                for stage in &branch.node.chain.node.stages {
                    self.check_pipeline_stage_ref(&stage.expr);
                }
            }
        }
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

    // ── Type reference resolution ──────────────────────────────────

    #[test]
    fn undefined_type_reference_detected() {
        let source = "TYPE Foo = UndefinedType";
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(checker.has_errors(), "should detect undefined type");
        let errors = checker.sink.errors();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Undefined type: 'UndefinedType'")),
            "should report UndefinedType"
        );
    }

    #[test]
    fn builtin_type_reference_accepted() {
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
        assert!(!checker.has_errors(), "built-in types should resolve");
    }

    #[test]
    fn schema_used_as_type_resolves() {
        let source = r#"
SCHEMA User => { name: Str, age: Int64 }
OPERATION GetUser =>
  INPUT id: Int64
  OUTPUT User
  BODY { EMIT id }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(
            !checker.has_errors(),
            "schema names should be valid type references"
        );
    }

    #[test]
    fn generic_type_params_in_scope() {
        let source = "TYPE Wrapper[T] = List[T]";
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(
            !checker.has_errors(),
            "type parameters should be in scope within their declaration"
        );
    }

    #[test]
    fn undefined_generic_param_detected() {
        let source = "TYPE Foo = List[Nonexistent]";
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(checker.has_errors(), "undefined generic arg should error");
    }

    // ── REQUIRE clause validation ──────────────────────────────────

    #[test]
    fn require_references_input() {
        let source = r#"
OPERATION Validate =>
  INPUT data: Record
  REQUIRE data GT 0
  BODY { EMIT data }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        // `data` is an input — no REQUIRE error; GT/0 are operators/literals.
        let require_errors: Vec<_> = checker
            .sink
            .errors()
            .into_iter()
            .filter(|e| e.message.contains("REQUIRE"))
            .collect();
        assert!(
            require_errors.is_empty(),
            "REQUIRE referencing input should be valid"
        );
    }

    #[test]
    fn require_unknown_identifier_detected() {
        let source = r#"
OPERATION Validate =>
  INPUT data: Record
  REQUIRE unknown_var GT 0
  BODY { EMIT data }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(
            checker.has_errors(),
            "REQUIRE with unknown identifier should error"
        );
    }

    // ── Pipeline/fork reference resolution ─────────────────────────

    #[test]
    fn pipeline_unresolved_stages_warned() {
        let source = "PIPELINE P => fetch -> validate";
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        // Should produce warnings (not errors) for unresolved stages
        assert!(
            !checker.has_errors(),
            "unresolved pipeline stages should warn, not error"
        );
        assert!(
            checker.sink.has_warnings(),
            "should warn about unresolved pipeline stages"
        );
    }

    #[test]
    fn pipeline_resolved_stages_no_warning() {
        let source = r#"
OPERATION Fetch => BODY { EMIT 1 }
OPERATION Validate => BODY { EMIT 2 }
PIPELINE P => Fetch -> Validate
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(!checker.has_errors());
        assert!(
            !checker.sink.has_warnings(),
            "resolved pipeline stages should not warn"
        );
    }
}
