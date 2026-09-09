pub mod expr;
pub mod keyword;
pub mod span;
pub mod statement;
pub mod token;

pub use span::Span;

pub use keyword::Keyword;

pub use token::{SpannedToken, Token};

pub use expr::{BinaryOp, Expr, ExprKind, Literal, UnaryOp, Value};

pub use statement::{
    ColumnDefinition, CreateTableStatement, DataType, InsertStatement, SelectItem, SelectStatement,
    Statement, StatementKind,
};

#[cfg(test)]
mod tests {
    use super::{Span, Value};

    #[test]
    fn spans_merge_slice_and_use_half_open_bounds() {
        let source = "a🚀bc";
        let rocket = Span::new(1, 5);
        assert_eq!(rocket.len(), 4);
        assert_eq!(rocket.slice(source), "🚀");
        assert!(rocket.contains(1));
        assert!(!rocket.contains(5));
        assert_eq!(rocket.merge(Span::new(5, 7)), Span::new(1, 7));
    }

    #[test]
    fn values_have_stable_type_names_and_display_values() {
        let values = [
            (Value::Integer(1), "INTEGER", "1"),
            (Value::Float(1.5), "FLOAT", "1.5"),
            (Value::String("text".into()), "STRING", "text"),
            (Value::Boolean(true), "BOOLEAN", "true"),
            (Value::Null, "NULL", "NULL"),
        ];

        for (value, type_name, display) in values {
            assert_eq!(value.type_name(), type_name);
            assert_eq!(value.to_string(), display);
        }
    }
}
