#[macro_use]
extern crate quickcheck;

use quickcheck::TestResult;

use saphyr::{Scalar, Yaml, YamlEmitter};

quickcheck! {
    fn test_check_weird_keys(xs: Vec<String>) -> TestResult {
        let mut out_str = String::new();
        let input = Yaml::Sequence(xs.into_iter().map(|x|Yaml::Value(Scalar::String(x.into()))).collect());
        {
            let mut emitter = YamlEmitter::new(&mut out_str);
            emitter.dump(&input).unwrap();
        }
        match Yaml::load_from_str(&out_str) {
            Ok(output) => TestResult::from_bool(output.len() == 1 && input == output[0]),
            Err(err) => TestResult::error(err.to_string()),
        }
    }
}
