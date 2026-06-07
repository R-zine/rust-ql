use interface::{Keyword, Span, SpannedToken, Token};

pub fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

pub fn is_identifier_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

pub fn parse_identifier_or_keyword(text: &str, start: usize, end: usize) -> SpannedToken {
    let value = &text[start..end];

    let token = match value.to_ascii_uppercase().as_str() {
        "SELECT" => Token::Keyword(Keyword::Select),
        "FROM" => Token::Keyword(Keyword::From),
        "WHERE" => Token::Keyword(Keyword::Where),

        "INSERT" => Token::Keyword(Keyword::Insert),
        "INTO" => Token::Keyword(Keyword::Into),
        "VALUES" => Token::Keyword(Keyword::Values),

        "CREATE" => Token::Keyword(Keyword::Create),
        "TABLE" => Token::Keyword(Keyword::Table),

        "PRIMARY" => Token::Keyword(Keyword::Primary),
        "KEY" => Token::Keyword(Keyword::Key),

        "TRUE" => Token::Boolean(true),
        "FALSE" => Token::Boolean(false),
        "NULL" => Token::Null,

        _ => Token::Identifier(value.to_string()),
    };

    SpannedToken {
        token,
        span: Span::new(start, end),
    }
}

pub fn parse_number(text: &str, start: usize, end: usize) -> SpannedToken {
    let value = &text[start..end];

    let token = if value.contains('.') {
        Token::Float(value.parse().unwrap())
    } else {
        Token::Integer(value.parse().unwrap())
    };

    SpannedToken {
        token,
        span: Span::new(start, end),
    }
}

pub fn parse_string(value: String, start: usize, end: usize) -> SpannedToken {
    SpannedToken {
        token: Token::String(value),
        span: Span::new(start, end),
    }
}
