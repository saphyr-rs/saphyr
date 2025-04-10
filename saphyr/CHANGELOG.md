# Changelog

## Upcoming

**Breaking Changes**:

- Allow `Yaml` to borrow from the input.
- All indexing traits now panic if the key is not found or the YAML variant is
  incorrect. This helps in making the behavior of `[]` more consistent across
  all operations.
- Use `Mapping` instead of `Hash` to refer to YAML mappings.
- Use `Sequence` instead of `Array` to refer to YAML sequences.
  Methods to access sequences using `vec` instead of `array` still exist.
  Another method using `sequence` has been added.
- `as_f64` -> `as_floating_point`, `as_i64` -> `as_integer`
- Reworked `Yaml` and `YamlData`
  - They now have a `Value` and a `Representation` variants for scalars.
    `Representation` holds the raw characters from the input (pre-parsing)
    while `Value` holds the parsed value. In `foo: 3`, `Representation` would
    hold `"foo"` and `"3"` (both as strings) while `Value` would hold `"foo"`
    and `3`.
  - The idea behind this is to allow lazy-parsing of scalar nodes and to give
    more control about key duplication detection in mappings (e.g.: is `{ 0xB:
    ~, 11: ~ }` considered a duplicate because `0xB == 11`?).
- Rename `from_str` to `value_from_str` to better highlight it:
  - Doesn't load a YAML document
  - Doesn't always load into a YAML string
- `load_from_*` functions now belong to the `LoadableYamlNode` trait to avoid
  implementing them in each YAML node type. It is now required to import
  `LoadableYamlNode` to use these functions (LSPs should have a fixit for it).

**Features**:
- Add the following convenience methods to the YAML objects:
  - `contains_mapping_key`
  - `as_mapping_get`
  - `as_mapping_get_mut`
- Add many more conversion methods (`as_*`, `as_*_mut`, `into_*`, ...).
- Use
  [`ordered-float`](https://docs.rs/ordered-float/latest/ordered_float/struct.OrderedFloat.html)
  to store floating point values in scalars. This allows using floating point
  values in mappings, with the caveats listed in the crate description (#18).
  The `OrderedFloat`s are kept within the `Scalar` object and conversion
  methods do not expose them.
- Add `YamlDataOwned`, an owned version of `YamlData` for when lifetimes are
  not required.
- Add `MarkedYamlOwned`, an owned version of `MarkedYaml` for when lifetimes
  are not required.
- Add `YamlOwned`, an owned version of `Yaml` which corresponds to what `Yaml`
  was prior to this version.

## v0.0.3

Skipping version `v0.0.2` to align this crate's version with that of
`saphyr-parser`.

**Breaking Changes**:

- Move `load_from_*` methods out of the `YamlLoader`. Now, `YamlLoader` gained
  a generic parameter. Moving those functions out of it spares having to
  manually specify the generic in `YamlLoader::<Yaml>::load_from_str`.
  Manipulating the `YamlLoader` directly was not common.
- Make `LoadError` `Clone` by storing an `Arc<std::io::Error>` instead of the
  error directly.

**Features**:

- ([#19](https://github.com/Ethiraric/yaml-rust2/pull/19)) `Yaml` now
  implements `IndexMut<usize>` and `IndexMut<&'a str>`. These functions may not
  return a mutable reference to a `BAD_VALUE`. Instead, `index_mut()` will
  panic if either:
  * The index is out of range, as per `IndexMut`'s requirements
  * The inner `Yaml` variant doesn't match `Yaml::Array` for `usize` or
    `Yaml::Hash` for `&'a str`

- Use cargo features
  
  This allows for more fine-grained control over MSRV and to completely remove
  debug code from the library when it is consumed.

  The `encoding` feature, governing the `YamlDecoder`, has been enabled by
  default. Users of `@davvid`'s fork of `yaml-rust` or of `yaml-rust2` might
  already use this. Users of the original `yaml-rust` crate may freely disable
  this feature (`cargo <...> --no-default-features`) and lower MSRV to 1.65.0.

- Load with metadata

  The `YamlLoader` now supports adding metadata alongside the nodes. For now,
  the only one supported is the `Marker`, pointing to the position in the input
  stream of the start of the node.

  This feature is extensible and should allow (later) to add comments.

**Fixes**:

- 1fc4692: Fix trailing newlines when emitting multiline strings.

# Older `yaml-rust2` changelgos
## v0.8.0

**Breaking Changes**:

- The `encoding` library has been replaced with `encoding_rs`. If you use the
`trap` of `YamlDecoder`, this change will make your code not compile.
An additional enum `YamlDecoderTrap` has been added to abstract the
underlying library and avoid breaking changes in the future. This
additionally lifts the `encoding` dependency on _your_ project if you were
using that feature.
  - The signature of the function for `YamlDecoderTrap::Call` has changed:
  - The `encoding::types::DecoderTrap` has been replaced with `YamlDecoderTrap`.
    ```rust
    // Before, with `encoding::types::DecoderTrap::Call`
    fn(_: &mut encoding::RawDecoder, _: &[u8], _: &mut encoding::StringWriter) -> bool;
    // Now, with `YamlDecoderTrap::Call`
    fn(_: u8, _: u8, _: &[u8], _: &mut String) -> ControlFlow<Cow<'static str>>;
    ```
    Please refer to the `YamlDecoderTrapFn` documentation for more details.

**Features**:

- Tags can now be retained across documents by calling `keep_tags(true)` on a
`Parser` before loading documents.
([#10](https://github.com/Ethiraric/yaml-rust2/issues/10)
([#12](https://github.com/Ethiraric/yaml-rust2/pull/12))

- `YamlLoader` structs now have a `documents()` method that returns the parsed
documents associated with a loader.

- `Parser::new_from_str(&str)` and `YamlLoader::load_from_parser(&Parser)` were added.

**Development**:

- Linguist attributes were added for the `tests/*.rs.inc` files to prevent github from
classifying them as C++ files.

## v0.7.0

**Features**:

- Multi-line strings are now
[emitted using block scalars](https://github.com/chyh1990/yaml-rust/pull/136).

- Error messages now contain a byte offset to aid debugging.
([#176](https://github.com/chyh1990/yaml-rust/pull/176))

- Yaml now has `or` and `borrowed_or` methods.
([#179](https://github.com/chyh1990/yaml-rust/pull/179))

- `Yaml::load_from_bytes()` is now available.
([#156](https://github.com/chyh1990/yaml-rust/pull/156))

- The parser and scanner now return Err() instead of calling panic.

**Development**:

- The documentation was updated to include a security note mentioning that
yaml-rust is safe because it does not interpret types.
([#195](https://github.com/chyh1990/yaml-rust/pull/195))

- Updated to quickcheck 1.0.
([#188](https://github.com/chyh1990/yaml-rust/pull/188))

- `hashlink` is [now used](https://github.com/chyh1990/yaml-rust/pull/157)
instead of `linked_hash_map`.

## v0.6.0

**Development**:

- `is_xxx` functions were moved into the private `char_traits` module.

- Benchmarking tools were added.

- Performance was improved.

## v0.5.0

- The parser now supports tag directives.
([#35](https://github.com/chyh1990/yaml-rust/issues/35)

- The `info` field has been exposed via a new `Yaml::info()` API method.
([#190](https://github.com/chyh1990/yaml-rust/pull/190))
