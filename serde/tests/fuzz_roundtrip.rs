//! Randomized round trips: whatever we serialize must deserialize to the same value.

use std::collections::BTreeMap;

use saphyr_serde::{from_str, to_string};
use serde::{Deserialize, Serialize};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn string(&mut self) -> String {
        const PIECES: &[&str] = &[
            "a", "b", "z", "0", "1", "9", " ", "  ", "\n", "\n\n", "\t", "\r", ":", ": ", "-",
            "- ", "?", "? ", "#", " #", "'", "\"", "\\", "[", "]", "{", "}", ",", "&", "*", "!",
            "|", ">", "%", "@", "`", ".", "...", "---", "~", "null", "true", "false", "yes", "no",
            "<<", "0x", "0o", "e", "E", "+", "inf", "nan", "é", "日", "🦀", "\u{0}", "\u{7f}",
            "\u{85}", "\u{a0}", "\u{2028}", "\u{2029}", "\u{feff}", "\u{1b}", "=", "%YAML",
            "!!str",
        ];
        let len = self.below(8);
        (0..len).map(|_| PIECES[self.below(PIECES.len())]).collect()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Pairs(Vec<(String, Value)>),
}

fn value(rng: &mut Rng, depth: usize) -> Value {
    match rng.below(if depth > 4 { 5 } else { 8 }) {
        0 => Value::Null,
        1 => Value::Bool(rng.next() % 2 == 0),
        2 => Value::Int(rng.next() as i64),
        3 => Value::Float(f64::from_bits(rng.next()).clamp(-1e300, 1e300)),
        4 => Value::Str(rng.string()),
        5 => Value::List((0..rng.below(4)).map(|_| value(rng, depth + 1)).collect()),
        6 => Value::Map(
            (0..rng.below(4))
                .map(|_| (rng.string(), value(rng, depth + 1)))
                .collect(),
        ),
        _ => Value::Pairs(
            (0..rng.below(4))
                .map(|_| (rng.string(), value(rng, depth + 1)))
                .collect(),
        ),
    }
}

fn float_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => {
            a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
        }
        (Value::List(a), Value::List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| float_eq(a, b))
        }
        (Value::Map(a), Value::Map(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ka, a), (kb, b))| ka == kb && float_eq(a, b))
        }
        (Value::Pairs(a), Value::Pairs(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ka, a), (kb, b))| ka == kb && float_eq(a, b))
        }
        (a, b) => a == b,
    }
}

#[test]
fn random_strings() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..20_000 {
        let s = rng.string();
        let yaml = to_string(&s).unwrap();
        let back: String = from_str(&yaml).unwrap_or_else(|e| panic!("{s:?}: {e}\n{yaml}"));
        assert_eq!(back, s, "{yaml}");
    }
}

#[test]
fn random_values() {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..5_000 {
        let v = value(&mut rng, 0);
        let yaml = to_string(&v).unwrap();
        let back: Value = from_str(&yaml).unwrap_or_else(|e| panic!("{v:?}: {e}\n{yaml}"));
        assert!(float_eq(&back, &v), "{v:?}\n{back:?}\n{yaml}");
    }
}

#[test]
fn rng_is_random() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let strings: std::collections::HashSet<String> = (0..1000).map(|_| rng.string()).collect();
    assert!(strings.len() > 500);
}
