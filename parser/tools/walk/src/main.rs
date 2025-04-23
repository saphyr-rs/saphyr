use hashlink::LinkedHashMap;
use miette::{bail, Diagnostic, NamedSource, Result, SourceSpan};
use rustyline::{error::ReadlineError, DefaultEditor};
use saphyr::{LoadableYamlNode, MarkedYaml};
use thiserror::Error;

/// A REPL to navigate a YAML document from the spans.
///
/// See [`read_action`] for commands.
fn main() {
    let args: Vec<_> = std::env::args().collect();
    match args.as_slice() {
        [_, filename] => {
            let contents = std::fs::read_to_string(filename).unwrap();
            let yaml = MarkedYaml::load_from_str(&contents)
                .unwrap()
                .into_iter()
                .next()
                .unwrap();
            walk(&contents, &yaml);
        }
        _ => {
            eprintln!("Usage: walker <file.yaml>");
        }
    }
}

fn walk(contents: &str, yaml: &MarkedYaml<'_>) {
    let mut stack = vec![];
    let mut io = DefaultEditor::new().unwrap();
    stack.push(yaml);

    print(contents.to_string(), yaml);

    loop {
        let err = match read_action(&mut io) {
            Action::StepIn => step_in(&mut stack),
            Action::StepInKey => step_in_key(&mut stack),
            Action::StepInValue => step_in_value(&mut stack),
            Action::Next => next(&mut stack),
            Action::Prev => prev(&mut stack),
            Action::Fin => fin(&mut stack),
            Action::Stop => break,
        };

        match err {
            Ok(()) => {
                io.clear_screen().unwrap();
                print(contents.to_string(), stack.last().unwrap());
            }
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn print(contents: String, yaml: &MarkedYaml<'_>) {
    let pos = yaml.span.start.index();
    let len = yaml.span.len();
    eprintln!(
        "{:?}",
        miette::Error::new(FakeErr {
            src: NamedSource::new("<input>", contents),
            span: (pos, len).into(),
        })
    );
}

fn step_in(stack: &mut Stack<'_>) -> Result<()> {
    match &stack.last().unwrap().data {
        saphyr::YamlData::Sequence(seq) => do_step_in_seq(stack, seq)?,
        saphyr::YamlData::Mapping(map) => do_step_in_value(stack, map)?,
        _ => bail!("Not in a mapping or a sequence"),
    }
    Ok(())
}

fn step_in_key(stack: &mut Stack<'_>) -> Result<()> {
    match &stack.last().unwrap().data {
        saphyr::YamlData::Mapping(map) => do_step_in_key(stack, map),
        _ => bail!("Not in a mapping"),
    }
}

fn step_in_value(stack: &mut Stack<'_>) -> Result<()> {
    match &stack.last().unwrap().data {
        saphyr::YamlData::Mapping(map) => do_step_in_value(stack, map),
        _ => bail!("Not in a mapping"),
    }
}

fn next(stack: &mut Stack<'_>) -> Result<()> {
    if stack.len() == 1 {
        bail!("Can't next from top-level");
    }
    let node = stack.pop().unwrap();
    let parent = stack.last().unwrap();
    let mut pos = pos_in_parent(node, parent);
    pos.idx += 1;

    match &parent.data {
        saphyr::YamlData::Sequence(seq) => {
            if pos.idx == seq.len() {
                bail!("Reached end of the sequence");
            } else {
                stack.push(&seq[pos.idx]);
            }
        }
        saphyr::YamlData::Mapping(map) => {
            if pos.idx == map.len() {
                bail!("Reached end of the map");
            } else {
                let (key, value) = map.iter().nth(pos.idx).unwrap();
                if pos.kvtype == KVType::Key {
                    stack.push(key);
                } else {
                    stack.push(value);
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn prev(stack: &mut Stack<'_>) -> Result<()> {
    if stack.len() == 1 {
        bail!("Can't prev from top-level");
    }
    let node = stack.pop().unwrap();
    let parent = stack.last().unwrap();
    let mut pos = pos_in_parent(node, parent);
    if pos.idx == 0 {
        bail!("Already at the beginning of the collection");
    }
    pos.idx -= 1;

    match &parent.data {
        saphyr::YamlData::Sequence(seq) => {
            stack.push(&seq[pos.idx]);
        }
        saphyr::YamlData::Mapping(map) => {
            let (key, value) = map.iter().nth(pos.idx).unwrap();
            if pos.kvtype == KVType::Key {
                stack.push(key);
            } else {
                stack.push(value);
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn fin(stack: &mut Stack<'_>) -> Result<()> {
    if stack.len() > 1 {
        stack.pop();
        Ok(())
    } else {
        bail!("Already at the top-level");
    }
}

fn do_step_in_seq<'a>(stack: &mut Stack<'a>, seq: &'a YamlSeq<'a>) -> Result<()> {
    if seq.is_empty() {
        bail!("Sequence is empty");
    } else {
        stack.push(&seq[0]);
        Ok(())
    }
}

fn do_step_in_key<'a>(stack: &mut Stack<'a>, map: &'a YamlMap<'a>) -> Result<()> {
    if let Some(node) = map.keys().next() {
        stack.push(node);
        Ok(())
    } else {
        bail!("Mapping is empty");
    }
}

fn do_step_in_value<'a>(stack: &mut Stack<'a>, map: &'a YamlMap<'a>) -> Result<()> {
    if let Some(node) = map.values().next() {
        stack.push(node);
        Ok(())
    } else {
        bail!("Mapping is empty");
    }
}

type Stack<'a> = Vec<&'a MarkedYaml<'a>>;
type YamlMap<'a> = LinkedHashMap<MarkedYaml<'a>, MarkedYaml<'a>>;
type YamlSeq<'a> = Vec<MarkedYaml<'a>>;

#[derive(Error, Debug, Diagnostic)]
#[error("")]
#[diagnostic()]
pub struct FakeErr {
    #[source_code]
    src: NamedSource<String>,
    #[label("Current node")]
    span: SourceSpan,
}

struct PositionInParent {
    idx: usize,
    kvtype: KVType,
}

#[derive(Eq, PartialEq)]
enum KVType {
    Key,
    Value,
}

fn pos_in_parent<'a>(node: &'a MarkedYaml<'a>, parent: &'a MarkedYaml<'a>) -> PositionInParent {
    let span = node.span;
    let mut pos = PositionInParent {
        idx: 0,
        kvtype: KVType::Key,
    };
    match &parent.data {
        saphyr::YamlData::Sequence(seq) => {
            for (idx, sibling) in seq.iter().enumerate() {
                if sibling.span == span {
                    pos.idx = idx;
                    return pos;
                }
            }
            unreachable!();
        }
        saphyr::YamlData::Mapping(map) => {
            for (idx, (key, value)) in map.iter().enumerate() {
                pos.idx = idx;
                if key.span == span {
                    return pos;
                } else if value.span == span {
                    pos.kvtype = KVType::Value;
                    return pos;
                }
            }
            unreachable!();
        }
        _ => unreachable!(),
    }
}

enum Action {
    StepIn,
    StepInKey,
    StepInValue,
    Next,
    Prev,
    Fin,
    Stop,
}

fn read_action(io: &mut DefaultEditor) -> Action {
    loop {
        match io.readline(">> ") {
            Ok(line) => match line.as_str() {
                "q" | "quit" => return Action::Stop,
                "n" | "next" => return Action::Next,
                "p" | "prev" => return Action::Prev,
                "s" | "si" | "i" => return Action::StepIn,
                "sk" => return Action::StepInKey,
                "sv" => return Action::StepInValue,
                "fin" | "out" | "up" => return Action::Fin,
                _ => {}
            },
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => return Action::Stop,
            Err(e) => panic!("{e:?}"),
        }
    }
}
