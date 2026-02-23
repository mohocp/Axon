//! # AgentLang HIR (High-level Intermediate Representation)
//!
//! HIR is lowered from AST and carries additional type/capability/vc metadata.
//! The lowering pass attaches metadata placeholders that are populated during
//! type checking and verification condition generation.

use al_ast::{self, Declaration, Expr, Statement};
use al_diagnostics::Span;

/// HIR node metadata carried on every node.
#[derive(Debug, Clone, PartialEq)]
pub struct HirMeta {
    pub span: Span,
    pub ty: Option<String>,
    pub required_caps: Vec<String>,
    pub profile: String,
    pub synthetic: bool,
}

impl HirMeta {
    pub fn new(span: Span) -> Self {
        Self {
            span,
            ty: None,
            required_caps: Vec::new(),
            profile: "mvp-0.1".to_string(),
            synthetic: false,
        }
    }

    pub fn synthetic(span: Span) -> Self {
        Self {
            span,
            ty: None,
            required_caps: Vec::new(),
            profile: "mvp-0.1".to_string(),
            synthetic: true,
        }
    }
}

// ---------------------------------------------------------------------------
// HIR node types
// ---------------------------------------------------------------------------

/// A complete HIR program (lowered from an AST `Program`).
#[derive(Debug, Clone, PartialEq)]
pub struct HirProgram {
    pub declarations: Vec<HirDeclaration>,
    pub meta: HirMeta,
}

/// A top-level declaration in HIR.
#[derive(Debug, Clone, PartialEq)]
pub enum HirDeclaration {
    Type {
        name: String,
        meta: HirMeta,
    },
    Schema {
        name: String,
        field_count: usize,
        meta: HirMeta,
    },
    Agent {
        name: String,
        capabilities: Vec<String>,
        meta: HirMeta,
    },
    Operation {
        name: String,
        input_count: usize,
        has_output: bool,
        body: Vec<HirStatement>,
        meta: HirMeta,
    },
    Pipeline {
        name: String,
        stage_count: usize,
        meta: HirMeta,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirStatement {
    Assert { condition: HirExpr, meta: HirMeta },
    Retry { count: u64, meta: HirMeta },
    Escalate { message: Option<String>, meta: HirMeta },
    Checkpoint { label: Option<String>, meta: HirMeta },
    Resume { expr: HirExpr, meta: HirMeta },
    Fork { branches: Vec<HirBranch>, meta: HirMeta },
    Delegate { task: String, target: String, meta: HirMeta },
    Store { name: String, meta: HirMeta },
    Mutable { name: String, reason: String, meta: HirMeta },
    Assign { target: String, meta: HirMeta },
    Match { arm_count: usize, has_otherwise: bool, meta: HirMeta },
    Loop { max_iters: u64, meta: HirMeta },
    Emit { meta: HirMeta },
    Halt { reason: String, meta: HirMeta },
    Expr { meta: HirMeta },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirExpr {
    pub kind: HirExprKind,
    pub meta: HirMeta,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirExprKind {
    Literal(String),
    Identifier(String),
    Call { func: String, args: Vec<HirExpr> },
    BinaryOp { op: String },
    UnaryOp { op: String },
    Member { field: String },
    Pipeline,
    List { count: usize },
    Map { count: usize },
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirBranch {
    pub name: String,
    pub meta: HirMeta,
}

// ---------------------------------------------------------------------------
// Lowering: AST -> HIR
// ---------------------------------------------------------------------------

/// Lower an AST `Program` into an HIR `HirProgram`.
///
/// This attaches metadata placeholders (type = None, required_caps = [])
/// that later passes will populate.
pub fn lower_program(program: &al_ast::Program) -> HirProgram {
    let declarations = program
        .declarations
        .iter()
        .map(|d| lower_declaration(&d.node, d.span))
        .collect();

    HirProgram {
        declarations,
        meta: HirMeta::new(program.span),
    }
}

fn lower_declaration(decl: &Declaration, span: Span) -> HirDeclaration {
    match decl {
        Declaration::TypeDecl { name, .. } => HirDeclaration::Type {
            name: name.node.clone(),
            meta: HirMeta::new(span),
        },
        Declaration::SchemaDecl { name, fields } => HirDeclaration::Schema {
            name: name.node.clone(),
            field_count: fields.len(),
            meta: HirMeta::new(span),
        },
        Declaration::AgentDecl { name, properties } => {
            let capabilities: Vec<String> = properties
                .iter()
                .filter_map(|p| match &p.node {
                    al_ast::AgentProperty::Capabilities(caps) => {
                        Some(caps.iter().map(|c| c.node.clone()).collect::<Vec<_>>())
                    }
                    _ => None,
                })
                .flatten()
                .collect();
            HirDeclaration::Agent {
                name: name.node.clone(),
                capabilities,
                meta: HirMeta::new(span),
            }
        }
        Declaration::OperationDecl {
            name,
            inputs,
            output,
            body,
            ..
        } => {
            let hir_body = body
                .node
                .stmts
                .iter()
                .map(|s| lower_statement(&s.node, s.span))
                .collect();
            HirDeclaration::Operation {
                name: name.node.clone(),
                input_count: inputs.len(),
                has_output: output.is_some(),
                body: hir_body,
                meta: HirMeta::new(span),
            }
        }
        Declaration::PipelineDecl { name, chain } => HirDeclaration::Pipeline {
            name: name.node.clone(),
            stage_count: chain.node.stages.len(),
            meta: HirMeta::new(span),
        },
    }
}

fn lower_statement(stmt: &Statement, span: Span) -> HirStatement {
    match stmt {
        Statement::Assert { condition } => HirStatement::Assert {
            condition: lower_expr(&condition.node, condition.span),
            meta: HirMeta::new(span),
        },
        Statement::Retry { count, .. } => HirStatement::Retry {
            count: count.node as u64,
            meta: HirMeta::new(span),
        },
        Statement::Escalate { message } => HirStatement::Escalate {
            message: message.as_ref().and_then(|m| {
                if let Expr::Literal(al_ast::Literal::String(s)) = &m.node {
                    Some(s.clone())
                } else {
                    None
                }
            }),
            meta: HirMeta::new(span),
        },
        Statement::Checkpoint { label } => HirStatement::Checkpoint {
            label: label.as_ref().map(|l| l.node.clone()),
            meta: HirMeta::new(span),
        },
        Statement::Store { name, .. } => HirStatement::Store {
            name: name.node.clone(),
            meta: HirMeta::new(span),
        },
        Statement::Mutable { name, reason, .. } => HirStatement::Mutable {
            name: name.node.clone(),
            reason: reason.node.clone(),
            meta: HirMeta::new(span),
        },
        Statement::Assign { target, .. } => HirStatement::Assign {
            target: target.node.clone(),
            meta: HirMeta::new(span),
        },
        Statement::Match {
            arms, otherwise, ..
        } => HirStatement::Match {
            arm_count: arms.len(),
            has_otherwise: otherwise.is_some(),
            meta: HirMeta::new(span),
        },
        Statement::Loop { max_iters, .. } => HirStatement::Loop {
            max_iters: max_iters.node as u64,
            meta: HirMeta::new(span),
        },
        Statement::Emit { .. } => HirStatement::Emit {
            meta: HirMeta::new(span),
        },
        Statement::Halt { reason, .. } => HirStatement::Halt {
            reason: reason.node.clone(),
            meta: HirMeta::new(span),
        },
        Statement::Delegate { task, target, .. } => HirStatement::Delegate {
            task: task.node.clone(),
            target: target.node.clone(),
            meta: HirMeta::new(span),
        },
        Statement::Expr { expr: _ } => HirStatement::Expr {
            meta: HirMeta::new(span),
        },
    }
}

fn lower_expr(expr: &Expr, span: Span) -> HirExpr {
    let kind = match expr {
        Expr::Literal(lit) => HirExprKind::Literal(format!("{:?}", lit)),
        Expr::Identifier(name) => HirExprKind::Identifier(name.clone()),
        Expr::Call { func, args } => {
            let func_name = match &func.node {
                Expr::Identifier(name) => name.clone(),
                _ => "<complex>".to_string(),
            };
            let hir_args = args
                .iter()
                .map(|a| lower_expr(&a.node.value.node, a.node.value.span))
                .collect();
            HirExprKind::Call {
                func: func_name,
                args: hir_args,
            }
        }
        Expr::BinaryOp { op, .. } => HirExprKind::BinaryOp {
            op: format!("{:?}", op.node),
        },
        Expr::UnaryOp { op, .. } => HirExprKind::UnaryOp {
            op: format!("{:?}", op.node),
        },
        Expr::Member { field, .. } => HirExprKind::Member {
            field: field.node.clone(),
        },
        Expr::List { elements } => HirExprKind::List {
            count: elements.len(),
        },
        Expr::Map { items } => HirExprKind::Map {
            count: items.len(),
        },
        _ => HirExprKind::Other,
    };

    HirExpr {
        kind,
        meta: HirMeta::new(span),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_empty_program() {
        let program = al_ast::Program {
            declarations: vec![],
            span: Span::dummy(),
        };
        let hir = lower_program(&program);
        assert!(hir.declarations.is_empty());
        assert_eq!(hir.meta.profile, "mvp-0.1");
    }

    #[test]
    fn lower_type_decl() {
        let source = "TYPE UserId = Int64";
        let program = al_parser::parse(source).unwrap();
        let hir = lower_program(&program);
        assert_eq!(hir.declarations.len(), 1);
        match &hir.declarations[0] {
            HirDeclaration::Type { name, .. } => assert_eq!(name, "UserId"),
            _ => panic!("expected HirDeclaration::Type"),
        }
    }

    #[test]
    fn lower_operation_with_body() {
        let source = r#"OPERATION Test =>
  INPUT x: Int64
  BODY {
    STORE y = x
    EMIT y
  }"#;
        let program = al_parser::parse(source).unwrap();
        let hir = lower_program(&program);
        match &hir.declarations[0] {
            HirDeclaration::Operation {
                name,
                input_count,
                body,
                ..
            } => {
                assert_eq!(name, "Test");
                assert_eq!(*input_count, 1);
                assert_eq!(body.len(), 2);
            }
            _ => panic!("expected HirDeclaration::Operation"),
        }
    }

    #[test]
    fn hir_meta_default_profile() {
        let meta = HirMeta::new(Span::dummy());
        assert_eq!(meta.profile, "mvp-0.1");
        assert!(!meta.synthetic);
        assert!(meta.ty.is_none());
    }

    #[test]
    fn hir_meta_synthetic() {
        let meta = HirMeta::synthetic(Span::dummy());
        assert!(meta.synthetic);
    }
}
