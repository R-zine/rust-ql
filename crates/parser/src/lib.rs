mod error;
mod helpers;
mod parser;

pub use crate::error::ParseError;
pub use crate::parser::Parser;

#[cfg(test)]
mod tests {
    use interface::{BinaryOp, DataType, ExprKind, Literal, StatementKind, UnaryOp};
    use lexer::Lexer;

    use super::{ParseError, Parser};

    fn parse(source: &str) -> Result<interface::Statement, ParseError> {
        let tokens = Lexer::new(source).tokenize().expect("source should lex");
        Parser::new(tokens).parse()
    }

    #[test]
    fn parses_insert_with_or_without_columns() {
        let without_columns = parse("INSERT INTO users VALUES (1, 'Phil');").unwrap();
        let StatementKind::Insert(statement) = without_columns.kind else {
            panic!("expected INSERT");
        };
        assert!(statement.columns.is_empty());
        assert_eq!(statement.values.len(), 2);

        let with_columns = parse("INSERT INTO users (name, id) VALUES ('Phil', 1)").unwrap();
        let StatementKind::Insert(statement) = with_columns.kind else {
            panic!("expected INSERT");
        };
        assert_eq!(statement.columns, ["name", "id"]);
    }

    #[test]
    fn parses_every_literal_and_operator_precedence() {
        let statement = parse(
            "SELECT -1 + 2 * 3 AS result FROM value_rows WHERE enabled = TRUE AND note != NULL",
        )
        .unwrap();
        let StatementKind::Select(statement) = statement.kind else {
            panic!("expected SELECT");
        };
        let interface::SelectItem::Expression { expr, alias } = &statement.columns[0] else {
            panic!("expected expression");
        };
        assert_eq!(alias.as_deref(), Some("result"));
        let ExprKind::Binary {
            left,
            op: BinaryOp::Add,
            right,
        } = &expr.kind
        else {
            panic!("expected addition at the root");
        };
        assert!(matches!(
            left.kind,
            ExprKind::Unary {
                op: UnaryOp::Minus,
                ..
            }
        ));
        assert!(matches!(
            right.kind,
            ExprKind::Binary {
                op: BinaryOp::Multiply,
                ..
            }
        ));

        let ExprKind::Binary {
            op: BinaryOp::And,
            right,
            ..
        } = statement.where_clause.unwrap().kind
        else {
            panic!("expected AND at the root");
        };
        assert!(matches!(
            right.kind,
            ExprKind::Binary {
                right,
                op: BinaryOp::NotEqual,
                ..
            } if matches!(right.kind, ExprKind::Literal(Literal::Null))
        ));
    }

    #[test]
    fn parses_nullability_and_rejects_multiple_primary_keys() {
        let statement =
            parse("CREATE TABLE users (id INTEGER PRIMARY KEY, name STRING NOT NULL, score FLOAT)")
                .unwrap();
        let StatementKind::CreateTable(statement) = statement.kind else {
            panic!("expected CREATE TABLE");
        };
        assert_eq!(statement.columns[0].data_type, DataType::Integer);
        assert!(!statement.columns[0].nullable);
        assert!(!statement.columns[1].nullable);
        assert!(statement.columns[2].nullable);

        assert!(matches!(
            parse("CREATE TABLE bad (a INTEGER PRIMARY KEY, b INTEGER PRIMARY KEY)"),
            Err(ParseError::Message { .. })
        ));
        assert!(matches!(
            parse("CREATE TABLE bad (id FLOAT PRIMARY KEY)"),
            Err(ParseError::Message { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_columns_and_trailing_input() {
        assert!(matches!(
            parse("CREATE TABLE bad (id INTEGER, ID STRING)"),
            Err(ParseError::Message { .. })
        ));
        assert!(matches!(
            parse("SELECT * FROM users trailing"),
            Err(ParseError::UnexpectedToken { .. })
        ));
        assert!(parse("SELECT * FROM users;").is_ok());
    }

    #[test]
    fn an_empty_token_stream_returns_an_error() {
        assert_eq!(
            Parser::new(Vec::new()).parse(),
            Err(ParseError::UnexpectedEof)
        );
    }
}
