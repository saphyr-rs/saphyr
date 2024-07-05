before_commit:
  cargo fmt --check
  cargo clippy --release --all-targets -- -D warnings
  cargo clippy --all-targets -- -D warnings
  cargo build --release --all-targets
  cargo build --all-targets
  cargo test
  cargo test --release
  cargo test --doc
  cargo build --profile=release-lto --package gen_large_yaml --bin gen_large_yaml --manifest-path tools/gen_large_yaml/Cargo.toml
  RUSTDOCFLAGS="-D warnings" cargo doc --all-features

ethi_bench:
  cargo build --release --all-targets
  cd ../Yaml-rust && cargo build --release --all-targets
  cd ../serde-yaml/ && cargo build --release --all-targets
  cd ../libfyaml/build && ninja
  cargo bench_compare run_bench

ethi_build_dump:
  (cargo test 2>&1 >/dev/null || (cargo test && false))
  CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release --bin time_parse
  valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes ./target/release/time_parse ~/Projects/yaml-rust2/bench_yaml/strings_array.yaml

ethi_compare: ethi_build_dump
  cg_file=`\ls -1t callgrind.out.* | head -n1` && callgrind_annotate $cg_file --auto=no --threshold=99.99 > cg/WORK && rm $cg_file
  callgrind_differ `\ls cg/0*` cg/WORK --show percentagediff,ircount --sort-by=-first-ir -a
