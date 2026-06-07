use interface::{BinaryOp, Expr, ExprKind, Keyword, SpannedToken, Token};

use crate::{error::ParseError, parser::Parser};

impl Parser {
    pub(crate) fn current(&self) -> &SpannedToken {
        &self.tokens[self.position]
    }

    pub(crate) fn advance(&mut self) {
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
    }

    pub(crate) fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), ParseError> {
        match &self.current().token {
            Token::Keyword(k) if *k == keyword => {
                self.advance();
                Ok(())
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", keyword),
                found: format!("{:?}", self.current().token),
                span: self.current().span,
            }),
        }
    }

    pub(crate) fn expect_token(&mut self, token: Token) -> Result<(), ParseError> {
        if self.current().token == token {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", token),
                found: format!("{:?}", self.current().token),
                span: self.current().span,
            })
        }
    }

    pub(crate) fn parse_identifier(&mut self) -> Result<String, ParseError> {
        match &self.current().token {
            Token::Identifier(name) => {
                let value = name.clone();
                self.advance();
                Ok(value)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".into(),
                found: format!("{:?}", self.current().token),
                span: self.current().span,
            }),
        }
    }

    pub(crate) fn parse_identifier_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut identifiers = Vec::new();

        self.expect_token(Token::LParen)?;

        loop {
            identifiers.push(self.parse_identifier()?);

            match &self.current().token {
                Token::Comma => {
                    self.advance();
                }
                Token::RParen => {
                    self.advance();
                    break;
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or ')'".into(),
                        found: format!("{:?}", self.current().token),
                        span: self.current().span,
                    });
                }
            }
        }

        Ok(identifiers)
    }

    pub(crate) fn parse_expression_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut expressions = Vec::new();

        self.expect_token(Token::LParen)?;

        loop {
            expressions.push(self.parse_expression()?);

            match &self.current().token {
                Token::Comma => {
                    self.advance();
                }
                Token::RParen => {
                    self.advance();
                    break;
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or ')'".into(),
                        found: format!("{:?}", self.current().token),
                        span: self.current().span,
                    });
                }
            }
        }

        Ok(expressions)
    }

    fn parse_binary_op_if_any(&mut self) -> Option<BinaryOp> {
        match &self.current().token {
            Token::Equal => {
                self.advance();
                Some(BinaryOp::Equal)
            }

            Token::NotEqual => {
                self.advance();
                Some(BinaryOp::NotEqual)
            }

            Token::GreaterThan => {
                self.advance();
                Some(BinaryOp::GreaterThan)
            }

            Token::LessThan => {
                self.advance();
                Some(BinaryOp::LessThan)
            }

            Token::GreaterThanOrEqual => {
                self.advance();
                Some(BinaryOp::GreaterThanOrEqual)
            }

            Token::LessThanOrEqual => {
                self.advance();
                Some(BinaryOp::LessThanOrEqual)
            }

            _ => None,
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.current().clone();

        match token.token {
            Token::Identifier(name) => {
                self.advance();
                Ok(Expr::identifier(name, token.span))
            }

            Token::Integer(i) => {
                self.advance();
                Ok(Expr::integer(i, token.span))
            }

            Token::String(s) => {
                self.advance();
                Ok(Expr::string(s, token.span))
            }

            _ => Err(ParseError::UnexpectedToken {
                expected: "expression".into(),
                found: format!("{:?}", token.token),
                span: token.span,
            }),
        }
    }

    pub(crate) fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?; // id or literal

        while let Some(op) = self.parse_binary_op_if_any() {
            let right = self.parse_primary()?; // next value

            let span = left.span.merge(right.span);

            left = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }
}
