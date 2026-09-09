use serde::{Deserialize, Serialize};
use std::fmt;

use crate::span::Span;

/// An expression node in the SQL AST.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn identifier(name: String, span: Span) -> Self {
        Self::new(ExprKind::Identifier(name), span)
    }

    pub fn integer(value: i64, span: Span) -> Self {
        Self::new(ExprKind::Literal(Literal::Integer(value)), span)
    }

    pub fn string(value: String, span: Span) -> Self {
        Self::new(ExprKind::Literal(Literal::String(value)), span)
    }

    pub fn boolean(value: bool, span: Span) -> Self {
        Self::new(ExprKind::Literal(Literal::Boolean(value)), span)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Column or table identifier.
    Identifier(String),

    /// Literal value.
    Literal(Literal),

    /// Unary operation.
    Unary { op: UnaryOp, expr: Box<Expr> },

    /// Binary operation.
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,

    // Logical
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

impl Value {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Integer(_) => "INTEGER",
            Self::Float(_) => "FLOAT",
            Self::String(_) => "STRING",
            Self::Boolean(_) => "BOOLEAN",
            Self::Null => "NULL",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(value) => value.fmt(formatter),
            Self::Float(value) => value.fmt(formatter),
            Self::String(value) => formatter.write_str(value),
            Self::Boolean(value) => value.fmt(formatter),
            Self::Null => formatter.write_str("NULL"),
        }
    }
}
