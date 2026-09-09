use std::collections::{HashMap, HashSet};

use interface::{ColumnDefinition, DataType, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimaryKeyValue {
    Integer(i64),
    String(String),
    Boolean(bool),
}

impl TryFrom<&Value> for PrimaryKeyValue {
    type Error = ();

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(value) => Ok(Self::Integer(*value)),
            Value::String(value) => Ok(Self::String(value.clone())),
            Value::Boolean(value) => Ok(Self::Boolean(*value)),
            Value::Float(_) | Value::Null => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<ColumnDefinition>,
    pub rows: Vec<Row>,

    #[serde(skip)]
    pub(crate) primary_key_column: Option<usize>,

    #[serde(skip)]
    pub(crate) primary_key_index: HashMap<PrimaryKeyValue, usize>,
}

impl Table {
    pub(crate) fn new(mut columns: Vec<ColumnDefinition>) -> Result<Self, String> {
        for column in &mut columns {
            if column.primary_key {
                column.nullable = false;
            }
        }

        let mut table = Self {
            columns,
            rows: Vec::new(),
            primary_key_column: None,
            primary_key_index: HashMap::new(),
        };
        table.validate_and_rebuild()?;
        Ok(table)
    }

    pub(crate) fn validate_and_rebuild(&mut self) -> Result<(), String> {
        if self.columns.is_empty() {
            return Err("a table must contain at least one column".into());
        }

        let mut names = HashSet::new();
        let mut primary_key = None;

        for (index, column) in self.columns.iter_mut().enumerate() {
            if !names.insert(column.name.to_ascii_lowercase()) {
                return Err(format!("duplicate column '{}'", column.name));
            }

            if column.primary_key {
                if primary_key.replace(index).is_some() {
                    return Err("a table may have only one primary key".into());
                }
                if column.data_type == DataType::Float {
                    return Err("FLOAT primary keys are not supported".into());
                }
                column.nullable = false;
            }
        }

        self.primary_key_column = primary_key;
        self.primary_key_index.clear();

        for (row_index, row) in self.rows.iter().enumerate() {
            if row.len() != self.columns.len() {
                return Err(format!(
                    "row {row_index} has {} values but the table has {} columns",
                    row.len(),
                    self.columns.len()
                ));
            }

            for (column, value) in self.columns.iter().zip(row) {
                validate_stored_value(column, value).map_err(|message| {
                    format!(
                        "invalid value in row {row_index}, column '{}': {message}",
                        column.name
                    )
                })?;
            }

            if let Some(column_index) = self.primary_key_column {
                let value = PrimaryKeyValue::try_from(&row[column_index])
                    .map_err(|_| format!("invalid primary key in row {row_index}"))?;
                if self.primary_key_index.insert(value, row_index).is_some() {
                    return Err(format!("duplicate primary key in row {row_index}"));
                }
            }
        }

        Ok(())
    }
}

fn validate_stored_value(column: &ColumnDefinition, value: &Value) -> Result<(), String> {
    if matches!(value, Value::Null) {
        return if column.nullable {
            Ok(())
        } else {
            Err("NULL violates a NOT NULL constraint".into())
        };
    }

    let matches_type = matches!(
        (column.data_type, value),
        (DataType::Integer, Value::Integer(_))
            | (DataType::Float, Value::Float(_))
            | (DataType::Boolean, Value::Boolean(_))
            | (DataType::String, Value::String(_))
    );

    if matches_type {
        Ok(())
    } else {
        Err(format!(
            "expected {}, found {}",
            column.data_type,
            value.type_name()
        ))
    }
}

pub type Row = Vec<Value>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Database {
    pub tables: HashMap<String, Table>,
}

impl Database {
    pub(crate) fn validate_and_rebuild(&mut self) -> Result<(), String> {
        let mut names = HashSet::new();
        for name in self.tables.keys() {
            if !names.insert(name.to_ascii_lowercase()) {
                return Err(format!("duplicate table name '{name}'"));
            }
        }

        for (name, table) in &mut self.tables {
            table
                .validate_and_rebuild()
                .map_err(|message| format!("invalid table '{name}': {message}"))?;
        }
        Ok(())
    }
}
