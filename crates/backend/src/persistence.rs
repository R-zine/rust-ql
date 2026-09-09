use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{database::Database, error::BackendError};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn save_db(database: &Database, path: &Path) -> Result<(), BackendError> {
    let temporary_path = temporary_path(path);
    let result = write_and_replace(database, path, &temporary_path);

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    result
}

fn write_and_replace(
    database: &Database,
    path: &Path,
    temporary_path: &Path,
) -> Result<(), BackendError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary_path)
        .map_err(|error| io_error("create temporary database", temporary_path, error))?;

    {
        let mut writer = BufWriter::new(&mut file);
        serde_json::to_writer_pretty(&mut writer, database).map_err(|error| {
            BackendError::SerializationError(format!("failed to serialize database: {error}"))
        })?;
        writer
            .flush()
            .map_err(|error| io_error("flush temporary database", temporary_path, error))?;
    }

    file.sync_all()
        .map_err(|error| io_error("sync temporary database", temporary_path, error))?;
    drop(file);

    fs::rename(temporary_path, path).map_err(|error| io_error("replace database", path, error))?;
    Ok(())
}

pub fn load_db(path: &Path) -> Result<Database, BackendError> {
    if !path.exists() {
        return Ok(Database::default());
    }

    let file = File::open(path).map_err(|error| io_error("open database", path, error))?;
    let mut database: Database = serde_json::from_reader(file).map_err(|error| {
        BackendError::SerializationError(format!(
            "failed to deserialize database '{}': {error}",
            path.display()
        ))
    })?;

    database.validate_and_rebuild().map_err(|message| {
        BackendError::InvalidSchema(format!(
            "database '{}' is invalid: {message}",
            path.display()
        ))
    })?;
    Ok(database)
}

fn temporary_path(path: &Path) -> PathBuf {
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let process = std::process::id();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("database");
    path.with_file_name(format!(".{file_name}.{process}.{counter}.tmp"))
}

fn io_error(operation: &str, path: &Path, error: std::io::Error) -> BackendError {
    BackendError::IoError(format!(
        "failed to {operation} '{}': {error}",
        path.display()
    ))
}
