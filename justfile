start:
  @cd ./scripts && uv run start --restart

[group("test")]
integration:
  @cd ./crates/vespa && cargo test -- --include-ignored
