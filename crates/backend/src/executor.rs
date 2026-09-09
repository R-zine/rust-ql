use std::{collections::HashSet, path::PathBuf};

use interface::{
    BinaryOp, ColumnDefinition, CreateTableStatement, DataType, Expr, ExprKind, InsertStatement,
    SelectItem, SelectStatement, Statement, StatementKind, Value,
};

use crate::{
    database::{Database, PrimaryKeyValue, Row, Table},
    error::BackendError,
    evaluation::{eval_const, eval_on_row, validate_expression, validate_where_expression},
    persistence::{load_db, save_db},
};

#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    TableCreated { table: String },
    RowsInserted { count: usize },
    Query(QueryResult),
}

pub struct Executor {
    database: Database,
    path: PathBuf,
}

impl Executor {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, BackendError> {
        let path = path.into();
        let database = load_db(&path)?;
        Ok(Self { database, path })
    }

    pub fn execute(&mut self, statement: Statement) -> Result<ExecutionResult, BackendError> {
        match statement.kind {
            StatementKind::CreateTable(statement) => self.execute_create_table(statement),
            StatementKind::Insert(statement) => self.execute_insert(statement),
            StatementKind::Select(statement) => self.execute_select(statement),
        }
    }

    fn execute_create_table(
        &mut self,
        statement: CreateTableStatement,
    ) -> Result<ExecutionResult, BackendError> {
        if find_table_name(&self.database, &statement.name).is_some() {
            return Err(BackendError::TableAlreadyExists(statement.name));
        }

        let table = Table::new(statement.columns).map_err(BackendError::InvalidSchema)?;
        let name = statement.name;
        let mut next_database = self.database.clone();
        next_database.tables.insert(name.clone(), table);
        save_db(&next_database, &self.path)?;
        self.database = next_database;

        Ok(ExecutionResult::TableCreated { table: name })
    }

    fn execute_insert(
        &mut self,
        statement: InsertStatement,
    ) -> Result<ExecutionResult, BackendError> {
        let table_name = find_table_name(&self.database, &statement.table)
            .ok_or_else(|| BackendError::TableNotFound(statement.table.clone()))?
            .to_string();
        let source_table = self
            .database
            .tables
            .get(&table_name)
            .ok_or_else(|| BackendError::TableNotFound(statement.table.clone()))?;
        let row = build_row(source_table, &table_name, &statement)?;

        let mut next_database = self.database.clone();
        let table = next_database
            .tables
            .get_mut(&table_name)
            .ok_or_else(|| BackendError::TableNotFound(statement.table.clone()))?;
        insert_row(table, row)?;
        save_db(&next_database, &self.path)?;
        self.database = next_database;

        Ok(ExecutionResult::RowsInserted { count: 1 })
    }

    fn execute_select(&self, statement: SelectStatement) -> Result<ExecutionResult, BackendError> {
        let table_name = find_table_name(&self.database, &statement.table)
            .ok_or_else(|| BackendError::TableNotFound(statement.table.clone()))?;
        let table = self
            .database
            .tables
            .get(table_name)
            .ok_or_else(|| BackendError::TableNotFound(statement.table.clone()))?;

        validate_select(&statement, table, table_name)?;
        let columns = projection_columns(&statement, table);
        let mut rows = Vec::new();

        match primary_key_lookup(table, statement.where_clause.as_ref())? {
            IndexLookup::Candidate(Some(row_index)) => {
                let row = &table.rows[row_index];
                if row_matches(row, table, statement.where_clause.as_ref())? {
                    rows.push(project_row(row, table, &statement)?);
                }
            }
            IndexLookup::Candidate(None) => {}
            IndexLookup::NotApplicable => {
                for row in &table.rows {
                    if row_matches(row, table, statement.where_clause.as_ref())? {
                        rows.push(project_row(row, table, &statement)?);
                    }
                }
            }
        }

        Ok(ExecutionResult::Query(QueryResult { columns, rows }))
    }

    pub fn seed(
        &mut self,
        requested_table_name: &str,
        count: usize,
    ) -> Result<ExecutionResult, BackendError> {
        let table_name = find_table_name(&self.database, requested_table_name)
            .ok_or_else(|| BackendError::TableNotFound(requested_table_name.to_string()))?
            .to_string();

        if count == 0 {
            return Ok(ExecutionResult::RowsInserted { count: 0 });
        }

        let mut next_database = self.database.clone();
        let table = next_database
            .tables
            .get_mut(&table_name)
            .ok_or_else(|| BackendError::TableNotFound(requested_table_name.to_string()))?;

        for _ in 0..count {
            let row = next_seed_row(table)?;
            insert_row(table, row)?;
        }

        save_db(&next_database, &self.path)?;
        self.database = next_database;
        Ok(ExecutionResult::RowsInserted { count })
    }
}

fn find_table_name<'a>(database: &'a Database, requested: &str) -> Option<&'a str> {
    database
        .tables
        .keys()
        .find(|name| name.eq_ignore_ascii_case(requested))
        .map(String::as_str)
}

fn build_row(
    table: &Table,
    table_name: &str,
    statement: &InsertStatement,
) -> Result<Row, BackendError> {
    let column_indices = if statement.columns.is_empty() {
        if statement.values.len() != table.columns.len() {
            return Err(BackendError::ValueCountMismatch {
                expected: table.columns.len(),
                actual: statement.values.len(),
            });
        }
        (0..table.columns.len()).collect()
    } else {
        if statement.values.len() != statement.columns.len() {
            return Err(BackendError::ValueCountMismatch {
                expected: statement.columns.len(),
                actual: statement.values.len(),
            });
        }

        let mut seen = HashSet::new();
        let mut indices = Vec::with_capacity(statement.columns.len());
        for requested in &statement.columns {
            let index = table
                .columns
                .iter()
                .position(|column| column.name.eq_ignore_ascii_case(requested))
                .ok_or_else(|| BackendError::ColumnNotFound {
                    table: table_name.to_string(),
                    column: requested.clone(),
                })?;
            if !seen.insert(index) {
                return Err(BackendError::DuplicateColumn(requested.clone()));
            }
            indices.push(index);
        }
        indices
    };

    let mut row = vec![Value::Null; table.columns.len()];
    for (expression, column_index) in statement.values.iter().zip(column_indices) {
        let value = eval_const(expression)?;
        row[column_index] = coerce_for_column(&table.columns[column_index], value)?;
    }

    for (column, value) in table.columns.iter().zip(&row) {
        validate_value(column, value)?;
    }
    Ok(row)
}

fn coerce_for_column(column: &ColumnDefinition, value: Value) -> Result<Value, BackendError> {
    match (column.data_type, value) {
        (DataType::Float, Value::Integer(value)) => Ok(Value::Float(value as f64)),
        (_, value) => {
            validate_value(column, &value)?;
            Ok(value)
        }
    }
}

fn validate_value(column: &ColumnDefinition, value: &Value) -> Result<(), BackendError> {
    if matches!(value, Value::Null) {
        return if column.nullable && !column.primary_key {
            Ok(())
        } else {
            Err(BackendError::NullConstraintViolation(column.name.clone()))
        };
    }

    let valid = matches!(
        (column.data_type, value),
        (DataType::Integer, Value::Integer(_))
            | (DataType::Float, Value::Float(_))
            | (DataType::Boolean, Value::Boolean(_))
            | (DataType::String, Value::String(_))
    );
    if valid {
        Ok(())
    } else {
        Err(BackendError::TypeMismatch {
            column: column.name.clone(),
            expected: column.data_type,
            found: value.type_name(),
        })
    }
}

fn insert_row(table: &mut Table, row: Row) -> Result<(), BackendError> {
    let row_index = table.rows.len();
    if let Some(column_index) = table.primary_key_column {
        let value = row.get(column_index).ok_or_else(|| {
            BackendError::InvalidPrimaryKey("primary key column is outside the row".into())
        })?;
        let key = PrimaryKeyValue::try_from(value)
            .map_err(|_| BackendError::InvalidPrimaryKey("unsupported primary key value".into()))?;
        if table.primary_key_index.contains_key(&key) {
            return Err(BackendError::DuplicatePrimaryKey(format!("{key:?}")));
        }
        table.primary_key_index.insert(key, row_index);
    }

    table.rows.push(row);
    Ok(())
}

fn validate_select(
    statement: &SelectStatement,
    table: &Table,
    table_name: &str,
) -> Result<(), BackendError> {
    for item in &statement.columns {
        if let SelectItem::Expression { expr, .. } = item {
            validate_expression(expr, table, table_name)?;
        }
    }
    if let Some(expression) = &statement.where_clause {
        validate_where_expression(expression, table, table_name)?;
    }
    Ok(())
}

fn projection_columns(statement: &SelectStatement, table: &Table) -> Vec<String> {
    let mut columns = Vec::new();
    for item in &statement.columns {
        match item {
            SelectItem::Wildcard => {
                columns.extend(table.columns.iter().map(|column| column.name.clone()));
            }
            SelectItem::Expression { expr, alias } => columns.push(
                alias
                    .clone()
                    .unwrap_or_else(|| expression_label(expr).to_string()),
            ),
        }
    }
    columns
}

fn expression_label(expression: &Expr) -> &str {
    match &expression.kind {
        ExprKind::Identifier(name) => name,
        _ => "expression",
    }
}

fn project_row(
    row: &[Value],
    table: &Table,
    statement: &SelectStatement,
) -> Result<Row, BackendError> {
    let mut projected = Vec::new();
    for item in &statement.columns {
        match item {
            SelectItem::Wildcard => projected.extend(row.iter().cloned()),
            SelectItem::Expression { expr, .. } => projected.push(eval_on_row(expr, row, table)?),
        }
    }
    Ok(projected)
}

fn row_matches(
    row: &[Value],
    table: &Table,
    expression: Option<&Expr>,
) -> Result<bool, BackendError> {
    let Some(expression) = expression else {
        return Ok(true);
    };

    match eval_on_row(expression, row, table)? {
        Value::Boolean(value) => Ok(value),
        Value::Null => Ok(false),
        value => Err(BackendError::InvalidExpression(format!(
            "WHERE expression must produce BOOLEAN, found {}",
            value.type_name()
        ))),
    }
}

enum IndexLookup {
    NotApplicable,
    Candidate(Option<usize>),
}

fn primary_key_lookup(
    table: &Table,
    expression: Option<&Expr>,
) -> Result<IndexLookup, BackendError> {
    let (column_name, constant) = match expression.map(|expression| &expression.kind) {
        Some(ExprKind::Binary {
            left,
            op: BinaryOp::Equal,
            right,
        }) => match (&left.kind, &right.kind) {
            (ExprKind::Identifier(name), ExprKind::Literal(_)) => (name, right.as_ref()),
            (ExprKind::Literal(_), ExprKind::Identifier(name)) => (name, left.as_ref()),
            _ => return Ok(IndexLookup::NotApplicable),
        },
        _ => return Ok(IndexLookup::NotApplicable),
    };

    let Some(column_index) = table.primary_key_column else {
        return Ok(IndexLookup::NotApplicable);
    };
    let column = &table.columns[column_index];
    if !column.name.eq_ignore_ascii_case(column_name) {
        return Ok(IndexLookup::NotApplicable);
    }

    let value = eval_const(constant)?;
    let exact_type = matches!(
        (column.data_type, &value),
        (DataType::Integer, Value::Integer(_))
            | (DataType::String, Value::String(_))
            | (DataType::Boolean, Value::Boolean(_))
    );
    if !exact_type {
        return Ok(IndexLookup::NotApplicable);
    }

    let key = PrimaryKeyValue::try_from(&value)
        .map_err(|_| BackendError::InvalidPrimaryKey("unsupported primary key value".into()))?;
    Ok(IndexLookup::Candidate(
        table.primary_key_index.get(&key).copied(),
    ))
}

fn next_seed_row(table: &Table) -> Result<Row, BackendError> {
    let mut ordinal = table.rows.len();
    let max_attempts = if table
        .primary_key_column
        .is_some_and(|index| table.columns[index].data_type == DataType::Boolean)
    {
        2
    } else {
        usize::MAX
    };

    for _ in 0..max_attempts {
        let row = table
            .columns
            .iter()
            .map(|column| seed_value(column.data_type, ordinal))
            .collect::<Result<Row, BackendError>>()?;
        let available = table.primary_key_column.is_none_or(|index| {
            PrimaryKeyValue::try_from(&row[index])
                .ok()
                .is_some_and(|key| !table.primary_key_index.contains_key(&key))
        });
        if available {
            return Ok(row);
        }
        ordinal = ordinal
            .checked_add(1)
            .ok_or_else(|| BackendError::Execution("seed counter overflow".into()))?;
    }

    Err(BackendError::InvalidPrimaryKey(
        "no unused BOOLEAN primary key remains".into(),
    ))
}

fn seed_value(data_type: DataType, ordinal: usize) -> Result<Value, BackendError> {
    let integer = i64::try_from(ordinal)
        .map_err(|_| BackendError::Execution("seed value exceeds INTEGER range".into()))?;
    Ok(match data_type {
        DataType::Integer => Value::Integer(integer),
        DataType::Float => Value::Float(integer as f64),
        DataType::Boolean => Value::Boolean(ordinal & 1 == 0),
        DataType::String => Value::String(format!("value{ordinal}")),
    })
}
