use interface::{
    ColumnDefinition, CreateTableStatement, DataType, InsertStatement, Keyword, SelectItem,
    SelectStatement, SpannedToken, Statement, StatementKind, Token,
};

use crate::error::ParseError;

pub struct Parser {
    pub(crate) tokens: Vec<SpannedToken>,
    pub(crate) position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Statement, ParseError> {
        match &self.current().token {
            Token::Keyword(Keyword::Select) => self.parse_select(),

            Token::Keyword(Keyword::Insert) => self.parse_insert(),

            Token::Keyword(Keyword::Create) => self.parse_create_table(),

            _ => Err(ParseError::UnexpectedToken {
                expected: "statement".into(),
                found: format!("{:?}", self.current().token),
                span: self.current().span,
            }),
        }
    }

    fn parse_select(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.current().span;

        self.expect_keyword(Keyword::Select)?;

        let columns = self.parse_select_columns()?;

        self.expect_keyword(Keyword::From)?;

        let table = self.parse_identifier()?;

        let where_clause = match &self.current().token {
            Token::Keyword(Keyword::Where) => {
                self.advance();
                Some(self.parse_expression()?)
            }
            _ => None,
        };

        let end_span = self.current().span;

        Ok(Statement::new(
            StatementKind::Select(SelectStatement {
                columns,
                table,
                where_clause,
            }),
            start_span.merge(end_span),
        ))
    }

    fn parse_insert(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.current().span;

        self.expect_keyword(Keyword::Insert)?;
        self.expect_keyword(Keyword::Into)?;

        let table = self.parse_identifier()?;

        let columns = self.parse_identifier_list()?;

        self.expect_keyword(Keyword::Values)?;

        let values = self.parse_expression_list()?;

        let end_span = self.current().span;

        Ok(Statement::new(
            StatementKind::Insert(InsertStatement {
                table,
                columns,
                values,
            }),
            start_span.merge(end_span),
        ))
    }

    fn parse_create_table(&mut self) -> Result<Statement, ParseError> {
        let start_span = self.current().span;

        self.expect_keyword(Keyword::Create)?;
        self.expect_keyword(Keyword::Table)?;

        let name = self.parse_identifier()?;

        let columns = self.parse_column_definitions()?;

        let end_span = self.current().span;

        Ok(Statement::new(
            StatementKind::CreateTable(CreateTableStatement { name, columns }),
            start_span.merge(end_span),
        ))
    }

    fn parse_select_columns(&mut self) -> Result<Vec<SelectItem>, ParseError> {
        let mut columns = Vec::new();

        if matches!(self.current().token, Token::Star) {
            self.advance();

            columns.push(SelectItem::Wildcard);

            return Ok(columns);
        }

        loop {
            let name = self.parse_identifier()?;

            columns.push(SelectItem::Expression {
                expr: interface::Expr::new(
                    interface::ExprKind::Identifier(name),
                    self.current().span,
                ),
                alias: None,
            });

            match &self.current().token {
                Token::Comma => {
                    self.advance();
                }

                Token::Keyword(Keyword::From) => {
                    break;
                }

                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or FROM".into(),
                        found: format!("{:?}", self.current().token),
                        span: self.current().span,
                    });
                }
            }
        }

        Ok(columns)
    }

    fn parse_column_definitions(&mut self) -> Result<Vec<ColumnDefinition>, ParseError> {
        let mut columns = Vec::new();

        self.expect_token(Token::LParen)?;

        loop {
            let name = self.parse_identifier()?;

            let data_type = self.parse_data_type()?;

            let mut primary_key = false;

            if matches!(self.current().token, Token::Keyword(Keyword::Primary)) {
                self.advance();

                self.expect_keyword(Keyword::Key)?;

                primary_key = true;
            }

            columns.push(ColumnDefinition {
                name,
                data_type,
                nullable: true,
                primary_key,
            });

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

        Ok(columns)
    }

    fn parse_data_type(&mut self) -> Result<DataType, ParseError> {
        match &self.current().token {
            Token::Identifier(name) => {
                let ty = match name.to_uppercase().as_str() {
                    "INTEGER" | "INT" => DataType::Integer,
                    "FLOAT" | "DOUBLE" => DataType::Float,
                    "BOOLEAN" | "BOOL" => DataType::Boolean,
                    "STRING" | "TEXT" | "VARCHAR" => DataType::String,

                    _ => {
                        return Err(ParseError::Message {
                            message: format!("Unknown data type '{}'", name),
                            span: self.current().span,
                        });
                    }
                };

                self.advance();

                Ok(ty)
            }

            _ => Err(ParseError::UnexpectedToken {
                expected: "data type".into(),
                found: format!("{:?}", self.current().token),
                span: self.current().span,
            }),
        }
    }
}
