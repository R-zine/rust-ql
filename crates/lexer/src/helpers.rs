use interface::{Keyword, Span, SpannedToken, Token};

use crate::lexer::LexError;

pub fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

pub fn is_identifier_part(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
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
        "AS" => Token::Keyword(Keyword::As),
        "PRIMARY" => Token::Keyword(Keyword::Primary),
        "KEY" => Token::Keyword(Keyword::Key),
        "AND" => Token::And,
        "OR" => Token::Or,
        "NOT" => Token::Not,
        "TRUE" => Token::Boolean(true),
        "FALSE" => Token::Boolean(false),
        "NULL" => Token::Null,
        _ => Token::Identifier(value.to_string()),
    };

    SpannedToken::new(token, Span::new(start, end))
}

pub fn invalid_number(text: &str, start: usize, end: usize) -> LexError {
    LexError::InvalidNumber {
        value: text[start..end].to_string(),
        span: Span::new(start, end),
    }
}

pub fn parse_number(text: &str, start: usize, end: usize) -> Result<SpannedToken, LexError> {
    let value = &text[start..end];
    let span = Span::new(start, end);

    let token = if value.contains(['.', 'e', 'E']) {
        let number = value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .ok_or_else(|| invalid_number(text, start, end))?;
        Token::Float(number)
    } else {
        Token::Integer(
            value
                .parse()
                .map_err(|_| invalid_number(text, start, end))?,
        )
    };

    Ok(SpannedToken::new(token, span))
}

pub fn parse_string(value: String, start: usize, end: usize) -> SpannedToken {
    SpannedToken::new(Token::String(value), Span::new(start, end))
}
