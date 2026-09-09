use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use backend::{BackendError, Engine, ExecutionResult, Executor, QueryResult};
use interface::{
    ColumnDefinition, CreateTableStatement, DataType, Span, Statement, StatementKind, Value,
};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestDatabase {
    directory: PathBuf,
    path: PathBuf,
}

impl TestDatabase {
    fn new() -> Self {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("rustql-backend-test-{}-{id}", std::process::id()));
        fs::create_dir(&directory).expect("test directory should be unique");
        let path = directory.join("database.json");
        Self { directory, path }
    }

    fn engine(&self) -> Engine {
        Engine::with_path(&self.path).expect("test database should open")
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).expect("test directory should be removable");
    }
}

fn query(engine: &mut Engine, sql: &str) -> QueryResult {
    match engine.execute(sql).expect("query should succeed") {
        ExecutionResult::Query(result) => result,
        result => panic!("expected query result, received {result:?}"),
    }
}

fn assert_no_rows(engine: &mut Engine, table: &str) {
    assert!(
        query(engine, &format!("SELECT * FROM {table}"))
            .rows
            .is_empty()
    );
}

#[test]
fn readme_workflow_honors_column_order_and_survives_reload() {
    let database = TestDatabase::new();
    let mut engine = database.engine();

    engine
        .execute("CREATE TABLE Users (id INTEGER PRIMARY KEY, name STRING NOT NULL)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (1, 'Phil')")
        .unwrap();
    engine
        .execute("INSERT INTO USERS (name, id) VALUES ('Alice', 2)")
        .unwrap();

    let result = query(&mut engine, "SELECT NAME, ID FROM users WHERE id >= 1");
    assert_eq!(result.columns, ["NAME", "ID"]);
    assert_eq!(
        result.rows,
        [
            vec![Value::String("Phil".into()), Value::Integer(1)],
            vec![Value::String("Alice".into()), Value::Integer(2)],
        ]
    );

    drop(engine);
    let mut reloaded = database.engine();
    assert_eq!(
        query(&mut reloaded, "SELECT * FROM USERS WHERE ID = 2").rows,
        [vec![Value::Integer(2), Value::String("Alice".into())]]
    );

    let persisted = fs::read_to_string(&database.path).unwrap();
    assert!(!persisted.contains("primary_key_index"));
    assert!(!persisted.contains("primary_key_column"));
    assert!(!fs::read_dir(&database.directory).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
}

#[test]
fn insert_validates_columns_arity_types_and_nullability() {
    let database = TestDatabase::new();
    let mut engine = database.engine();
    engine
        .execute(
            "CREATE TABLE items (id INTEGER PRIMARY KEY, score FLOAT, active BOOLEAN NOT NULL, note STRING)",
        )
        .unwrap();

    assert!(matches!(
        engine.execute("INSERT INTO items (missing) VALUES (1)"),
        Err(BackendError::ColumnNotFound { .. })
    ));
    assert!(matches!(
        engine.execute("INSERT INTO items (id, ID) VALUES (1, 2)"),
        Err(BackendError::DuplicateColumn(_))
    ));
    assert!(matches!(
        engine.execute("INSERT INTO items (id, active) VALUES (1)"),
        Err(BackendError::ValueCountMismatch { .. })
    ));
    assert!(matches!(
        engine.execute("INSERT INTO items VALUES (1, 2.0, TRUE)"),
        Err(BackendError::ValueCountMismatch { .. })
    ));
    assert!(matches!(
        engine.execute("INSERT INTO items (id, active) VALUES ('wrong', TRUE)"),
        Err(BackendError::TypeMismatch { .. })
    ));
    assert!(matches!(
        engine.execute("INSERT INTO items (id) VALUES (1)"),
        Err(BackendError::NullConstraintViolation(column)) if column == "active"
    ));
    assert!(matches!(
        engine.execute("INSERT INTO items (id, active) VALUES (other_id, TRUE)"),
        Err(BackendError::InvalidExpression(_))
    ));
    assert_no_rows(&mut engine, "items");

    engine
        .execute("INSERT INTO items (id, score, active) VALUES (1, 2, TRUE)")
        .unwrap();
    assert_eq!(
        query(&mut engine, "SELECT * FROM items").rows,
        [vec![
            Value::Integer(1),
            Value::Float(2.0),
            Value::Boolean(true),
            Value::Null,
        ]]
    );
}

#[test]
fn primary_key_constraints_are_enforced() {
    let database = TestDatabase::new();
    let mut engine = database.engine();
    engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name STRING)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (1, 'first')")
        .unwrap();

    assert!(matches!(
        engine.execute("INSERT INTO users VALUES (1, 'duplicate')"),
        Err(BackendError::DuplicatePrimaryKey(_))
    ));
    assert!(matches!(
        engine.execute("INSERT INTO users VALUES (NULL, 'missing')"),
        Err(BackendError::NullConstraintViolation(_))
    ));
    assert_eq!(query(&mut engine, "SELECT * FROM users").rows.len(), 1);
    assert_eq!(
        query(&mut engine, "SELECT * FROM users WHERE id = 1.0")
            .rows
            .len(),
        1
    );
    assert!(matches!(
        engine.execute("CREATE TABLE USERS (other INTEGER)"),
        Err(BackendError::TableAlreadyExists(_))
    ));

    assert!(
        engine
            .execute("CREATE TABLE multiple (a INTEGER PRIMARY KEY, b INTEGER PRIMARY KEY)")
            .is_err()
    );
    assert!(
        engine
            .execute("CREATE TABLE float_key (id FLOAT PRIMARY KEY)")
            .is_err()
    );
}

#[test]
fn evaluates_literals_arithmetic_logic_aliases_and_nulls() {
    let database = TestDatabase::new();
    let mut engine = database.engine();
    engine
        .execute(
            "CREATE TABLE values_table (id INTEGER PRIMARY KEY, score FLOAT, enabled BOOLEAN, note STRING)",
        )
        .unwrap();
    engine
        .execute("INSERT INTO values_table VALUES (-2, 1.5, TRUE, NULL)")
        .unwrap();

    let result = query(
        &mut engine,
        "SELECT id + 2 * 3 AS total, score >= .5 AS good, NOT enabled AS disabled FROM values_table WHERE id = -2 AND enabled = TRUE",
    );
    assert_eq!(result.columns, ["total", "good", "disabled"]);
    assert_eq!(
        result.rows,
        [vec![
            Value::Integer(4),
            Value::Boolean(true),
            Value::Boolean(false),
        ]]
    );

    assert!(
        query(&mut engine, "SELECT * FROM values_table WHERE note = NULL")
            .rows
            .is_empty()
    );
    assert!(matches!(
        engine.execute("SELECT * FROM values_table WHERE id / 0 = 1"),
        Err(BackendError::Execution(_))
    ));
    assert!(matches!(
        engine.execute("SELECT * FROM values_table WHERE id"),
        Err(BackendError::InvalidExpression(_))
    ));
}

#[test]
fn rejects_unknown_columns_trailing_input_and_numeric_overflow() {
    let database = TestDatabase::new();
    let mut engine = database.engine();
    engine
        .execute("CREATE TABLE empty_table (id INTEGER)")
        .unwrap();

    assert!(matches!(
        engine.execute("SELECT missing FROM empty_table"),
        Err(BackendError::ColumnNotFound { .. })
    ));
    assert!(matches!(
        engine.execute("SELECT 1 + 'wrong' FROM empty_table"),
        Err(BackendError::InvalidExpression(_))
    ));
    assert!(matches!(
        engine.execute("SELECT * FROM empty_table WHERE id"),
        Err(BackendError::InvalidExpression(_))
    ));
    assert!(matches!(
        engine.execute("SELECT * FROM empty_table trailing"),
        Err(BackendError::Parse(_))
    ));
    assert!(matches!(
        engine.execute("SELECT * FROM empty_table WHERE id = 999999999999999999999999999999"),
        Err(BackendError::Lex(_))
    ));
}

#[test]
fn corrupt_database_is_reported_and_left_untouched() {
    let database = TestDatabase::new();
    let corrupt = b"{ definitely not valid JSON";
    fs::write(&database.path, corrupt).unwrap();

    assert!(matches!(
        Engine::with_path(&database.path),
        Err(BackendError::SerializationError(_))
    ));
    assert_eq!(fs::read(&database.path).unwrap(), corrupt);
}

#[test]
fn malformed_persisted_rows_are_rejected_without_panicking() {
    let database = TestDatabase::new();
    fs::write(
        &database.path,
        r#"{
            "tables": {
                "users": {
                    "columns": [{
                        "name": "id",
                        "data_type": "Integer",
                        "nullable": false,
                        "primary_key": true
                    }],
                    "rows": [[]]
                }
            }
        }"#,
    )
    .unwrap();

    assert!(matches!(
        Engine::with_path(&database.path),
        Err(BackendError::InvalidSchema(_))
    ));
}

#[test]
fn save_failure_does_not_mutate_the_live_database() {
    let database = TestDatabase::new();
    let mut engine = database.engine();
    engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY)")
        .unwrap();
    fs::remove_file(&database.path).unwrap();
    fs::create_dir(&database.path).unwrap();

    assert!(matches!(
        engine.execute("INSERT INTO users VALUES (1)"),
        Err(BackendError::IoError(_))
    ));
    assert_no_rows(&mut engine, "users");

    assert!(matches!(
        engine.execute("CREATE TABLE other (id INTEGER PRIMARY KEY)"),
        Err(BackendError::IoError(_))
    ));
    assert!(matches!(
        engine.execute("SELECT * FROM other"),
        Err(BackendError::TableNotFound(_))
    ));
}

#[test]
fn executor_validates_ast_callers_that_bypass_the_parser() {
    let database = TestDatabase::new();
    let mut executor = Executor::new(&database.path).unwrap();
    let columns = vec![
        ColumnDefinition {
            name: "id".into(),
            data_type: DataType::Integer,
            nullable: true,
            primary_key: true,
        },
        ColumnDefinition {
            name: "ID".into(),
            data_type: DataType::String,
            nullable: true,
            primary_key: false,
        },
    ];
    let statement = Statement::new(
        StatementKind::CreateTable(CreateTableStatement {
            name: "bad".into(),
            columns,
        }),
        Span::default(),
    );

    assert!(matches!(
        executor.execute(statement),
        Err(BackendError::InvalidSchema(_))
    ));
}

#[test]
fn schema_aware_seed_is_atomic_when_key_space_is_exhausted() {
    let database = TestDatabase::new();
    let mut engine = database.engine();
    engine
        .execute("CREATE TABLE flags (enabled BOOLEAN PRIMARY KEY, score FLOAT, label STRING)")
        .unwrap();
    engine.seed("FLAGS", 2).unwrap();

    assert!(matches!(
        engine.seed("flags", 1),
        Err(BackendError::InvalidPrimaryKey(_))
    ));
    assert_eq!(query(&mut engine, "SELECT * FROM flags").rows.len(), 2);
}

#[test]
fn database_path_can_be_borrowed() {
    fn open(path: &Path) -> Result<Engine, BackendError> {
        Engine::with_path(path)
    }

    let database = TestDatabase::new();
    assert!(open(&database.path).is_ok());
}
