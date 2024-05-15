use saphyr::{Hash, Yaml, YamlEmitter, YamlLoader};

#[test]
fn test_mapvec_legal() {
    // Emitting a `map<map<seq<_>>, _>` should result in legal yaml that
    // we can parse.

    let key = vec![
        Yaml::Integer(1, None),
        Yaml::Integer(2, None),
        Yaml::Integer(3, None),
    ];

    let mut keyhash = Hash::new();
    keyhash.insert(Yaml::String("key".into(), None), Yaml::Array(key, None));

    let val = vec![
        Yaml::Integer(4, None),
        Yaml::Integer(5, None),
        Yaml::Integer(6, None),
    ];

    let mut hash = Hash::new();
    hash.insert(Yaml::Hash(keyhash, None), Yaml::Array(val, None));

    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(&Yaml::Hash(hash, None)).unwrap();
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

    YamlLoader::load_from_str(&out_str).unwrap();
}
