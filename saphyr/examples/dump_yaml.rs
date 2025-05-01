<<<<<<< HEAD
use saphyr::{MarkedYaml, YamlData};
||||||| 3143cd2
use saphyr::Yaml;
=======
use saphyr::{LoadableYamlNode, Yaml};
>>>>>>> master
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn print_indent(indent: usize) {
    for _ in 0..indent {
        print!("    ");
    }
}

<<<<<<< HEAD
fn dump_node(node: &MarkedYaml, indent: usize) {
    match node.data {
        YamlData::Array(ref v) => {
||||||| 3143cd2
fn dump_node(doc: &Yaml, indent: usize) {
    match *doc {
        Yaml::Array(ref v) => {
=======
fn dump_node(doc: &Yaml, indent: usize) {
    match *doc {
        Yaml::Sequence(ref v) => {
>>>>>>> master
            for x in v {
                dump_node(x, indent + 1);
            }
        }
<<<<<<< HEAD
        YamlData::Hash(ref h) => {
||||||| 3143cd2
        Yaml::Hash(ref h) => {
=======
        Yaml::Mapping(ref h) => {
>>>>>>> master
            for (k, v) in h {
                print_indent(indent);
                println!("{k:?}:");
                dump_node(v, indent + 1);
            }
        }
        _ => {
            print_indent(indent);
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut f = File::open(&args[1]).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let docs = MarkedYaml::load_from_str(&s).unwrap();
    for doc in &docs {
        println!("---");
        dump_node(doc, 0);
    }
}
