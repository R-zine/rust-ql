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

    let content = fs::read_to_string(path).map_err(|e| BackendError::Execution(e.to_string()))?;

    let db = serde_json::from_str(&content).map_err(|e| BackendError::Execution(e.to_string()))?;

    Ok(db)
}
