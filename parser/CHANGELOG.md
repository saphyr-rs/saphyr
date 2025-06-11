# Changelog

## Upcoming

## v0.0.6

**Fixes**:
- Fix emitting of tags with empty handles. `!tag` no longer emits as `!!tag`.

## v0.0.5

**Breaking Changes**:

- Emit `Cow<'input, Tag>` instead of `Tag` to avoid copies.

**Fixes**:

- 8ef76dcc: Fix `Marker`s for `null` and empty values.
- Fix `Span`s for collections to correctly mark the end of the collection.

**Changes**

- Exclude `yaml-test-suite` from the Cargo package.
- Bump `libtest-mimic` to `0.8.1`.

## v0.0.4

**Breaking Changes**:

- Allow events to borrow from the input.
- Rename `TScalarStyle` to `ScalarStyle`.

## v0.0.3

**Breaking Changes**:

- 926fdfb: Events now use spans rather than markers, allowing for tracking both
  the beginning and the end of scalars.
- 6c57b5b: Add a boolean to `DocumentStart` to know whether the start was
  explicit (`---`) or implicit.

**Features**:

- Add an `Input` interface to prepare the ground to future input-specific.
  optimizations (such as returning `Cow`'d strings when possible). This also
  potentially allows for user-defined optimizations.
- Add `Parser::new_from_iter` to load from an iterator. This automatically
  wraps using `BufferedInput`, which implements the new `Input` trait the
  `Parser` needs.

**Fixes**:

- 750c992: Add support for nested implicit flow mappings.
- 11cffc6: Fix error with deeply indented block scalars.
- d3b9641: Fix assertion that could erroneously trigger with multibyte
  characters.
- 95fe3fe: Fix parse errors when `---` appeared in the middle of plain scalars.
- 3358629: Fix infinite loop with `...` in plain scalars in flow contexts.
- Fix panics on other various erroneous inputs found while fuzzing.

**Internal changes**:

- Run all tests with both `Input` backends
- #15: Add fuzzing

## v0.0.2

This release does not provide much but is needed for the `saphyr` library to
depend on the new features.

**Breaking Changes**:

**Features**:
- Add `Marker::default()`
- Rework string handling in `ScanError`

**Fixes**:
- [yaml-rust2 #21](https://github.com/Ethiraric/yaml-rust2/issues/21#issuecomment-2053513507)
  Fix parser failing when a comment immediately follows a tag.

**Internal changes**:
- Various readability improvements and code cleanups
