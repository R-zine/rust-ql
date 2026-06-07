#[derive(Debug)]
pub enum BackendError {
    Lex(lexer::LexError),
    Parse(parser::ParseError),

    TableAlreadyExists(String),
    TableNotFound(String),

    Execution(String),
    InvalidPrimaryKey(String),
    DuplicatePrimaryKey(String),
}

impl From<lexer::LexError> for BackendError {
    fn from(err: lexer::LexError) -> Self {
        Self::Lex(err)
    }
}

impl From<parser::ParseError> for BackendError {
    fn from(err: parser::ParseError) -> Self {
        Self::Parse(err)
    }
}
