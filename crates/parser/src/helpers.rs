use interface::{BinaryOp, Expr, ExprKind, Keyword, Literal, Span, SpannedToken, Token, UnaryOp};

use crate::{error::ParseError, parser::Parser};

impl Parser {
    pub(crate) fn current(&self) -> &SpannedToken {
        // Parser::new always installs an EOF sentinel.
        &self.tokens[self.position]
    }

    pub(crate) fn previous_span(&self) -> Span {
        self.tokens[self.position.saturating_sub(1)].span
    }

    pub(crate) fn advance(&mut self) {
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
    }

    pub(crate) fn unexpected(&self, expected: impl Into<String>) -> ParseError {
        if self.current().token == Token::Eof {
            ParseError::UnexpectedEof
        } else {
            ParseError::UnexpectedToken {
                expected: expected.into(),
                found: format!("{:?}", self.current().token),
                span: self.current().span,
            }
        }
    }

    pub(crate) fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), ParseError> {
        if self.current().token == Token::Keyword(keyword) {
            self.advance();
            Ok(())
        } else {
            Err(self.unexpected(format!("{keyword:?}")))
        }
    }

    pub(crate) fn expect_token(&mut self, token: Token) -> Result<(), ParseError> {
        if self.current().token == token {
            self.advance();
            Ok(())
        } else {
            Err(self.unexpected(format!("{token:?}")))
        }
    }

    pub(crate) fn parse_identifier(&mut self) -> Result<String, ParseError> {
        self.parse_identifier_with_span().map(|(name, _)| name)
    }

    pub(crate) fn parse_identifier_with_span(&mut self) -> Result<(String, Span), ParseError> {
        let token = self.current().clone();
        match token.token {
            Token::Identifier(name) => {
                self.advance();
                Ok((name, token.span))
            }
            _ => Err(self.unexpected("identifier")),
        }
    }

    pub(crate) fn parse_identifier_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut identifiers = Vec::new();
        self.expect_token(Token::LParen)?;

        loop {
            identifiers.push(self.parse_identifier()?);
            match &self.current().token {
                Token::Comma => self.advance(),
                Token::RParen => {
                    self.advance();
                    break;
                }
                _ => return Err(self.unexpected("',' or ')'")),
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
                Token::Comma => self.advance(),
                Token::RParen => {
                    self.advance();
                    break;
                }
                _ => return Err(self.unexpected("',' or ')'")),
            }
        }

        Ok(expressions)
    }

    pub(crate) fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.current().token == Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = binary(left, BinaryOp::Or, right);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        while self.current().token == Token::And {
            self.advance();
            let right = self.parse_comparison()?;
            left = binary(left, BinaryOp::And, right);
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_additive()?;
        let operator = match self.current().token {
            Token::Equal => Some(BinaryOp::Equal),
            Token::NotEqual => Some(BinaryOp::NotEqual),
            Token::GreaterThan => Some(BinaryOp::GreaterThan),
            Token::LessThan => Some(BinaryOp::LessThan),
            Token::GreaterThanOrEqual => Some(BinaryOp::GreaterThanOrEqual),
            Token::LessThanOrEqual => Some(BinaryOp::LessThanOrEqual),
            _ => None,
        };

        if let Some(operator) = operator {
            self.advance();
            let right = self.parse_additive()?;
            Ok(binary(left, operator, right))
        } else {
            Ok(left)
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let operator = match self.current().token {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = binary(left, operator, right);
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let operator = match self.current().token {
                Token::Star => BinaryOp::Multiply,
                Token::Slash => BinaryOp::Divide,
                Token::Percent => BinaryOp::Modulo,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = binary(left, operator, right);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        let (operator, start) = match self.current().token {
            Token::Plus => (UnaryOp::Plus, self.current().span),
            Token::Minus => (UnaryOp::Minus, self.current().span),
            Token::Not => (UnaryOp::Not, self.current().span),
            _ => return self.parse_primary(),
        };

        self.advance();
        let expression = self.parse_unary()?;
        let span = start.merge(expression.span);
        Ok(Expr::new(
            ExprKind::Unary {
                op: operator,
                expr: Box::new(expression),
            },
            span,
        ))
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.current().clone();
        let expression = match token.token {
            Token::Identifier(name) => Expr::identifier(name, token.span),
            Token::Integer(value) => Expr::integer(value, token.span),
            Token::Float(value) => Expr::new(ExprKind::Literal(Literal::Float(value)), token.span),
            Token::String(value) => Expr::string(value, token.span),
            Token::Boolean(value) => Expr::boolean(value, token.span),
            Token::Null => Expr::new(ExprKind::Literal(Literal::Null), token.span),
            Token::LParen => {
                self.advance();
                let mut inner = self.parse_expression()?;
                self.expect_token(Token::RParen)?;
                inner.span = token.span.merge(self.previous_span());
                return Ok(inner);
            }
            _ => return Err(self.unexpected("expression")),
        };

        self.advance();
        Ok(expression)
    }
}

fn binary(left: Expr, operator: BinaryOp, right: Expr) -> Expr {
    let span = left.span.merge(right.span);
    Expr::new(
        ExprKind::Binary {
            left: Box::new(left),
            op: operator,
            right: Box::new(right),
        },
        span,
    )
}
