use saphyr_parser::{Parser, ScanError};

// ZYU8: Directive variants
// In YAML 1.2, a directive name is any non‑space, non‑line‑break sequence of characters
// Saphyr expects only alphabetic characters in a directive name, dot . triggers the error.
#[test]
fn yaml_zyu8_directive_variant_yaml11_null_document() {
    let yaml = "%YAML1.1\n---\n";
    let mut got_err: Option<ScanError> = None;

    for item in Parser::new_from_str(yaml) {
        match item {
            Ok((_event, _span)) => {
                continue
            },
            Err(e) => {
                got_err = Some(e);
                break;
            }
        }
    }
    assert!(got_err.is_none(), "Error: {}", got_err.unwrap().info());
}

#[test]
fn yaml_reserved_directive_stars() {
    let yaml = "%***\n---\n";
    let mut got_err: Option<ScanError> = None;

    for item in Parser::new_from_str(yaml) {
        if let Err(e) = item {
            got_err = Some(e);
            break;
        }
    }
    assert!(got_err.is_none(), "Error: {}", got_err.unwrap().info());
}

#[test]
fn yaml_bad_yaml_directive() {
    let yaml = "%YAML 1.1 1.2\n---\n";
    let mut got_err: Option<ScanError> = None;

    for item in Parser::new_from_str(yaml) {
        if let Err(e) = item {
            got_err = Some(e);
            break;
        }
    }
    // This should fail because "YAML" is a defined directive and it has too many parameters.
    assert!(got_err.is_some());
    assert!(got_err.unwrap().info().contains("did not find expected comment or line break"));
}

#[test]
fn yaml_reserved_directive_with_params() {
    let yaml = "%FOO bar baz\n---\n";
    let mut got_err: Option<ScanError> = None;

    for item in Parser::new_from_str(yaml) {
        if let Err(e) = item {
            got_err = Some(e);
            break;
        }
    }
    assert!(got_err.is_none(), "Error: {}", got_err.unwrap().info());
}
