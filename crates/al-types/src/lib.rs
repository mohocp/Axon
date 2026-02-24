//! AgentLang Type System - MVP v0.1
//!
//! Basic type checking: declaration/type table, expression typing,
//! failure arity enforcement, capability requirement tracking.

use al_ast::{self, Declaration, Expr, MatchBody, Pattern, Statement, TypeExpr};
use al_capabilities::{resolve_capability, Capability, CapabilityError};
use al_diagnostics::{Diagnostic, DiagnosticSink, ErrorCode, Span, WarningCode};
use al_vc::{StubSolver, StubSolverConfig, VcGenerator, VerificationCondition};
use std::collections::{HashMap, HashSet};

/// The profile tag for all MVP diagnostics.
pub const MVP_PROFILE: &str = "mvp-0.1";

/// Built-in type names that are always available without explicit definition.
const BUILTIN_TYPES: &[&str] = &[
    "Int64",
    "Float64",
    "Str",
    "Bool",
    "List",
    "Map",
    "Set",
    "Result",
    "Option",
    "Duration",
    "Size",
    "Confidence",
    "Hash",
    "Record",
    "Any",
    "Unit",
    "Void",
    "Int",
    "Float",
    "String",
    "Bytes",
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
    pub inputs: Vec<(String, TypeExpr)>,
    pub output: Option<TypeExpr>,
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
    pub vc_results: Vec<VerificationCondition>,
    pub synthetic_asserts: Vec<al_vc::SyntheticAssertRewrite>,
    pub hir_after_vc: Option<al_hir::HirProgram>,
    vc_solver: StubSolver,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::default(),
            sink: DiagnosticSink::new(),
            vc_results: Vec::new(),
            synthetic_asserts: Vec::new(),
            hir_after_vc: None,
            vc_solver: StubSolver::new(StubSolverConfig::default()),
        }
    }

    pub fn with_vc_solver(config: StubSolverConfig) -> Self {
        Self {
            vc_solver: StubSolver::new(config),
            ..Self::new()
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
        // Pass 6: Delegation static validation.
        self.check_delegation_statics(program);
        // Pass 7: Resolve pipeline/fork references (P2)
        self.resolve_pipeline_fork_references(program);
        // Pass 8: Propagate operation output/input types across pipelines.
        self.check_pipeline_type_propagation(program);
        // Pass 9: VC pipeline (REQUIRE/ENSURE/ASSERT generation + solve + rewrite).
        self.run_vc_pipeline(program);
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
                            .map(|f| (f.node.name.node.clone(), format!("{:?}", f.node.ty.node)))
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
                        let input_info: Vec<(String, TypeExpr)> = inputs
                            .iter()
                            .map(|p| (p.node.name.node.clone(), p.node.ty.node.clone()))
                            .collect();
                        let output_ty = output.as_ref().map(|o| o.node.clone());
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
                self.check_failure_arity_in_pattern(&arm.node.pattern.node, &arm.node.pattern.span);
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
                Declaration::OperationDecl { inputs, output, .. } => {
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
    fn check_type_expr(&mut self, ty: &al_ast::Spanned<TypeExpr>, type_params: &HashSet<String>) {
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
                requires,
                inputs,
                body,
                ..
            } = &decl.node
            {
                let mut known_names: HashSet<String> =
                    inputs.iter().map(|i| i.node.name.node.clone()).collect();
                self.collect_store_bindings_from_block(body, &mut known_names);
                for req in requires {
                    self.check_require_expr(req, &known_names);
                }
            }
        }
    }

    fn collect_store_bindings_from_block(
        &self,
        block: &al_ast::Spanned<al_ast::Block>,
        known_names: &mut HashSet<String>,
    ) {
        for stmt in &block.node.stmts {
            self.collect_store_bindings_from_stmt(&stmt.node, known_names);
        }
    }

    fn collect_store_bindings_from_stmt(
        &self,
        stmt: &Statement,
        known_names: &mut HashSet<String>,
    ) {
        match stmt {
            Statement::Store { name, .. } => {
                known_names.insert(name.node.clone());
            }
            Statement::Match {
                arms, otherwise, ..
            } => {
                for arm in arms {
                    if let MatchBody::Block(block) = &arm.node.body.node {
                        self.collect_store_bindings_from_block(block, known_names);
                    }
                }
                if let Some(ow) = otherwise {
                    if let MatchBody::Block(block) = &ow.node {
                        self.collect_store_bindings_from_block(block, known_names);
                    }
                }
            }
            Statement::Loop { body, .. } => {
                self.collect_store_bindings_from_block(body, known_names);
            }
            _ => {}
        }
    }

    /// Walk a REQUIRE expression and verify identifiers are in scope.
    fn check_require_expr(&mut self, expr: &al_ast::Spanned<Expr>, known_names: &HashSet<String>) {
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

    fn check_delegation_statics(&mut self, program: &al_ast::Program) {
        let has_delegate_capability = self
            .env
            .agents
            .values()
            .any(|agent| Self::caps_include_delegate(&agent.capabilities));

        for decl in &program.declarations {
            if let Declaration::OperationDecl { body, .. } = &decl.node {
                for stmt in &body.node.stmts {
                    self.check_delegation_in_stmt(stmt, has_delegate_capability);
                }
            }
        }
    }

    fn check_delegation_in_stmt(
        &mut self,
        stmt: &al_ast::Spanned<Statement>,
        has_delegate_capability: bool,
    ) {
        match &stmt.node {
            Statement::Delegate { target, .. } => {
                if !has_delegate_capability {
                    self.sink.emit(Diagnostic::error(
                        ErrorCode::CapabilityDenied,
                        "DELEGATE statement is not permitted: no AGENT declares DELEGATE capability",
                        stmt.span,
                    ));
                }
                if !self.env.agents.contains_key(&target.node) {
                    self.sink.emit(Diagnostic::error(
                        ErrorCode::UnknownIdentifier,
                        format!("Unknown delegate target agent '{}'", target.node),
                        target.span,
                    ));
                }
            }
            Statement::Match {
                arms, otherwise, ..
            } => {
                for arm in arms {
                    if let MatchBody::Block(block) = &arm.node.body.node {
                        for nested in &block.node.stmts {
                            self.check_delegation_in_stmt(nested, has_delegate_capability);
                        }
                    }
                }
                if let Some(ow) = otherwise {
                    if let MatchBody::Block(block) = &ow.node {
                        for nested in &block.node.stmts {
                            self.check_delegation_in_stmt(nested, has_delegate_capability);
                        }
                    }
                }
            }
            Statement::Loop { body, .. } => {
                for nested in &body.node.stmts {
                    self.check_delegation_in_stmt(nested, has_delegate_capability);
                }
            }
            _ => {}
        }
    }

    fn caps_include_delegate(capability_names: &[String]) -> bool {
        capability_names.iter().any(|name| {
            if name.eq_ignore_ascii_case("DELEGATE") {
                return true;
            }
            match resolve_capability(name) {
                Ok(Capability::Delegate) => true,
                Err(CapabilityError::DeprecatedAlias { canonical, .. }) => {
                    canonical == Capability::Delegate
                }
                _ => false,
            }
        })
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

    // ── Pass 7: Pipeline type propagation ───────────────────────────

    /// Check adjacent operation stages for output->input type compatibility.
    fn check_pipeline_type_propagation(&mut self, program: &al_ast::Program) {
        for decl in &program.declarations {
            match &decl.node {
                Declaration::PipelineDecl { name, chain } => {
                    self.check_pipeline_chain_type_compatibility(
                        &chain.node,
                        &format!("pipeline '{}'", name.node),
                    );
                }
                Declaration::OperationDecl { name, body, .. } => {
                    for stmt in &body.node.stmts {
                        self.check_pipeline_types_in_stmt(&stmt.node, &name.node);
                    }
                }
                _ => {}
            }
        }
    }

    /// Check operation type compatibility for each adjacent pair in a chain.
    fn check_pipeline_chain_type_compatibility(
        &mut self,
        chain: &al_ast::PipelineChain,
        context: &str,
    ) {
        let mut prev_stage: Option<(String, OperationInfo)> = None;

        for stage in &chain.stages {
            let Expr::Identifier(curr_name) = &stage.expr.node else {
                prev_stage = None;
                continue;
            };

            let Some(curr_op) = self.env.operations.get(curr_name).cloned() else {
                prev_stage = None;
                continue;
            };

            if let Some((prev_name, prev_op)) = &prev_stage {
                self.check_stage_type_compatibility(
                    prev_name,
                    prev_op,
                    curr_name,
                    &curr_op,
                    stage.expr.span,
                    context,
                );
            }

            prev_stage = Some((curr_name.clone(), curr_op));
        }
    }

    /// Recursively inspect statements for FORK branch pipeline chains.
    fn check_pipeline_types_in_stmt(&mut self, stmt: &Statement, op_name: &str) {
        match stmt {
            Statement::Store { value, .. } => {
                self.check_pipeline_types_in_expr(&value.node, op_name);
            }
            Statement::Assign { value, .. } => {
                self.check_pipeline_types_in_expr(&value.node, op_name);
            }
            Statement::Expr { expr } => {
                self.check_pipeline_types_in_expr(&expr.node, op_name);
            }
            Statement::Match {
                arms, otherwise, ..
            } => {
                for arm in arms {
                    match &arm.node.body.node {
                        MatchBody::Block(block) => {
                            for stmt in &block.node.stmts {
                                self.check_pipeline_types_in_stmt(&stmt.node, op_name);
                            }
                        }
                        MatchBody::Expr(expr) => {
                            self.check_pipeline_types_in_expr(&expr.node, op_name);
                        }
                    }
                }
                if let Some(otherwise) = otherwise {
                    match &otherwise.node {
                        MatchBody::Block(block) => {
                            for stmt in &block.node.stmts {
                                self.check_pipeline_types_in_stmt(&stmt.node, op_name);
                            }
                        }
                        MatchBody::Expr(expr) => {
                            self.check_pipeline_types_in_expr(&expr.node, op_name);
                        }
                    }
                }
            }
            Statement::Loop { body, .. } => {
                for stmt in &body.node.stmts {
                    self.check_pipeline_types_in_stmt(&stmt.node, op_name);
                }
            }
            _ => {}
        }
    }

    /// Recursively inspect expressions for FORK branches.
    fn check_pipeline_types_in_expr(&mut self, expr: &Expr, op_name: &str) {
        match expr {
            Expr::Fork { branches, .. } => {
                for branch in branches {
                    self.check_pipeline_chain_type_compatibility(
                        &branch.node.chain.node,
                        &format!(
                            "operation '{}' fork branch '{}'",
                            op_name, branch.node.name.node
                        ),
                    );
                }
            }
            Expr::Call { func, args } => {
                self.check_pipeline_types_in_expr(&func.node, op_name);
                for arg in args {
                    self.check_pipeline_types_in_expr(&arg.node.value.node, op_name);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_pipeline_types_in_expr(&left.node, op_name);
                self.check_pipeline_types_in_expr(&right.node, op_name);
            }
            Expr::UnaryOp { operand, .. } => {
                self.check_pipeline_types_in_expr(&operand.node, op_name);
            }
            Expr::Member { object, .. } => {
                self.check_pipeline_types_in_expr(&object.node, op_name);
            }
            Expr::Confidence { expr } => {
                self.check_pipeline_types_in_expr(&expr.node, op_name);
            }
            Expr::Range { start, end } => {
                self.check_pipeline_types_in_expr(&start.node, op_name);
                self.check_pipeline_types_in_expr(&end.node, op_name);
            }
            Expr::Pipeline { left, right, .. } => {
                self.check_pipeline_types_in_expr(&left.node, op_name);
                self.check_pipeline_types_in_expr(&right.node, op_name);
            }
            Expr::Resume { expr } => {
                self.check_pipeline_types_in_expr(&expr.node, op_name);
            }
            Expr::List { elements } => {
                for element in elements {
                    self.check_pipeline_types_in_expr(&element.node, op_name);
                }
            }
            Expr::Map { items } => {
                for item in items {
                    self.check_pipeline_types_in_expr(&item.node.value.node, op_name);
                }
            }
            Expr::Paren { inner } => {
                self.check_pipeline_types_in_expr(&inner.node, op_name);
            }
            Expr::Literal(_) | Expr::Identifier(_) => {}
        }
    }

    /// Validate that `prev.output` matches the first input type of `curr`.
    fn check_stage_type_compatibility(
        &mut self,
        prev_name: &str,
        prev: &OperationInfo,
        curr_name: &str,
        curr: &OperationInfo,
        span: Span,
        context: &str,
    ) {
        let Some(prev_output) = &prev.output else {
            return;
        };
        let Some((curr_input_name, curr_input_ty)) = curr.inputs.first() else {
            return;
        };

        if !Self::types_compatible(prev_output, curr_input_ty) {
            self.sink.emit(Diagnostic::error(
                ErrorCode::TypeMismatch,
                format!(
                    "Type mismatch in {}: stage '{}' outputs {}, but stage '{}' expects first input '{}' as {}",
                    context,
                    prev_name,
                    Self::format_type_expr(prev_output),
                    curr_name,
                    curr_input_name,
                    Self::format_type_expr(curr_input_ty),
                ),
                span,
            ));
        }
    }

    fn types_compatible(left: &TypeExpr, right: &TypeExpr) -> bool {
        Self::format_type_expr(left) == Self::format_type_expr(right)
    }

    fn format_type_expr(ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Named { name, params } => {
                if params.is_empty() {
                    name.node.clone()
                } else {
                    let params = params
                        .iter()
                        .map(|p| Self::format_type_expr(&p.node))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}[{}]", name.node, params)
                }
            }
            TypeExpr::Record { fields } => {
                let fields = fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}: {}",
                            f.node.name.node,
                            Self::format_type_expr(&f.node.ty.node)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", fields)
            }
            TypeExpr::Union { types } => types
                .iter()
                .map(|t| Self::format_type_expr(&t.node))
                .collect::<Vec<_>>()
                .join(" | "),
            TypeExpr::Constrained { ty, .. } => Self::format_type_expr(&ty.node),
        }
    }

    /// Pass 8: Generate VCs, solve with stub solver, emit diagnostics on invalid,
    /// and prepare synthetic ASSERT rewrites for unknown results.
    fn run_vc_pipeline(&mut self, program: &al_ast::Program) {
        let mut generator = VcGenerator::new();
        let mut vcs = generator.generate_program(program);
        for vc in &mut vcs {
            let _ = self.vc_solver.solve(vc);
        }

        let mut hir = al_hir::lower_program(program);
        Self::populate_required_caps(&mut hir);
        let rewrites = al_vc::apply_vc_results(&vcs, &mut hir, &mut self.sink);

        self.vc_results = vcs;
        self.synthetic_asserts = rewrites;
        self.hir_after_vc = Some(hir);
    }

    fn populate_required_caps(hir: &mut al_hir::HirProgram) {
        for decl in &mut hir.declarations {
            if let al_hir::HirDeclaration::Operation { body, meta, .. } = decl {
                let mut op_caps = HashSet::new();
                for stmt in body {
                    let stmt_caps = Self::required_caps_for_stmt(stmt);
                    let stmt_meta = Self::statement_meta_mut(stmt);
                    stmt_meta.required_caps = stmt_caps.iter().map(|cap| cap.to_string()).collect();
                    op_caps.extend(stmt_caps.into_iter().map(|cap| cap.to_string()));
                }
                let mut caps: Vec<String> = op_caps.into_iter().collect();
                caps.sort();
                meta.required_caps = caps;
            }
        }
    }

    fn required_caps_for_stmt(stmt: &al_hir::HirStatement) -> Vec<&'static str> {
        match stmt {
            al_hir::HirStatement::Delegate { .. } => vec!["DELEGATE"],
            _ => Vec::new(),
        }
    }

    fn statement_meta_mut(stmt: &mut al_hir::HirStatement) -> &mut al_hir::HirMeta {
        match stmt {
            al_hir::HirStatement::Assert { meta, .. }
            | al_hir::HirStatement::Retry { meta, .. }
            | al_hir::HirStatement::Escalate { meta, .. }
            | al_hir::HirStatement::Checkpoint { meta, .. }
            | al_hir::HirStatement::Resume { meta, .. }
            | al_hir::HirStatement::Fork { meta, .. }
            | al_hir::HirStatement::Delegate { meta, .. }
            | al_hir::HirStatement::Store { meta, .. }
            | al_hir::HirStatement::Mutable { meta, .. }
            | al_hir::HirStatement::Assign { meta, .. }
            | al_hir::HirStatement::Match { meta, .. }
            | al_hir::HirStatement::Loop { meta, .. }
            | al_hir::HirStatement::Emit { meta, .. }
            | al_hir::HirStatement::Halt { meta, .. }
            | al_hir::HirStatement::Expr { meta, .. } => meta,
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
    use al_vc::StubSolverMode;

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

    #[test]
    fn require_references_store_binding() {
        let source = r#"
OPERATION Validate =>
  INPUT data: Int64
  REQUIRE cached GT 0
  BODY {
    STORE cached = data
    EMIT cached
  }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        let require_errors: Vec<_> = checker
            .sink
            .errors()
            .into_iter()
            .filter(|e| e.message.contains("REQUIRE"))
            .collect();
        assert!(
            require_errors.is_empty(),
            "REQUIRE should accept enclosing STORE bindings"
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

    #[test]
    fn pipeline_stage_type_mismatch_detected() {
        let source = r#"
OPERATION Fetch =>
  OUTPUT Int64
  BODY { EMIT 1 }

OPERATION Normalize =>
  INPUT text: Str
  OUTPUT Str
  BODY { EMIT text }

PIPELINE P => Fetch -> Normalize
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(checker.has_errors(), "pipeline type mismatch should error");
        let errors = checker.sink.errors();
        assert!(errors.iter().any(|e| {
            e.code == al_diagnostics::DiagnosticCode::Error(ErrorCode::TypeMismatch)
                && e.message.contains("Fetch")
                && e.message.contains("Normalize")
        }));
    }

    #[test]
    fn pipeline_stage_type_match_accepted() {
        let source = r#"
OPERATION FetchText =>
  OUTPUT Str
  BODY { EMIT "x" }

OPERATION Normalize =>
  INPUT text: Str
  OUTPUT Str
  BODY { EMIT text }

PIPELINE P => FetchText -> Normalize
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(
            !checker.has_errors(),
            "compatible operation chains should pass type propagation"
        );
    }

    #[test]
    fn vc_pipeline_generates_require_ensure_assert() {
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
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert_eq!(checker.vc_results.len(), 3);
        assert!(checker
            .vc_results
            .iter()
            .all(|vc| vc.vc_id.starts_with("vc_")));
    }

    #[test]
    fn vc_pipeline_generates_invariant_boundary_vcs() {
        let source = r#"
OPERATION Verify =>
  INPUT x: Int64
  INVARIANT x GTE 0
  BODY { EMIT x }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert_eq!(checker.vc_results.len(), 2);
        assert!(checker
            .vc_results
            .iter()
            .any(|vc| vc.origin == al_vc::VcOrigin::InvariantLoopEntry));
        assert!(checker
            .vc_results
            .iter()
            .any(|vc| vc.origin == al_vc::VcOrigin::InvariantIterationBoundary));
    }

    #[test]
    fn vc_unknown_results_create_synthetic_assert_rewrites() {
        let source = r#"
OPERATION Verify =>
  INPUT x: Int64
  REQUIRE x GT 0
  BODY { EMIT x }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(!checker.has_errors());
        assert_eq!(checker.synthetic_asserts.len(), 1);
        let hir = checker
            .hir_after_vc
            .as_ref()
            .expect("hir should be captured");
        let op = hir
            .declarations
            .iter()
            .find(
                |d| matches!(d, al_hir::HirDeclaration::Operation { name, .. } if name == "Verify"),
            )
            .expect("operation exists");
        if let al_hir::HirDeclaration::Operation { body, .. } = op {
            assert!(matches!(
                body.last(),
                Some(al_hir::HirStatement::Assert { meta, .. }) if meta.synthetic
            ));
        }
    }

    #[test]
    fn vc_invalid_is_compile_error() {
        let source = r#"
OPERATION Verify =>
  INPUT x: Int64
  REQUIRE x GT 0
  BODY { EMIT x }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::with_vc_solver(StubSolverConfig {
            default_mode: StubSolverMode::AlwaysInvalid {
                counterexample: "x = -1".to_string(),
            },
            per_vc: HashMap::new(),
        });
        checker.check(&program);
        assert!(checker.has_errors());
        assert!(checker
            .sink
            .errors()
            .iter()
            .any(|e| { e.code == al_diagnostics::DiagnosticCode::Error(ErrorCode::VcInvalid) }));
    }

    #[test]
    fn delegate_requires_declared_delegate_capability() {
        let source = r#"
AGENT Worker =>
  CAPABILITIES [FILE_READ]

OPERATION Route =>
  BODY {
    DELEGATE task TO Worker => { }
    EMIT 1
  }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(checker.sink.errors().iter().any(|e| {
            e.code == al_diagnostics::DiagnosticCode::Error(ErrorCode::CapabilityDenied)
                && e.message.contains("DELEGATE statement is not permitted")
        }));
    }

    #[test]
    fn delegate_requires_known_target_agent() {
        let source = r#"
AGENT Caller =>
  CAPABILITIES [delegate]

OPERATION Route =>
  BODY {
    DELEGATE task TO MissingWorker => { }
    EMIT 1
  }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        assert!(checker.sink.errors().iter().any(|e| {
            e.code == al_diagnostics::DiagnosticCode::Error(ErrorCode::UnknownIdentifier)
                && e.message
                    .contains("Unknown delegate target agent 'MissingWorker'")
        }));
    }

    #[test]
    fn hir_required_caps_populated_for_delegate() {
        let source = r#"
AGENT Caller =>
  CAPABILITIES [delegate]

AGENT Worker =>
  CAPABILITIES [FILE_READ]

OPERATION Route =>
  BODY {
    DELEGATE task TO Worker => { }
    EMIT 1
  }
"#;
        let program = al_parser::parse(source).unwrap();
        let mut checker = TypeChecker::new();
        checker.check(&program);
        let hir = checker
            .hir_after_vc
            .as_ref()
            .expect("hir should be captured");
        let op = hir
            .declarations
            .iter()
            .find(
                |d| matches!(d, al_hir::HirDeclaration::Operation { name, .. } if name == "Route"),
            )
            .expect("operation exists");
        if let al_hir::HirDeclaration::Operation { body, meta, .. } = op {
            assert_eq!(meta.required_caps, vec!["DELEGATE".to_string()]);
            assert!(matches!(
                &body[0],
                al_hir::HirStatement::Delegate { meta, .. } if meta.required_caps == vec!["DELEGATE".to_string()]
            ));
        }
    }

    // ── Property-based tests ────────────────────────────────────────

    mod proptest_types {
        use super::*;
        use proptest::prelude::*;

        /// Strategy for builtin type names.
        fn builtin_type() -> impl Strategy<Value = &'static str> {
            prop::sample::select(vec![
                "Int64", "Float64", "Str", "Bool", "Int", "Float", "String",
            ])
        }

        /// All AgentLang keywords to filter.
        const KEYWORDS: &[&str] = &[
            "TYPE",
            "SCHEMA",
            "AGENT",
            "OPERATION",
            "PIPELINE",
            "BODY",
            "INPUT",
            "OUTPUT",
            "REQUIRE",
            "ENSURE",
            "INVARIANT",
            "STORE",
            "MUTABLE",
            "MATCH",
            "WHEN",
            "OTHERWISE",
            "LOOP",
            "EMIT",
            "ASSERT",
            "RETRY",
            "ESCALATE",
            "CHECKPOINT",
            "RESUME",
            "HALT",
            "DELEGATE",
            "TO",
            "FORK",
            "JOIN",
            "SUCCESS",
            "FAILURE",
            "TRUE",
            "FALSE",
            "NONE",
            "AND",
            "OR",
            "NOT",
            "EQ",
            "NEQ",
            "GT",
            "GTE",
            "LT",
            "LTE",
        ];

        /// Strategy for valid type names (uppercase start, not keywords).
        fn type_name() -> impl Strategy<Value = String> {
            "[A-Z][a-z][a-zA-Z]{0,6}"
                .prop_filter("not a keyword", |s| !KEYWORDS.contains(&s.as_str()))
        }

        proptest! {
            /// TYPE decls with builtin types always typecheck without errors.
            #[test]
            fn typecheck_valid_type_decl(
                name in type_name(),
                ty in builtin_type()
            ) {
                let source = format!("TYPE {} = {}", name, ty);
                if let Ok(program) = al_parser::parse(&source) {
                    let mut checker = TypeChecker::new();
                    checker.check(&program);
                    prop_assert!(
                        !checker.has_errors(),
                        "Valid TYPE decl should not error: {} (errors: {:?})",
                        source,
                        checker.sink.errors()
                    );
                }
            }

            /// Duplicate TYPE declarations always produce DUPLICATE_DEFINITION.
            #[test]
            fn typecheck_duplicate_type(
                name in type_name(),
                ty1 in builtin_type(),
                ty2 in builtin_type(),
            ) {
                let source = format!("TYPE {} = {}\nTYPE {} = {}", name, ty1, name, ty2);
                if let Ok(program) = al_parser::parse(&source) {
                    let mut checker = TypeChecker::new();
                    checker.check(&program);
                    prop_assert!(
                        checker.has_errors(),
                        "Duplicate TYPE should error: {}",
                        source
                    );
                }
            }

            /// Undefined type references always produce errors.
            #[test]
            fn typecheck_undefined_type(
                name in type_name(),
                undef in "[A-Z][a-z]{6,10}", // long enough to not match builtins
            ) {
                // Skip if the random name happens to be a builtin
                if !BUILTIN_TYPES.contains(&undef.as_str()) {
                    let source = format!("TYPE {} = {}", name, undef);
                    if let Ok(program) = al_parser::parse(&source) {
                        let mut checker = TypeChecker::new();
                        checker.check(&program);
                        prop_assert!(
                            checker.has_errors(),
                            "Undefined type ref should error: {}",
                            source
                        );
                    }
                }
            }

            /// OPERATION with valid REQUIRE clause should typecheck.
            #[test]
            fn typecheck_require_valid(val in 1i64..1000) {
                let source = format!(
                    "OPERATION Test =>\n  INPUT x: Int64\n  REQUIRE x GT {}\n  BODY {{ EMIT x }}",
                    val
                );
                if let Ok(program) = al_parser::parse(&source) {
                    let mut checker = TypeChecker::new();
                    checker.check(&program);
                    prop_assert!(
                        !checker.has_errors(),
                        "Valid REQUIRE should not error: {}",
                        source
                    );
                }
            }
        }
    }
}
