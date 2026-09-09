use crate::keyword::Keyword;
use crate::span::Span;

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Keyword(Keyword),

    // Identifiers
    Identifier(String),

    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,

    // Punctuation
    Comma,
    Semicolon,

    LParen,
    RParen,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    Equal,
    NotEqual,

    LessThan,
    LessThanOrEqual,

    GreaterThan,
    GreaterThanOrEqual,

    // Logical operators
    And,
    Or,
    Not,

    // End of input
    Eof,
}

/// A token together with its source location.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

impl SpannedToken {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

impl Token {
    /// Returns true if this token is a literal value.
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            Token::Integer(_)
                | Token::Float(_)
                | Token::String(_)
                | Token::Boolean(_)
                | Token::Null
        )
    }

    /// Returns true if this token is an identifier.
    pub fn is_identifier(&self) -> bool {
        matches!(self, Token::Identifier(_))
    }

    /// Returns true if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(self, Token::Keyword(_))
    }
}
