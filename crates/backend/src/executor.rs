use interface::{
    Statement,
    StatementKind,
    CreateTableStatement,
    InsertStatement,
    SelectStatement,
    Expr,
    ExprKind,
    Literal,
    Value,
    BinaryOp,
};

use crate::persistence::{save_db, load_db};

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
            _ => todo!()
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

fn eval_expr(expr: &Expr, row: &Vec<Value>, table: &Table) -> Value {

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

            Value::Boolean(eval_binary(l, op.clone(), r))
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

    pub fn execute(
        &mut self,
        statement: Statement,
    ) -> Result<(), BackendError> {
        match statement.kind {
            StatementKind::CreateTable(stmt) => {
                self.execute_create_table(stmt)
            }
            StatementKind::Insert(stmt) => {
                self.execute_insert(stmt)
            }
            StatementKind::Select(stmt) => {
                self.execute_select(stmt)
            }
        }
    }


    // ========================
    // CREATE TABLE
    // ========================

    fn execute_create_table(
        &mut self,
        stmt: CreateTableStatement,
    ) -> Result<(), BackendError> {
        if self.database.tables.contains_key(&stmt.name) {
            return Err(BackendError::TableAlreadyExists(stmt.name));
        }

        self.database.tables.insert(
            stmt.name,
            Table {
                columns: stmt.columns,
                rows: Vec::new(),
            },
        );

        save_db(&self.database, &self.path)?;
        Ok(())
    }


    // ========================
    // INSERT
    // ========================

    fn execute_insert(
        &mut self,
        stmt: InsertStatement,
    ) -> Result<(), BackendError> {
        let table = self
            .database
            .tables
            .get_mut(&stmt.table)
            .ok_or_else(|| {
                BackendError::TableNotFound(stmt.table.clone())
            })?;

        // IMPORTANT FIX:
        // ensure row aligns with schema size
        let mut row = vec![Value::Null; table.columns.len()];

        for (i, expr) in stmt.values.iter().enumerate() {
            if i < row.len() {
                row[i] = eval_const(expr);
            }
        }

        table.rows.push(row);

        save_db(&self.database, &self.path)?;
        Ok(())
    }


    // ========================
    // SELECT
    // ========================

    fn execute_select(
        &mut self,
        stmt: SelectStatement,
    ) -> Result<(), BackendError> {
        let table = self
            .database
            .tables
            .get(&stmt.table)
            .ok_or_else(|| {
                BackendError::TableNotFound(stmt.table.clone())
            })?;

        for row in &table.rows {
            let mut matches = true;

            if let Some(where_expr) = &stmt.where_clause {
                let value = eval_expr(where_expr, row, table);

                matches = matches!(value, Value::Boolean(true));
            }

            if matches {
                let formatted: Vec<String> =
                    row.iter().map(format_value).collect();

                println!("{}", formatted.join(" | "));
            }
        }

        Ok(())
    }
}