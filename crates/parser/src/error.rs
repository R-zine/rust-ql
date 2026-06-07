use interface::Span;

#[derive(Debug, Clone)]
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
