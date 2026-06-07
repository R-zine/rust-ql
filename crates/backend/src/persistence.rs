use std::{fs, path::Path};

use crate::database::Database;
use crate::error::BackendError;

pub fn save_db(db: &Database, path: &str) -> Result<(), BackendError> {
    let json =
        serde_json::to_string_pretty(db).map_err(|e| BackendError::Execution(e.to_string()))?;

    fs::write(path, json).map_err(|e| BackendError::Execution(e.to_string()))?;

    Ok(())
}

pub fn load_db(path: &str) -> Result<Database, BackendError> {
    if !Path::new(path).exists() {
        return Ok(Database::default());
    }

    let file = std::fs::read_to_string(path).map_err(|e| BackendError::IoError(e.to_string()))?;

    let mut database: Database =
        serde_json::from_str(&file).map_err(|e| BackendError::SerializationError(e.to_string()))?;

    for table in database.tables.values_mut() {
        table.rebuild_index();
    }

    Ok(database)
}
