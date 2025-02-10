use std::fs::{self, DirEntry};

use libtest_mimic::{run_tests, Arguments, Outcome, Test};

use saphyr::{Mapping, Yaml};
use saphyr_parser::{
    Event, Marker, Parser, ScanError, Span, SpannedEventReceiver, TScalarStyle, Tag,
};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

struct YamlTest {
    yaml_visual: String,
    yaml: String,
    expected_events: String,
    expected_error: bool,
}

fn main() -> Result<()> {
    let mut arguments = Arguments::from_args();
    if arguments.num_threads.is_none() {
        arguments.num_threads = Some(1);
    }
    let tests: Vec<Vec<_>> = std::fs::read_dir("tests/yaml-test-suite/src")?
        .map(|entry| -> Result<_> {
            let entry = entry?;
            let tests = load_tests_from_file(&entry)?;
            Ok(tests)
        })
        .collect::<Result<_>>()?;
    let mut tests: Vec<_> = tests.into_iter().flatten().collect();
    tests.sort_by_key(|t| t.name.clone());

    run_tests(&arguments, tests, run_yaml_test).exit();
}

fn run_yaml_test(test: &Test<YamlTest>) -> Outcome {
    let desc = &test.data;
    let reporter = parse_to_events(&desc.yaml);
    let actual_events = reporter.as_ref().map(|reporter| &reporter.events);
    let events_diff = actual_events.map(|events| events_differ(events, &desc.expected_events));
    let error_text = match (&events_diff, desc.expected_error) {
        (Ok(x), true) => Some(format!("no error when expected: {x:#?}")),
        (Err(_), true) | (Ok(None), false) => None,
        (Err(e), false) => Some(format!("unexpected error {e:?}")),
        (Ok(Some(diff)), false) => Some(format!("events differ: {diff}")),
    };

    if let Some(mut txt) = error_text {
        add_error_context(
            &mut txt,
            desc,
            events_diff.err().map(saphyr::ScanError::marker),
        );
        Outcome::Failed { msg: Some(txt) }
    } else if let Some((mut msg, span)) = reporter
        .as_ref()
        .ok()
        .and_then(|reporter| reporter.span_failures.first().cloned())
    {
        add_error_context(&mut msg, desc, Some(&span.start));
        Outcome::Failed { msg: Some(msg) }
    } else {
        Outcome::Passed
    }
}

// Enrich the error message with the failing input, and a caret pointing
// at the position that errored.
fn add_error_context(text: &mut String, desc: &YamlTest, marker: Option<&Marker>) {
    use std::fmt::Write;
    let _ = writeln!(text, "\n### Input:\n{}\n### End", desc.yaml_visual);
    if let Some(mark) = marker {
        writeln!(text, "### Error position").unwrap();
        let mut lines = desc.yaml.lines();
        for _ in 0..(mark.line() - 1) {
            let l = lines.next().unwrap();
            writeln!(text, "{l}").unwrap();
        }
        writeln!(text, "\x1B[91;1m{}", lines.next().unwrap()).unwrap();
        for _ in 0..mark.col() {
            write!(text, " ").unwrap();
        }
        writeln!(text, "^\x1b[m").unwrap();
        for l in lines {
            writeln!(text, "{l}").unwrap();
        }
        writeln!(text, "### End error position").unwrap();
    }
}

fn load_tests_from_file(entry: &DirEntry) -> Result<Vec<Test<YamlTest>>> {
    let file_name = entry.file_name().to_string_lossy().to_string();
    let test_name = file_name
        .strip_suffix(".yaml")
        .ok_or("unexpected filename")?;
    let tests = Yaml::load_from_str(&fs::read_to_string(entry.path())?)
        .map_err(|e| format!("While reading {file_name}: {e}"))?;
    let tests = tests[0].as_vec().ok_or("no test list found in file")?;

    let mut result = vec![];
    let mut current_test = Mapping::new();
    for (idx, test_data) in tests.iter().enumerate() {
        let name = if tests.len() > 1 {
            format!("{test_name}-{idx:02}")
        } else {
            test_name.to_string()
        };

        // Test fields except `fail` are "inherited"
        let test_data = test_data.as_mapping().unwrap();
        current_test.remove(&Yaml::String("fail".into()));
        for (key, value) in test_data.clone() {
            current_test.insert(key, value);
        }

        let current_test = Yaml::Mapping(current_test.clone()); // Much better indexing

        if current_test.contains_mapping_key("skip") {
            continue;
        }

        result.push(Test {
            name,
            kind: String::new(),
            is_ignored: false,
            is_bench: false,
            data: YamlTest {
                yaml_visual: current_test["yaml"].as_str().unwrap().to_string(),
                yaml: visual_to_raw(current_test["yaml"].as_str().unwrap()),
                expected_events: visual_to_raw(current_test["tree"].as_str().unwrap()),
                expected_error: current_test
                    .as_mapping_get("fail")
                    .map(|x| x.as_bool().unwrap_or(false))
                    == Some(true),
            },
        });
    }
    Ok(result)
}

fn parse_to_events(source: &str) -> Result<EventReporter, ScanError> {
    let mut str_events = vec![];
    let mut str_error = None;
    let mut iter_events = vec![];
    let mut iter_error = None;

    // Parse as string
    for x in Parser::new_from_str(source) {
        match x {
            Ok(event) => str_events.push(event),
            Err(e) => {
                str_error = Some(e);
                break;
            }
        }
    }
    // Parse as iter
    for x in Parser::new_from_iter(source.chars()) {
        match x {
            Ok(event) => iter_events.push(event),
            Err(e) => {
                iter_error = Some(e);
                break;
            }
        }
    }

    // No matter the input, we should parse into the same events.
    assert_eq!(str_events, iter_events);
    // Or the same error.
    assert_eq!(str_error, iter_error);
    // If we had an error, return it so the test fails.
    if let Some(err) = str_error {
        return Err(err);
    }

    // Put events into the reporter, for comparison with the test suite.
    let mut reporter = EventReporter::default();
    for x in str_events {
        reporter.on_event(x.0, x.1);
    }
    Ok(reporter)
}

#[derive(Default)]
/// A [`SpannedEventReceiver`] checking for inconsistencies in event [`Spans`].
pub struct EventReporter<'input> {
    pub events: Vec<String>,
    last_span: Option<(Event<'input>, Span)>,
    pub span_failures: Vec<(String, Span)>,
}

impl<'input> SpannedEventReceiver<'input> for EventReporter<'input> {
    fn on_event(&mut self, ev: Event<'input>, span: Span) {
        if let Some((last_ev, last_span)) = self.last_span.take() {
            if span.start.index() < last_span.start.index()
                || span.end.index() < last_span.end.index()
            {
                self.span_failures.push((
                    format!("event {ev:?}@{span:?} came before event {last_ev:?}@{last_span:?}"),
                    span,
                ));
            }
        }
        self.last_span = Some((ev.clone(), span));

        let line: String = match ev {
            Event::StreamStart => "+STR".into(),
            Event::StreamEnd => "-STR".into(),

            Event::DocumentStart(_) => "+DOC".into(),
            Event::DocumentEnd => "-DOC".into(),

            Event::SequenceStart(idx, tag) => {
                format!("+SEQ{}{}", format_index(idx), format_tag(&tag))
            }
            Event::SequenceEnd => "-SEQ".into(),

            Event::MappingStart(idx, tag) => {
                format!("+MAP{}{}", format_index(idx), format_tag(&tag))
            }
            Event::MappingEnd => "-MAP".into(),

            Event::Scalar(ref text, style, idx, ref tag) => {
                let kind = match style {
                    TScalarStyle::Plain => ":",
                    TScalarStyle::SingleQuoted => "'",
                    TScalarStyle::DoubleQuoted => r#"""#,
                    TScalarStyle::Literal => "|",
                    TScalarStyle::Folded => ">",
                };
                format!(
                    "=VAL{}{} {}{}",
                    format_index(idx),
                    format_tag(tag),
                    kind,
                    escape_text(text)
                )
            }
            Event::Alias(idx) => format!("=ALI *{idx}"),
            Event::Nothing => return,
        };
        self.events.push(line);
    }
}

fn format_index(idx: usize) -> String {
    if idx > 0 {
        format!(" &{idx}")
    } else {
        String::new()
    }
}

fn escape_text(text: &str) -> String {
    let mut text = text.to_owned();
    for (ch, replacement) in [
        ('\\', r"\\"),
        ('\n', "\\n"),
        ('\r', "\\r"),
        ('\x08', "\\b"),
        ('\t', "\\t"),
    ] {
        text = text.replace(ch, replacement);
    }
    text
}

fn format_tag(tag: &Option<Tag>) -> String {
    if let Some(tag) = tag {
        format!(" <{}{}>", tag.handle, tag.suffix)
    } else {
        String::new()
    }
}

fn events_differ(actual: &[String], expected: &str) -> Option<String> {
    let actual = actual.iter().map(Some).chain(std::iter::repeat(None));
    let expected = expected_events(expected);
    let expected = expected.iter().map(Some).chain(std::iter::repeat(None));
    for (idx, (act, exp)) in actual.zip(expected).enumerate() {
        return match (act, exp) {
            (Some(act), Some(exp)) => {
                if act == exp {
                    continue;
                } else {
                    Some(format!(
                        "line {idx} differs: \n=> expected `{exp}`\n=>    found `{act}`",
                    ))
                }
            }
            (Some(a), None) => Some(format!("extra actual line: {a:?}")),
            (None, Some(e)) => Some(format!("extra expected line: {e:?}")),
            (None, None) => None,
        };
    }
    unreachable!()
}

/// Convert the snippets from "visual" to "actual" representation
fn visual_to_raw(yaml: &str) -> String {
    let mut yaml = yaml.to_owned();
    for (pat, replacement) in [
        ("␣", " "),
        ("»", "\t"),
        ("—", ""), // Tab line continuation ——»
        ("←", "\r"),
        ("⇔", "\u{FEFF}"),
        ("↵", ""), // Trailing newline marker
        ("∎\n", ""),
    ] {
        yaml = yaml.replace(pat, replacement);
    }
    yaml
}

/// Adapt the expectations to the yaml-rust reasonable limitations
///
/// Drop information on node styles (flow/block) and anchor names.
/// Both are things that can be omitted according to spec.
fn expected_events(expected_tree: &str) -> Vec<String> {
    let mut anchors = vec![];
    expected_tree
        .split('\n')
        .map(|s| s.trim_start().to_owned())
        .filter(|s| !s.is_empty())
        .map(|mut s| {
            // Anchor name-to-number conversion
            if let Some(start) = s.find('&') {
                if s[..start].find(':').is_none() {
                    let len = s[start..].find(' ').unwrap_or(s[start..].len());
                    anchors.push(s[start + 1..start + len].to_owned());
                    s = s.replace(&s[start..start + len], &format!("&{}", anchors.len()));
                }
            }
            // Alias nodes name-to-number
            if s.starts_with("=ALI") {
                let start = s.find('*').unwrap();
                let name = &s[start + 1..];
                let idx = anchors
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v == &name)
                    .last()
                    .unwrap()
                    .0;
                s = s.replace(&s[start..], &format!("*{}", idx + 1));
            }
            // Dropping style information
            match &*s {
                "+DOC ---" => "+DOC".into(),
                "-DOC ..." => "-DOC".into(),
                s if s.starts_with("+SEQ []") => s.replacen("+SEQ []", "+SEQ", 1),
                s if s.starts_with("+MAP {}") => s.replacen("+MAP {}", "+MAP", 1),
                "=VAL :" => "=VAL :~".into(), // FIXME: known bug
                s => s.into(),
            }
        })
        .collect()
}
