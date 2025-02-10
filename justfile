before_commit:
  cargo fmt --check
  cargo clippy --release --all-targets -- -D warnings
  cargo clippy --all-targets -- -D warnings
  cargo build --release --all-targets
  cargo build --all-targets
  cargo test
  cargo test --release
  cargo test --doc
  cargo build --release --package gen_large_yaml --bin gen_large_yaml --manifest-path bench/tools/gen_large_yaml/Cargo.toml
  cargo build --release --package bench_compare --bin bench_compare --manifest-path bench/tools/bench_compare/Cargo.toml
  RUSTDOCFLAGS="-D warnings" cargo doc --all-features --document-private-items

fuzz:
  CARGO_PROFILE_RELEASE_LTO=false cargo +nightly fuzz run parse
