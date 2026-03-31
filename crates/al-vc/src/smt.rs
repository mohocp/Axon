//! Solver-agnostic SMT expression representation and AST-to-SMT translation.
//!
//! `SmtExpr` is the intermediate representation between AgentLang AST expressions
//! and solver backends. Any solver (SimpleSolver, Z3, CVC5) consumes `SmtExpr`.

use al_ast::{BinaryOp, Expr, Literal, UnaryOp};

/// SMT sort (type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtSort {
    Int,
    Real,
    Bool,
    String,
}

/// Solver-agnostic SMT expression.
#[derive(Debug, Clone, PartialEq)]
pub enum SmtExpr {
    // Literals
    IntLit(i64),
    RealLit(f64),
    BoolLit(bool),
    StringLit(String),

    // Variables
    Var {
        name: String,
        sort: SmtSort,
    },

    // Arithmetic
    Add(Box<SmtExpr>, Box<SmtExpr>),
    Sub(Box<SmtExpr>, Box<SmtExpr>),
    Mul(Box<SmtExpr>, Box<SmtExpr>),
    Div(Box<SmtExpr>, Box<SmtExpr>),
    Mod(Box<SmtExpr>, Box<SmtExpr>),
    Neg(Box<SmtExpr>),

    // Comparison
    Gt(Box<SmtExpr>, Box<SmtExpr>),
    Gte(Box<SmtExpr>, Box<SmtExpr>),
    Lt(Box<SmtExpr>, Box<SmtExpr>),
    Lte(Box<SmtExpr>, Box<SmtExpr>),
    Eq(Box<SmtExpr>, Box<SmtExpr>),
    Neq(Box<SmtExpr>, Box<SmtExpr>),

    // Boolean
    And(Box<SmtExpr>, Box<SmtExpr>),
    Or(Box<SmtExpr>, Box<SmtExpr>),
    Not(Box<SmtExpr>),
    Implies(Box<SmtExpr>, Box<SmtExpr>),

    /// Opaque expression that the solver cannot reason about.
    Unknown(String),
}

impl SmtExpr {
    pub fn int(v: i64) -> Self {
        SmtExpr::IntLit(v)
    }
    pub fn real(v: f64) -> Self {
        SmtExpr::RealLit(v)
    }
    pub fn bool(v: bool) -> Self {
        SmtExpr::BoolLit(v)
    }
    pub fn var(name: impl Into<String>, sort: SmtSort) -> Self {
        SmtExpr::Var {
            name: name.into(),
            sort,
        }
    }

    /// Check if this expression is opaque (cannot be reasoned about).
    pub fn is_unknown(&self) -> bool {
        matches!(self, SmtExpr::Unknown(_))
    }

    /// Structurally equal, ignoring variable naming for comparison purposes.
    pub fn structurally_eq(&self, other: &SmtExpr) -> bool {
        match (self, other) {
            (SmtExpr::IntLit(a), SmtExpr::IntLit(b)) => a == b,
            (SmtExpr::RealLit(a), SmtExpr::RealLit(b)) => (a - b).abs() < f64::EPSILON,
            (SmtExpr::BoolLit(a), SmtExpr::BoolLit(b)) => a == b,
            (SmtExpr::StringLit(a), SmtExpr::StringLit(b)) => a == b,
            (SmtExpr::Var { name: a, .. }, SmtExpr::Var { name: b, .. }) => a == b,
            (SmtExpr::Add(a1, a2), SmtExpr::Add(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Sub(a1, a2), SmtExpr::Sub(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Mul(a1, a2), SmtExpr::Mul(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Div(a1, a2), SmtExpr::Div(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Mod(a1, a2), SmtExpr::Mod(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Neg(a), SmtExpr::Neg(b)) => a.structurally_eq(b),
            (SmtExpr::Gt(a1, a2), SmtExpr::Gt(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Gte(a1, a2), SmtExpr::Gte(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Lt(a1, a2), SmtExpr::Lt(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Lte(a1, a2), SmtExpr::Lte(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Eq(a1, a2), SmtExpr::Eq(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Neq(a1, a2), SmtExpr::Neq(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::And(a1, a2), SmtExpr::And(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Or(a1, a2), SmtExpr::Or(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            (SmtExpr::Not(a), SmtExpr::Not(b)) => a.structurally_eq(b),
            (SmtExpr::Implies(a1, a2), SmtExpr::Implies(b1, b2)) => {
                a1.structurally_eq(b1) && a2.structurally_eq(b2)
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// AST → SmtExpr translator
// ---------------------------------------------------------------------------

/// Translates AgentLang AST expressions into solver-agnostic `SmtExpr`.
pub struct SmtTranslator;

impl SmtTranslator {
    /// Translate an AgentLang expression to an SMT expression.
    pub fn translate(expr: &Expr) -> SmtExpr {
        match expr {
            Expr::Literal(lit) => Self::translate_literal(lit),
            Expr::Identifier(name) => SmtExpr::Var {
                name: name.clone(),
                sort: SmtSort::Int, // default sort; refined by context
            },
            Expr::BinaryOp { left, op, right } => {
                let l = Self::translate(&left.node);
                let r = Self::translate(&right.node);
                Self::translate_binop(&op.node, l, r)
            }
            Expr::UnaryOp { op, operand } => {
                let inner = Self::translate(&operand.node);
                Self::translate_unop(&op.node, inner)
            }
            // Anything we can't translate becomes opaque.
            other => SmtExpr::Unknown(format!("{:?}", other)),
        }
    }

    fn translate_literal(lit: &Literal) -> SmtExpr {
        match lit {
            Literal::Integer(v) => SmtExpr::IntLit(*v),
            Literal::Float(v) => SmtExpr::RealLit(*v),
            Literal::Bool(v) => SmtExpr::BoolLit(*v),
            Literal::String(v) => SmtExpr::StringLit(v.clone()),
            Literal::None => SmtExpr::Unknown("None".to_string()),
            other => SmtExpr::Unknown(format!("{:?}", other)),
        }
    }

    fn translate_binop(op: &BinaryOp, left: SmtExpr, right: SmtExpr) -> SmtExpr {
        let l = Box::new(left);
        let r = Box::new(right);
        match op {
            BinaryOp::Add => SmtExpr::Add(l, r),
            BinaryOp::Sub => SmtExpr::Sub(l, r),
            BinaryOp::Mul => SmtExpr::Mul(l, r),
            BinaryOp::Div => SmtExpr::Div(l, r),
            BinaryOp::Mod => SmtExpr::Mod(l, r),
            BinaryOp::Gt => SmtExpr::Gt(l, r),
            BinaryOp::Gte => SmtExpr::Gte(l, r),
            BinaryOp::Lt => SmtExpr::Lt(l, r),
            BinaryOp::Lte => SmtExpr::Lte(l, r),
            BinaryOp::Eq => SmtExpr::Eq(l, r),
            BinaryOp::Neq => SmtExpr::Neq(l, r),
            BinaryOp::And => SmtExpr::And(l, r),
            BinaryOp::Or => SmtExpr::Or(l, r),
        }
    }

    fn translate_unop(op: &UnaryOp, inner: SmtExpr) -> SmtExpr {
        let inner = Box::new(inner);
        match op {
            UnaryOp::Not => SmtExpr::Not(inner),
            UnaryOp::Neg => SmtExpr::Neg(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_ast::{Literal, Spanned};
    use al_diagnostics::Span;

    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned {
            node,
            span: Span::dummy(),
        }
    }

    #[test]
    fn translate_int_literal() {
        let expr = Expr::Literal(Literal::Integer(42));
        assert_eq!(SmtTranslator::translate(&expr), SmtExpr::IntLit(42));
    }

    #[test]
    fn translate_float_literal() {
        let expr = Expr::Literal(Literal::Float(1.5));
        assert_eq!(SmtTranslator::translate(&expr), SmtExpr::RealLit(1.5));
    }

    #[test]
    fn translate_bool_literal() {
        let expr = Expr::Literal(Literal::Bool(true));
        assert_eq!(SmtTranslator::translate(&expr), SmtExpr::BoolLit(true));
    }

    #[test]
    fn translate_identifier() {
        let expr = Expr::Identifier("x".to_string());
        assert_eq!(
            SmtTranslator::translate(&expr),
            SmtExpr::Var {
                name: "x".to_string(),
                sort: SmtSort::Int
            }
        );
    }

    #[test]
    fn translate_binary_gt() {
        let expr = Expr::BinaryOp {
            left: Box::new(spanned(Expr::Identifier("x".to_string()))),
            op: spanned(BinaryOp::Gt),
            right: Box::new(spanned(Expr::Literal(Literal::Integer(0)))),
        };
        let smt = SmtTranslator::translate(&expr);
        assert!(matches!(smt, SmtExpr::Gt(_, _)));
    }

    #[test]
    fn translate_compound_expression() {
        // (x + 1) GT 0
        let expr = Expr::BinaryOp {
            left: Box::new(spanned(Expr::BinaryOp {
                left: Box::new(spanned(Expr::Identifier("x".to_string()))),
                op: spanned(BinaryOp::Add),
                right: Box::new(spanned(Expr::Literal(Literal::Integer(1)))),
            })),
            op: spanned(BinaryOp::Gt),
            right: Box::new(spanned(Expr::Literal(Literal::Integer(0)))),
        };
        let smt = SmtTranslator::translate(&expr);
        match smt {
            SmtExpr::Gt(left, right) => {
                assert!(matches!(*left, SmtExpr::Add(_, _)));
                assert_eq!(*right, SmtExpr::IntLit(0));
            }
            _ => panic!("Expected Gt"),
        }
    }

    #[test]
    fn translate_not_unary() {
        let expr = Expr::UnaryOp {
            op: spanned(UnaryOp::Not),
            operand: Box::new(spanned(Expr::Literal(Literal::Bool(true)))),
        };
        let smt = SmtTranslator::translate(&expr);
        assert!(matches!(smt, SmtExpr::Not(_)));
    }

    #[test]
    fn translate_boolean_combination() {
        // (x GT 0) AND (x LT 100)
        let expr = Expr::BinaryOp {
            left: Box::new(spanned(Expr::BinaryOp {
                left: Box::new(spanned(Expr::Identifier("x".to_string()))),
                op: spanned(BinaryOp::Gt),
                right: Box::new(spanned(Expr::Literal(Literal::Integer(0)))),
            })),
            op: spanned(BinaryOp::And),
            right: Box::new(spanned(Expr::BinaryOp {
                left: Box::new(spanned(Expr::Identifier("x".to_string()))),
                op: spanned(BinaryOp::Lt),
                right: Box::new(spanned(Expr::Literal(Literal::Integer(100)))),
            })),
        };
        let smt = SmtTranslator::translate(&expr);
        assert!(matches!(smt, SmtExpr::And(_, _)));
    }

    #[test]
    fn translate_unsupported_returns_unknown() {
        let expr = Expr::Call {
            func: Box::new(spanned(Expr::Identifier("foo".to_string()))),
            args: vec![],
        };
        let smt = SmtTranslator::translate(&expr);
        assert!(smt.is_unknown());
    }

    #[test]
    fn structural_equality() {
        let a = SmtExpr::Gt(
            Box::new(SmtExpr::var("x", SmtSort::Int)),
            Box::new(SmtExpr::int(0)),
        );
        let b = SmtExpr::Gt(
            Box::new(SmtExpr::var("x", SmtSort::Int)),
            Box::new(SmtExpr::int(0)),
        );
        assert!(a.structurally_eq(&b));
    }

    #[test]
    fn all_arithmetic_ops_translate() {
        for op in &[
            BinaryOp::Add,
            BinaryOp::Sub,
            BinaryOp::Mul,
            BinaryOp::Div,
            BinaryOp::Mod,
        ] {
            let expr = Expr::BinaryOp {
                left: Box::new(spanned(Expr::Literal(Literal::Integer(1)))),
                op: spanned(*op),
                right: Box::new(spanned(Expr::Literal(Literal::Integer(2)))),
            };
            let smt = SmtTranslator::translate(&expr);
            assert!(!smt.is_unknown(), "Op {:?} should translate", op);
        }
    }

    #[test]
    fn all_comparison_ops_translate() {
        for op in &[
            BinaryOp::Gt,
            BinaryOp::Gte,
            BinaryOp::Lt,
            BinaryOp::Lte,
            BinaryOp::Eq,
            BinaryOp::Neq,
        ] {
            let expr = Expr::BinaryOp {
                left: Box::new(spanned(Expr::Literal(Literal::Integer(1)))),
                op: spanned(*op),
                right: Box::new(spanned(Expr::Literal(Literal::Integer(2)))),
            };
            let smt = SmtTranslator::translate(&expr);
            assert!(!smt.is_unknown(), "Op {:?} should translate", op);
        }
    }

    #[test]
    fn all_boolean_ops_translate() {
        for op in &[BinaryOp::And, BinaryOp::Or] {
            let expr = Expr::BinaryOp {
                left: Box::new(spanned(Expr::Literal(Literal::Bool(true)))),
                op: spanned(*op),
                right: Box::new(spanned(Expr::Literal(Literal::Bool(false)))),
            };
            let smt = SmtTranslator::translate(&expr);
            assert!(!smt.is_unknown(), "Op {:?} should translate", op);
        }
    }
}
