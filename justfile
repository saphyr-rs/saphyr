before_commit:
  cargo fmt --check
  cargo clippy --release --all-targets -- -D warnings
  cargo clippy --all-targets -- -D warnings
  cargo build --release --all-targets
  cargo build --all-targets
  cargo test
  cargo test --release
  cargo test --doc
  RUSTDOCFLAGS="-D warnings" cargo doc --all-features
