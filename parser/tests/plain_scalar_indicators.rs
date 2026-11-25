use saphyr_parser::{Event, Parser, ScalarStyle, ScanError};

// Regression guards for StrInput::next_can_be_plain_scalar simplification.
// YAML 1.2 7.3.3: indicator characters can end a plain scalar in certain positions.

#[test]
fn colon_followed_by_space_ends_plain_scalar_not_part_of_value() {
    // After "foo:", seeing "a: b" means the colon after 'a' is treated as YAML syntax,
    // not as part of the plain scalar. Without a newline/indentation or flow braces,
    // this input is invalid in YAML and our parser must error (it must NOT accept
    // "a: b" as a single scalar value).
    let s = "foo: a: b\n";

    let it = Parser::new_from_str(s);
    let mut got_err: Option<ScanError> = None;
    for res in it {
        if let Err(e) = res {
            got_err = Some(e);
            break;
        }
    }
    let err = got_err.expect("no error on inline nested mapping without indentation");
    assert_eq!(err.info(), "mapping values are not allowed in this context");
}

#[test]
fn colon_without_space_is_part_of_scalar_value() {
    // When there is no blank after ':', the ':' is part of the plain scalar (7.3.3).
    // Here the value should be the single scalar "a:b".
    let s = "k: a:b\n";
    let events: Vec<_> = Parser::new_from_str(s).map(|r| r.unwrap().0).collect();
    assert_eq!(
        events,
        vec![
            Event::StreamStart,
            Event::DocumentStart(false),
            Event::MappingStart(0, None),
            Event::Scalar("k".into(), ScalarStyle::Plain, 0, None),
            Event::Scalar("a:b".into(), ScalarStyle::Plain, 0, None),
            Event::MappingEnd,
            Event::DocumentEnd,
            Event::StreamEnd,
        ]
    );
}

/// Collects the values of the scalar events, or the first error.
fn scalars(input: &str) -> Result<Vec<String>, ScanError> {
    let mut values = Vec::new();
    for event in Parser::new_from_str(input) {
        if let Event::Scalar(value, ..) = event?.0 {
            values.push(value.into_owned());
        }
    }
    Ok(values)
}

#[test]
fn dash_continuing_plain_scalar_may_precede_flow_indicator() {
    // Only the first character of a plain scalar restricts what may follow a `-`.
    for (input, expected) in [
        ("[a -]", &["a -"][..]),
        ("[a\n-]", &["a -"]),
        ("[a -, b]", &["a -", "b"]),
        ("[a  -  ]", &["a  -"]),
        ("{a -}", &["a -", ""]),
        ("{a -: b -}", &["a -", "b -"]),
    ] {
        assert_eq!(scalars(input).unwrap(), expected, "{input:?}");
    }
}

#[test]
fn plain_scalar_cannot_start_with_dash_followed_by_flow_indicator() {
    for input in ["[-]", "[a, -]", "{-}", "[a,\n-]"] {
        let err = scalars(input).unwrap_err();
        assert_eq!(
            err.info(),
            "plain scalar cannot start with '-' followed by ,[]{}",
            "{input:?}"
        );
    }
}
