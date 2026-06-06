use serde::{Serialize, Deserialize};

use std::collections::HashMap;

use interface::{
    ColumnDefinition,
 Value,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<ColumnDefinition>,
    pub rows: Vec<Row>,
}

pub type Row = Vec<Value>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Database {
    pub tables: HashMap<String, Table>,
}