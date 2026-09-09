use std::{error::Error, fmt};

use interface::DataType;

#[derive(Debug)]
pub enum BackendError {
    Lex(lexer::LexError),
    Parse(parser::ParseError),
    TableAlreadyExists(String),
    TableNotFound(String),
    ColumnNotFound {
        table: String,
        column: String,
    },
    DuplicateColumn(String),
    ValueCountMismatch {
        expected: usize,
        actual: usize,
    },
    TypeMismatch {
        column: String,
        expected: DataType,
        found: &'static str,
    },
    NullConstraintViolation(String),
    InvalidExpression(String),
    InvalidSchema(String),
    InvalidPrimaryKey(String),
    DuplicatePrimaryKey(String),
    Execution(String),
    IoError(String),
    SerializationError(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lex(error) => write!(formatter, "lexer error: {error}"),
            Self::Parse(error) => write!(formatter, "parser error: {error}"),
            Self::TableAlreadyExists(table) => write!(formatter, "table '{table}' already exists"),
            Self::TableNotFound(table) => write!(formatter, "table '{table}' does not exist"),
            Self::ColumnNotFound { table, column } => {
                write!(
                    formatter,
                    "column '{column}' does not exist in table '{table}'"
                )
            }
            Self::DuplicateColumn(column) => {
                write!(formatter, "column '{column}' appears more than once")
            }
            Self::ValueCountMismatch { expected, actual } => {
                write!(formatter, "expected {expected} values, received {actual}")
            }
            Self::TypeMismatch {
                column,
                expected,
                found,
            } => write!(
                formatter,
                "column '{column}' expects {expected}, received {found}"
            ),
            Self::NullConstraintViolation(column) => {
                write!(formatter, "column '{column}' may not be NULL")
            }
            Self::InvalidExpression(message)
            | Self::InvalidSchema(message)
            | Self::InvalidPrimaryKey(message)
            | Self::Execution(message)
            | Self::IoError(message)
            | Self::SerializationError(message) => formatter.write_str(message),
            Self::DuplicatePrimaryKey(value) => {
                write!(formatter, "duplicate primary key {value}")
            }
        }
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Lex(error) => Some(error),
            Self::Parse(error) => Some(error),
            _ => None,
        }
    }
}

impl From<lexer::LexError> for BackendError {
    fn from(error: lexer::LexError) -> Self {
        Self::Lex(error)
    }
}

impl From<parser::ParseError> for BackendError {
    fn from(error: parser::ParseError) -> Self {
        Self::Parse(error)
    }
}
