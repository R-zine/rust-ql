mod database;
mod error;
mod executor;
mod persistence;

use lexer::Lexer;
use parser::Parser;

pub use crate::{
    error::BackendError,
    executor::Executor,
};

pub use database::{
    Database,
    Table,
};


pub struct Engine {
    executor: Executor,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            executor: Executor::new("db.json"),
        }
    }


    pub fn execute(
        &mut self,
        sql: &str,
    ) -> Result<(), BackendError> {

        let tokens =
            Lexer::new(sql).tokenize()?;

        let statement =
            Parser::new(tokens).parse()?;

        self.executor.execute(statement)
    }
}