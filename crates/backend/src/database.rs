use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use interface::{ColumnDefinition, Value};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimaryKeyValue {
    Integer(i64),
    String(String),
    Boolean(bool),
}

impl TryFrom<&Value> for PrimaryKeyValue {
    type Error = ();

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(i) => Ok(Self::Integer(*i)),
            Value::String(s) => Ok(Self::String(s.clone())),
            Value::Boolean(b) => Ok(Self::Boolean(*b)),

            Value::Float(_) | Value::Null => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<ColumnDefinition>,
    pub rows: Vec<Row>,

    pub primary_key_column: Option<usize>,

    #[serde(skip)]
    pub primary_key_index: HashMap<PrimaryKeyValue, usize>,
}

impl Table {
    pub fn rebuild_index(&mut self) {
        self.primary_key_index.clear();

        let Some(pk_col) = self.primary_key_column else {
            return;
        };

        for (i, row) in self.rows.iter().enumerate() {
            if let Ok(pk) = PrimaryKeyValue::try_from(&row[pk_col]) {
                self.primary_key_index.insert(pk, i);
            }
        }
    }
}

pub type Row = Vec<Value>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Database {
    pub tables: HashMap<String, Table>,
}
