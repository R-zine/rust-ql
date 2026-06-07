use interface::{Span, SpannedToken, Token};

use crate::helpers::*;

#[derive(Debug)]
pub enum LexError {
    UnexpectedCharacter { character: char, position: usize },

    UnterminatedString { start: usize },
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            position: 0,
        }
    }

    fn current(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn simple(&self, token: Token) -> SpannedToken {
        SpannedToken {
            token,
            span: Span::new(self.position, self.position + 1),
        }
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.position + 1).copied()
    }

    pub fn tokenize(mut self) -> Result<Vec<SpannedToken>, LexError> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                self.advance();
                continue;
            }

            if is_identifier_start(ch) {
                tokens.push(self.lex_identifier());
                continue;
            }

            if ch.is_ascii_digit() {
                tokens.push(self.lex_number());
                continue;
            }

            match ch {
                '\'' => {
                    tokens.push(self.lex_string()?);
                }

                ',' => {
                    tokens.push(self.simple(Token::Comma));
                    self.advance();
                }

                ';' => {
                    tokens.push(self.simple(Token::Semicolon));
                    self.advance();
                }

                '(' => {
                    tokens.push(self.simple(Token::LParen));
                    self.advance();
                }

                ')' => {
                    tokens.push(self.simple(Token::RParen));
                    self.advance();
                }

                '*' => {
                    tokens.push(self.simple(Token::Star));
                    self.advance();
                }

                '=' => {
                    tokens.push(self.simple(Token::Equal));
                    self.advance();
                }

                '>' => {
                    match self.peek_next() {
                        Some('=') => {
                            self.advance(); // >
                            self.advance(); // =
                            tokens.push(self.simple(Token::GreaterThanOrEqual));
                        }
                        _ => {
                            self.advance();
                            tokens.push(self.simple(Token::GreaterThan));
                        }
                    }
                }

                '<' => {
                    match self.peek_next() {
                        Some('=') => {
                            self.advance(); // <
                            self.advance(); // =
                            tokens.push(self.simple(Token::LessThanOrEqual));
                        }
                        Some('>') => {
                            self.advance(); // <
                            self.advance(); // >
                            tokens.push(self.simple(Token::NotEqual));
                        }
                        _ => {
                            self.advance();
                            tokens.push(self.simple(Token::LessThan));
                        }
                    }
                }

                '!' => match self.peek_next() {
                    Some('=') => {
                        self.advance();
                        self.advance();
                        tokens.push(self.simple(Token::NotEqual));
                    }
                    _ => {
                        return Err(LexError::UnexpectedCharacter {
                            character: '!',
                            position: self.position,
                        });
                    }
                },

                _ => {
                    return Err(LexError::UnexpectedCharacter {
                        character: ch,
                        position: self.position,
                    });
                }
            }
        }

        tokens.push(SpannedToken {
            token: Token::Eof,
            span: Span::new(self.position, self.position),
        });

        Ok(tokens)
    }

    fn lex_identifier(&mut self) -> SpannedToken {
        let start = self.position;

        while let Some(ch) = self.current() {
            if !is_identifier_part(ch) {
                break;
            }

            self.advance();
        }

        parse_identifier_or_keyword(self.source, start, self.position)
    }

    fn lex_number(&mut self) -> SpannedToken {
        let start = self.position;
        let mut seen_dot = false;

        while let Some(ch) = self.current() {
            if ch == '.' && !seen_dot {
                seen_dot = true;
                self.advance();
                continue;
            }

            if !ch.is_ascii_digit() {
                break;
            }

            self.advance();
        }

        parse_number(self.source, start, self.position)
    }

    fn lex_string(&mut self) -> Result<SpannedToken, LexError> {
        let start = self.position;

        self.advance();

        let mut value = String::new();

        while let Some(ch) = self.current() {
            if ch == '\'' {
                self.advance();

                return Ok(parse_string(value, start, self.position));
            }

            value.push(ch);
            self.advance();
        }

        Err(LexError::UnterminatedString { start })
    }
}
