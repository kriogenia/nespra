mod crates

set default-list := true

# Runs the same checks as the CI quality gates (no Vespa required)
[group("test")]
@ci:
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
  cargo build --workspace --examples

# TODO: set-up with uv and Vespa container set-up

# Starts the development Vespa Docker service
[group("docker")]
[working-directory: "scripts"]
@start:
  uv run start --restart
