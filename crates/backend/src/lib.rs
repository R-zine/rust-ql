mod database;
mod error;
mod evaluation;
mod executor;
mod persistence;

use std::path::PathBuf;

use lexer::Lexer;
use parser::Parser;

pub use crate::{
    error::BackendError,
    executor::{ExecutionResult, Executor, QueryResult},
};

pub use database::{Database, Row, Table};

pub struct Engine {
    executor: Executor,
}

impl Engine {
    pub fn new() -> Result<Self, BackendError> {
        Self::with_path("db.json")
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Result<Self, BackendError> {
        Ok(Self {
            executor: Executor::new(path)?,
        })
    }

    pub fn execute(&mut self, sql: &str) -> Result<ExecutionResult, BackendError> {
        let tokens = Lexer::new(sql).tokenize()?;
        let statement = Parser::new(tokens).parse()?;
        self.executor.execute(statement)
    }

    pub fn seed(
        &mut self,
        table_name: &str,
        count: usize,
    ) -> Result<ExecutionResult, BackendError> {
        self.executor.seed(table_name, count)
    }
}
