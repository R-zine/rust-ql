use interface::{
    ColumnDefinition, CreateTableStatement, DataType, InsertStatement, Keyword, SelectItem,
    SelectStatement, Span, SpannedToken, Statement, StatementKind, Token,
};

use crate::error::ParseError;

pub struct Parser {
    pub(crate) tokens: Vec<SpannedToken>,
    pub(crate) position: usize,
}

impl Parser {
    pub fn new(mut tokens: Vec<SpannedToken>) -> Self {
        if !tokens.last().is_some_and(|token| token.token == Token::Eof) {
            let end = tokens.last().map_or(0, |token| token.span.end);
            tokens.push(SpannedToken::new(Token::Eof, Span::new(end, end)));
        }

        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Statement, ParseError> {
        let statement = match &self.current().token {
            Token::Keyword(Keyword::Select) => self.parse_select()?,
            Token::Keyword(Keyword::Insert) => self.parse_insert()?,
            Token::Keyword(Keyword::Create) => self.parse_create_table()?,
            Token::Eof => return Err(ParseError::UnexpectedEof),
            _ => return Err(self.unexpected("statement")),
        };

        if self.current().token == Token::Semicolon {
            self.advance();
        }
        self.expect_token(Token::Eof)?;

        Ok(statement)
    }

    fn parse_select(&mut self) -> Result<Statement, ParseError> {
        let start = self.current().span;
        self.expect_keyword(Keyword::Select)?;
        let columns = self.parse_select_columns()?;
        self.expect_keyword(Keyword::From)?;
        let table = self.parse_identifier()?;

        let where_clause = if self.current().token == Token::Keyword(Keyword::Where) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::new(
            StatementKind::Select(SelectStatement {
                columns,
                table,
                where_clause,
            }),
            start.merge(self.previous_span()),
        ))
    }

    fn parse_insert(&mut self) -> Result<Statement, ParseError> {
        let start = self.current().span;
        self.expect_keyword(Keyword::Insert)?;
        self.expect_keyword(Keyword::Into)?;
        let table = self.parse_identifier()?;

        let columns = if self.current().token == Token::LParen {
            self.parse_identifier_list()?
        } else {
            Vec::new()
        };

        self.expect_keyword(Keyword::Values)?;
        let values = self.parse_expression_list()?;

        Ok(Statement::new(
            StatementKind::Insert(InsertStatement {
                table,
                columns,
                values,
            }),
            start.merge(self.previous_span()),
        ))
    }

    fn parse_create_table(&mut self) -> Result<Statement, ParseError> {
        let start = self.current().span;
        self.expect_keyword(Keyword::Create)?;
        self.expect_keyword(Keyword::Table)?;
        let name = self.parse_identifier()?;
        let columns = self.parse_column_definitions()?;

        Ok(Statement::new(
            StatementKind::CreateTable(CreateTableStatement { name, columns }),
            start.merge(self.previous_span()),
        ))
    }

    fn parse_select_columns(&mut self) -> Result<Vec<SelectItem>, ParseError> {
        let mut columns = Vec::new();

        loop {
            if self.current().token == Token::Star {
                self.advance();
                columns.push(SelectItem::Wildcard);
            } else {
                let expr = self.parse_expression()?;
                let alias = if self.current().token == Token::Keyword(Keyword::As) {
                    self.advance();
                    Some(self.parse_identifier()?)
                } else {
                    None
                };
                columns.push(SelectItem::Expression { expr, alias });
            }

            match &self.current().token {
                Token::Comma => self.advance(),
                Token::Keyword(Keyword::From) => break,
                _ => return Err(self.unexpected("',' or FROM")),
            }
        }

        Ok(columns)
    }

    fn parse_column_definitions(&mut self) -> Result<Vec<ColumnDefinition>, ParseError> {
        let mut columns = Vec::new();
        let mut primary_key_seen = false;
        self.expect_token(Token::LParen)?;

        loop {
            let (name, name_span) = self.parse_identifier_with_span()?;
            if columns
                .iter()
                .any(|column: &ColumnDefinition| column.name.eq_ignore_ascii_case(&name))
            {
                return Err(ParseError::Message {
                    message: format!("duplicate column '{name}'"),
                    span: name_span,
                });
            }

            let data_type = self.parse_data_type()?;
            let mut nullable = true;
            let mut primary_key = false;
            let mut not_null_seen = false;

            loop {
                match &self.current().token {
                    Token::Keyword(Keyword::Primary) => {
                        let constraint_span = self.current().span;
                        if primary_key || primary_key_seen {
                            return Err(ParseError::Message {
                                message: "a table may have only one primary key".into(),
                                span: constraint_span,
                            });
                        }
                        self.advance();
                        self.expect_keyword(Keyword::Key)?;
                        primary_key = true;
                        primary_key_seen = true;
                        nullable = false;
                    }
                    Token::Not => {
                        let constraint_span = self.current().span;
                        if not_null_seen {
                            return Err(ParseError::Message {
                                message: "duplicate NOT NULL constraint".into(),
                                span: constraint_span,
                            });
                        }
                        self.advance();
                        self.expect_token(Token::Null)?;
                        nullable = false;
                        not_null_seen = true;
                    }
                    _ => break,
                }
            }

            if primary_key && data_type == DataType::Float {
                return Err(ParseError::Message {
                    message: "FLOAT primary keys are not supported".into(),
                    span: name_span,
                });
            }

            columns.push(ColumnDefinition {
                name,
                data_type,
                nullable,
                primary_key,
            });

            match &self.current().token {
                Token::Comma => self.advance(),
                Token::RParen => {
                    self.advance();
                    break;
                }
                _ => return Err(self.unexpected("',' or ')'")),
            }
        }

        Ok(columns)
    }

    fn parse_data_type(&mut self) -> Result<DataType, ParseError> {
        let token = self.current().clone();
        let Token::Identifier(name) = token.token else {
            return Err(self.unexpected("data type"));
        };

        let data_type = match name.to_ascii_uppercase().as_str() {
            "INTEGER" | "INT" => DataType::Integer,
            "FLOAT" | "DOUBLE" => DataType::Float,
            "BOOLEAN" | "BOOL" => DataType::Boolean,
            "STRING" | "TEXT" | "VARCHAR" => DataType::String,
            _ => {
                return Err(ParseError::Message {
                    message: format!("unknown data type '{name}'"),
                    span: token.span,
                });
            }
        };

        self.advance();
        Ok(data_type)
    }
}
