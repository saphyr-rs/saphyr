use saphyr_parser::{Event, Parser, ScanError};

/// Returns the tags of the scalars in `input`.
fn scalar_tags(input: &str) -> Result<Vec<String>, ScanError> {
    let mut tags = vec![];
    for event in Parser::new_from_str(input) {
        if let (Event::Scalar(_, _, _, Some(tag)), _) = event? {
            tags.push(format!("{}{}", tag.handle, tag.suffix));
        }
    }
    Ok(tags)
}

#[test]
fn multiple_tag_directives() {
    let yaml =
        "%TAG !s! tag:yaml.org,2002:\n%TAG !e! tag:example.com,2000:\n---\n- !s!str a\n- !e!b c";
    assert_eq!(
        scalar_tags(yaml).unwrap(),
        ["tag:yaml.org,2002:str", "tag:example.com,2000:b"]
    );
}

#[test]
fn duplicate_tag_directive() {
    let yaml = "%TAG !e! tag:a,2000:\n%TAG !e! tag:b,2000:\n---\n!e!x y";
    let error = scalar_tags(yaml).unwrap_err();
    assert!(error.info().contains("at most once per handle"), "{error}");
}
