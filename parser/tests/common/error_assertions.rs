use saphyr_parser::ParseError;

pub fn assert_errors_equal<E>(left: Option<&ParseError<E>>, right: Option<&ParseError<E>>) {
    match (left, right) {
        (None, None) => {}
        (Some(ParseError::Syntax(left)), Some(ParseError::Syntax(right))) => {
            assert_eq!(left, right);
        }
        _ => panic!("parser errors differ or are not syntax errors"),
    }
}
