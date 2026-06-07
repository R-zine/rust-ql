use std::collections::HashMap;

use interface::{
    BinaryOp, CreateTableStatement, Expr, ExprKind, InsertStatement, Literal, SelectItem,
    SelectStatement, Statement, StatementKind, Value,
};

use crate::database::PrimaryKeyValue;
use crate::persistence::{load_db, save_db};

use crate::{
    database::{Database, Table},
    error::BackendError,
};

// ========================
// BINARY EVALUATION
// ========================

fn eval_binary(left: Value, op: BinaryOp, right: Value) -> bool {
    use Value::*;

    match (left, right) {
        (Integer(a), Integer(b)) => match op {
            BinaryOp::Equal => a == b,
            BinaryOp::NotEqual => a != b,
            BinaryOp::GreaterThan => a > b,
            BinaryOp::LessThan => a < b,
            BinaryOp::GreaterThanOrEqual => a >= b,
            BinaryOp::LessThanOrEqual => a <= b,
            _ => todo!(),
        },

        (String(a), String(b)) => match op {
            BinaryOp::Equal => a == b,
            BinaryOp::NotEqual => a != b,
            _ => false,
        },

        (Boolean(a), Boolean(b)) => match op {
            BinaryOp::Equal => a == b,
            BinaryOp::NotEqual => a != b,
            _ => false,
        },

        _ => false,
    }
}

// ========================
// EXPRESSION EVALUATION
// ========================

fn eval_expr(expr: &Expr, row: &[Value], table: &Table) -> Value {
    match &expr.kind {
        ExprKind::Literal(lit) => match lit {
            Literal::Integer(i) => Value::Integer(*i),
            Literal::Float(f) => Value::Float(*f),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Boolean(b) => Value::Boolean(*b),
            Literal::Null => Value::Null,
        },

        ExprKind::Identifier(name) => {
            let idx = table
                .columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(name));

            match idx {
                Some(i) => row.get(i).cloned().unwrap_or(Value::Null),
                None => Value::Null,
            }
        }

        ExprKind::Binary { left, op, right } => {
            let l = eval_expr(left, row, table);
            let r = eval_expr(right, row, table);

            Value::Boolean(eval_binary(l, *op, r))
        }

        _ => Value::Null,
    }
}

// ========================
// CONSTANT EVAL (INSERT)
// ========================

fn eval_const(expr: &Expr) -> Value {
    match &expr.kind {
        ExprKind::Literal(lit) => match lit {
            Literal::Integer(i) => Value::Integer(*i),
            Literal::Float(f) => Value::Float(*f),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Boolean(b) => Value::Boolean(*b),
            Literal::Null => Value::Null,
        },
        _ => Value::Null,
    }
}

// ========================
// FORMATTING
// ========================

fn format_value(v: &Value) -> String {
    match v {
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Boolean(b) => b.to_string(),
        Value::Null => "NULL".to_string(),
    }
}

fn print_projected_row(row: &[Value], table: &Table, stmt: &SelectStatement) {
    let select_all = stmt.columns.len() == 1 && matches!(stmt.columns[0], SelectItem::Wildcard);

    if select_all {
        let formatted: Vec<String> = row.iter().map(format_value).collect();

        println!("{}", formatted.join(" | "));
    } else {
        let formatted: Vec<String> = stmt
            .columns
            .iter()
            .filter_map(|item| match item {
                SelectItem::Expression { expr, .. } => {
                    Some(format_value(&eval_expr(expr, row, table)))
                }

                SelectItem::Wildcard => None,
            })
            .collect();

        println!("{}", formatted.join(" | "));
    }
}

// ========================
// EXECUTOR
// ========================

pub struct Executor {
    database: Database,
    path: String,
}

impl Executor {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let database = load_db(&path).unwrap_or_default();

        Self { database, path }
    }

    pub fn execute(&mut self, statement: Statement) -> Result<(), BackendError> {
        match statement.kind {
            StatementKind::CreateTable(stmt) => self.execute_create_table(stmt),
            StatementKind::Insert(stmt) => self.execute_insert(stmt),
            StatementKind::Select(stmt) => self.execute_select(stmt),
        }
    }

    // ========================
    // CREATE TABLE
    // ========================

    fn execute_create_table(&mut self, stmt: CreateTableStatement) -> Result<(), BackendError> {
        if self.database.tables.contains_key(&stmt.name) {
            return Err(BackendError::TableAlreadyExists(stmt.name));
        }

        let primary_key_column = stmt.columns.iter().position(|c| c.primary_key);

        self.database.tables.insert(
            stmt.name,
            Table {
                columns: stmt.columns,
                rows: Vec::new(),
                primary_key_column,
                primary_key_index: HashMap::new(),
            },
        );

        save_db(&self.database, &self.path)?;
        Ok(())
    }

    // ========================
    // INSERT
    // ========================
    fn execute_insert(&mut self, stmt: InsertStatement) -> Result<(), BackendError> {
        let table = self
            .database
            .tables
            .get_mut(&stmt.table)
            .ok_or_else(|| BackendError::TableNotFound(stmt.table.clone()))?;

        let mut row = vec![Value::Null; table.columns.len()];

        for (i, expr) in stmt.values.iter().enumerate() {
            if i < row.len() {
                row[i] = eval_const(expr);
            }
        }

        // Primary key validation
        if let Some(pk_col) = table.primary_key_column {
            let pk_value = PrimaryKeyValue::try_from(&row[pk_col]).map_err(|_| {
                BackendError::InvalidPrimaryKey("unsupported primary key type".to_string())
            })?;

            if table.primary_key_index.contains_key(&pk_value) {
                return Err(BackendError::DuplicatePrimaryKey(format!("{pk_value:?}")));
            }
        }

        let row_index = table.rows.len();

        table.rows.push(row);

        // Update index
        if let Some(pk_col) = table.primary_key_column {
            let pk_value =
                PrimaryKeyValue::try_from(&table.rows[row_index][pk_col]).map_err(|_| {
                    BackendError::InvalidPrimaryKey("unsupported primary key type".to_string())
                })?;

            table.primary_key_index.insert(pk_value, row_index);
        }

        save_db(&self.database, &self.path)?;

        Ok(())
    }

    // ========================
    // SELECT
    // ========================

    fn execute_select(&mut self, stmt: SelectStatement) -> Result<(), BackendError> {
        let table = self
            .database
            .tables
            .get(&stmt.table)
            .ok_or_else(|| BackendError::TableNotFound(stmt.table.clone()))?;

        // Fast path:
        // SELECT ... WHERE pk = literal
        if let Some(where_expr) = &stmt.where_clause
            && let ExprKind::Binary {
                left,
                op: BinaryOp::Equal,
                right,
            } = &where_expr.kind
                && let ExprKind::Identifier(column_name) = &left.kind
                    && let Some(pk_col) = table.primary_key_column {
                        let pk_name = &table.columns[pk_col].name;

                        if pk_name == column_name {
                            let literal_value = eval_const(right);

                            if let Ok(pk_value) = PrimaryKeyValue::try_from(&literal_value) {
                                if let Some(row_idx) = table.primary_key_index.get(&pk_value) {
                                    let row = &table.rows[*row_idx];

                                    print_projected_row(row, table, &stmt);

                                    return Ok(());
                                }

                                return Ok(());
                            }
                        }
                    }

        // Fallback: full scan
        for row in &table.rows {
            let mut matches = true;

            if let Some(where_expr) = &stmt.where_clause {
                let value = eval_expr(where_expr, row, table);

                matches = matches!(value, Value::Boolean(true));
            }

            if matches {
                print_projected_row(row, table, &stmt);
            }
        }

        Ok(())
    }

    pub fn seed(&mut self, table_name: &str, count: usize) -> Result<(), BackendError> {
        let table = self
            .database
            .tables
            .get_mut(table_name)
            .ok_or_else(|| BackendError::TableNotFound(table_name.to_string()))?;

        for i in 0..count {
            table.rows.push(vec![
                Value::Integer(i as i64),
                Value::String(format!("user{}", i)),
            ]);
        }

        save_db(&self.database, &self.path)?;

        Ok(())
    }
}
