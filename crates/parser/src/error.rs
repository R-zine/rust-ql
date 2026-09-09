use std::{error::Error, fmt};

use interface::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },
    UnexpectedEof,
    Message {
        message: String,
        span: Span,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedToken {
                expected,
                found,
                span,
            } => write!(
                formatter,
                "expected {expected}, found {found} at bytes {}..{}",
                span.start, span.end
            ),
            Self::UnexpectedEof => formatter.write_str("unexpected end of input"),
            Self::Message { message, span } => {
                write!(formatter, "{message} at bytes {}..{}", span.start, span.end)
            }
        }
    }
}

impl Error for ParseError {}
