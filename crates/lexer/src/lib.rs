mod helpers;
mod lexer;

pub use lexer::{LexError, Lexer};

#[cfg(test)]
mod tests {
    use interface::{Keyword, Span, Token};

    use super::{LexError, Lexer};

    fn tokens(source: &str) -> Vec<Token> {
        Lexer::new(source)
            .tokenize()
            .expect("source should lex")
            .into_iter()
            .map(|token| token.token)
            .collect()
    }

    #[test]
    fn lexes_keywords_literals_and_operators() {
        assert_eq!(
            tokens("SELECT -1, .5, 2e3, TRUE, FALSE, NULL WHERE a >= 1 AND b != 2"),
            vec![
                Token::Keyword(Keyword::Select),
                Token::Minus,
                Token::Integer(1),
                Token::Comma,
                Token::Float(0.5),
                Token::Comma,
                Token::Float(2000.0),
                Token::Comma,
                Token::Boolean(true),
                Token::Comma,
                Token::Boolean(false),
                Token::Comma,
                Token::Null,
                Token::Keyword(Keyword::Where),
                Token::Identifier("a".into()),
                Token::GreaterThanOrEqual,
                Token::Integer(1),
                Token::And,
                Token::Identifier("b".into()),
                Token::NotEqual,
                Token::Integer(2),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn uses_utf8_byte_spans_and_supports_sql_string_escaping() {
        let source = "SELECT\u{2003}name, 'O''Reilly 🚀' FROM users";
        let output = Lexer::new(source).tokenize().expect("source should lex");

        assert_eq!(output[1].token, Token::Identifier("name".into()));
        assert_eq!(output[1].span.slice(source), "name");
        assert_eq!(output[3].token, Token::String("O'Reilly 🚀".into()));
        assert_eq!(output[3].span.slice(source), "'O''Reilly 🚀'");
    }

    #[test]
    fn records_the_full_operator_span() {
        let output = Lexer::new("a >= 1").tokenize().expect("source should lex");
        assert_eq!(output[1].span, Span::new(2, 4));
    }

    #[test]
    fn reports_integer_overflow_instead_of_panicking() {
        let error = Lexer::new("999999999999999999999999999999")
            .tokenize()
            .expect_err("overflow should be rejected");
        assert!(matches!(error, LexError::InvalidNumber { .. }));
    }

    #[test]
    fn reports_unterminated_strings() {
        assert_eq!(
            Lexer::new("'unfinished").tokenize(),
            Err(LexError::UnterminatedString { start: 0 })
        );
    }
}
