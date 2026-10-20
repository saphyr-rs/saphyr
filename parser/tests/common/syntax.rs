use saphyr_parser::{ParseError, ScanError};

pub fn syntax_error<E>(error: &ParseError<E>) -> &ScanError {
    match error {
        ParseError::Syntax(error) => error,
        ParseError::Source(_) => panic!("expected a syntax error, got a source error"),
        ParseError::Failed => panic!("expected a syntax error, got a failed input"),
    }
}
