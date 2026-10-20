use std::{cell::RefCell, rc::Rc, string::String};

use saphyr::{LoadableYamlNode, ParseError, Yaml};

struct BorrowedError<'a> {
    message: &'a str,
    identity: Rc<()>,
}

#[test]
fn fallible_loader_returns_source_error_instead_of_a_partial_document_set() {
    let message = String::from("read budget exhausted");
    let identity = Rc::new(());
    let error_identity = Rc::clone(&identity);
    let reads = Rc::new(RefCell::new(0));
    let source = FailingChars {
        chars: "key: value\n".chars(),
        error: Some(BorrowedError {
            message: &message,
            identity: error_identity,
        }),
        reads: Rc::clone(&reads),
    };

    let error = Yaml::load_from_fallible_iter(source).expect_err("source failure must fail load");
    match error {
        ParseError::Source(error) => {
            assert_eq!(error.message, "read budget exhausted");
            assert!(Rc::ptr_eq(&identity, &error.identity));
        }
        _ => panic!("expected source error"),
    }
    assert_eq!(*reads.borrow(), "key: value\n".chars().count() + 1);
}

struct FailingChars<'a> {
    chars: std::str::Chars<'a>,
    error: Option<BorrowedError<'a>>,
    reads: Rc<RefCell<usize>>,
}

impl<'a> Iterator for FailingChars<'a> {
    type Item = Result<char, BorrowedError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        *self.reads.borrow_mut() += 1;
        if let Some(ch) = self.chars.next() {
            return Some(Ok(ch));
        }
        let error = self
            .error
            .take()
            .expect("the loader read after the source error");
        Some(Err(error))
    }
}

#[test]
fn ordinary_string_loading_still_accepts_eof_and_complete_documents() {
    let docs = Yaml::load_from_str("key: value\nnext: row\n").unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["key"].as_str(), Some("value"));
    assert_eq!(docs[0]["next"].as_str(), Some("row"));
}

#[test]
fn iterator_sources_can_be_consumed_into_owned_documents() {
    fn parse_string(source: String) -> Vec<Yaml<'static>> {
        Yaml::load_from_str(&source).unwrap()
    }

    fn parse_char_iterator(source: String) -> Vec<Yaml<'static>> {
        Yaml::load_from_iter(source.chars()).unwrap()
    }

    fn parse_fallible_iterator(source: String) -> Vec<Yaml<'static>> {
        Yaml::load_from_fallible_iter(source.chars().map(Ok::<char, core::convert::Infallible>))
            .unwrap()
    }

    for documents in [
        parse_string(String::from("key: value\n")),
        parse_char_iterator(String::from("key: value\n")),
        parse_fallible_iterator(String::from("key: value\n")),
    ] {
        assert_eq!(documents[0]["key"].as_str(), Some("value"));
    }
}
