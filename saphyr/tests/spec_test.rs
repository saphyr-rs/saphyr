use saphyr::{LoadableYamlNode, Mapping, Scalar, Yaml, YamlEmitter};

#[test]
fn test_mapvec_legal() {
    // Emitting a `map<map<seq<_>>, _>` should result in legal yaml that
    // we can parse.

    let key = vec![
        Yaml::Value(Scalar::Integer(1)),
        Yaml::Value(Scalar::Integer(2)),
        Yaml::Value(Scalar::Integer(3)),
    ];

    let mut keyhash = Mapping::new();
    keyhash.insert(
        Yaml::Value(Scalar::String("key".into())),
        Yaml::Sequence(key),
    );

    let val = vec![
        Yaml::Value(Scalar::Integer(4)),
        Yaml::Value(Scalar::Integer(5)),
        Yaml::Value(Scalar::Integer(6)),
    ];

    let mut hash = Mapping::new();
    hash.insert(Yaml::Mapping(keyhash), Yaml::Sequence(val));

    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(&Yaml::Mapping(hash)).unwrap();
    }

    // At this point, we are tempted to naively render like this:
    //
    //  ```yaml
    //  ---
    //  {key:
    //      - 1
    //      - 2
    //      - 3}:
    //    - 4
    //    - 5
    //    - 6
    //  ```
    //
    // However, this doesn't work, because the key sequence [1, 2, 3] is
    // rendered in block mode, which is not legal (as far as I can tell)
    // inside the flow mode of the key. We need to either fully render
    // everything that's in a key in flow mode (which may make for some
    // long lines), or use the explicit map identifier '?':
    //
    //  ```yaml
    //  ---
    //  ?
    //    key:
    //      - 1
    //      - 2
    //      - 3
    //  :
    //    - 4
    //    - 5
    //    - 6
    //  ```

    Yaml::load_from_str(&out_str).unwrap();
}
