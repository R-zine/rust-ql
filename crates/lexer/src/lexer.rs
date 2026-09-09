use std::{error::Error, fmt};

use interface::{Span, SpannedToken, Token};

use crate::helpers::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    UnexpectedCharacter { character: char, position: usize },
    UnterminatedString { start: usize },
    InvalidNumber { value: String, span: Span },
}

impl fmt::Display for LexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter {
                character,
                position,
            } => write!(
                formatter,
                "unexpected character '{character}' at byte {position}"
            ),
            Self::UnterminatedString { start } => {
                write!(formatter, "unterminated string starting at byte {start}")
            }
            Self::InvalidNumber { value, span } => write!(
                formatter,
                "invalid numeric literal '{value}' at bytes {}..{}",
                span.start, span.end
            ),
        }
    }
}

impl Error for LexError {}

pub struct Lexer<'a> {
    source: &'a str,
    // Byte offset into `source`. Keeping byte offsets makes every Span safe to
    // use for slicing, including after non-ASCII whitespace or string content.
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    fn current(&self) -> Option<char> {
        self.source.get(self.position..)?.chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut chars = self.source.get(self.position..)?.chars();
        chars.next()?;
        chars.next()
    }

    fn advance(&mut self) {
        if let Some(character) = self.current() {
            self.position += character.len_utf8();
        }
    }

    fn lex_simple(&mut self, token: Token) -> SpannedToken {
        let start = self.position;
        self.advance();
        SpannedToken::new(token, Span::new(start, self.position))
    }

    fn lex_double(&mut self, token: Token) -> SpannedToken {
        let start = self.position;
        self.advance();
        self.advance();
        SpannedToken::new(token, Span::new(start, self.position))
    }

    pub fn tokenize(mut self) -> Result<Vec<SpannedToken>, LexError> {
        let mut tokens = Vec::new();

        while let Some(character) = self.current() {
            if character.is_whitespace() {
                self.advance();
                continue;
            }

            if is_identifier_start(character) {
                tokens.push(self.lex_identifier());
                continue;
            }

            if character.is_ascii_digit()
                || (character == '.' && self.peek_next().is_some_and(|c| c.is_ascii_digit()))
            {
                tokens.push(self.lex_number()?);
                continue;
            }

            let token = match character {
                '\'' => self.lex_string()?,
                ',' => self.lex_simple(Token::Comma),
                ';' => self.lex_simple(Token::Semicolon),
                '(' => self.lex_simple(Token::LParen),
                ')' => self.lex_simple(Token::RParen),
                '+' => self.lex_simple(Token::Plus),
                '-' => self.lex_simple(Token::Minus),
                '*' => self.lex_simple(Token::Star),
                '/' => self.lex_simple(Token::Slash),
                '%' => self.lex_simple(Token::Percent),
                '=' => self.lex_simple(Token::Equal),
                '>' if self.peek_next() == Some('=') => self.lex_double(Token::GreaterThanOrEqual),
                '>' => self.lex_simple(Token::GreaterThan),
                '<' if self.peek_next() == Some('=') => self.lex_double(Token::LessThanOrEqual),
                '<' if self.peek_next() == Some('>') => self.lex_double(Token::NotEqual),
                '<' => self.lex_simple(Token::LessThan),
                '!' if self.peek_next() == Some('=') => self.lex_double(Token::NotEqual),
                _ => {
                    return Err(LexError::UnexpectedCharacter {
                        character,
                        position: self.position,
                    });
                }
            };

            tokens.push(token);
        }

        tokens.push(SpannedToken::new(
            Token::Eof,
            Span::new(self.position, self.position),
        ));
        Ok(tokens)
    }

    fn lex_identifier(&mut self) -> SpannedToken {
        let start = self.position;

        while self.current().is_some_and(is_identifier_part) {
            self.advance();
        }

        parse_identifier_or_keyword(self.source, start, self.position)
    }

    fn lex_number(&mut self) -> Result<SpannedToken, LexError> {
        let start = self.position;

        while self.current().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }

        if self.current() == Some('.') {
            self.advance();
            while self.current().is_some_and(|c| c.is_ascii_digit()) {
                self.advance();
            }
        }

        if self.current().is_some_and(|c| matches!(c, 'e' | 'E')) {
            self.advance();
            if self.current().is_some_and(|c| matches!(c, '+' | '-')) {
                self.advance();
            }

            let exponent_start = self.position;
            while self.current().is_some_and(|c| c.is_ascii_digit()) {
                self.advance();
            }

            if exponent_start == self.position {
                return Err(invalid_number(self.source, start, self.position));
            }
        }

        parse_number(self.source, start, self.position)
    }

    fn lex_string(&mut self) -> Result<SpannedToken, LexError> {
        let start = self.position;
        self.advance();
        let mut value = String::new();

        while let Some(character) = self.current() {
            if character == '\'' {
                if self.peek_next() == Some('\'') {
                    value.push('\'');
                    self.advance();
                    self.advance();
                    continue;
                }

                self.advance();
                return Ok(parse_string(value, start, self.position));
            }

            value.push(character);
            self.advance();
        }

        Err(LexError::UnterminatedString { start })
    }
}
