use serde::{Deserialize, Serialize};
use std::fmt;

use crate::expr::Expr;
use crate::span::Span;

/// A complete SQL statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

impl Statement {
    pub fn new(kind: StatementKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    Select(SelectStatement),
    Insert(InsertStatement),
    CreateTable(CreateTableStatement),
}

/// SELECT ... FROM ... [WHERE ...]
#[derive(Debug, Clone, PartialEq)]
pub struct SelectStatement {
    pub columns: Vec<SelectItem>,
    pub table: String,
    pub where_clause: Option<Expr>,
}

/// A column in the SELECT list.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectItem {
    Wildcard,
    Expression { expr: Expr, alias: Option<String> },
}

/// INSERT INTO table (...) VALUES (...)
#[derive(Debug, Clone, PartialEq)]
pub struct InsertStatement {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Expr>,
}

/// CREATE TABLE ...
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTableStatement {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub primary_key: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Integer,
    Float,
    Boolean,
    String,
}

impl fmt::Display for DataType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Integer => "INTEGER",
            Self::Float => "FLOAT",
            Self::Boolean => "BOOLEAN",
            Self::String => "STRING",
        };

        formatter.write_str(name)
    }
}
