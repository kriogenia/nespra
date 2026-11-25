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
  just crates pynespra lint
  just crates pynespra test

# Installs the git hooks (prek, https://prek.j178.dev) defined in .pre-commit-config.yaml
[group("dev")]
@hooks:
  prek install

# Starts the development Vespa Docker service
[group("docker")]
[working-directory: "scripts"]
@start:
  uv run start --restart

# Stops the development Vespa Docker service
[group("docker")]
@stop:
  docker stop sample

# Feeds 100 documents of the NFCorpus dataset to the development Vespa
[group("docker")]
[working-directory: "scripts"]
@feed:
  uv run feed

# Runs a sample query against the development Vespa and prints its JSON response
[group("docker")]
[working-directory: "scripts"]
@query:
  uv run query

# Opens lazyvespa on the development Vespa, or with the given arguments: `just tui --target prod`
[group("lazyvespa")]
tui *args:
  cargo run -q -p lazyvespa -- {{ if args == "" { "--url http://localhost:8080" } else { args } }}

# Starts the development Vespa, feeds it and opens lazyvespa on it, try `select * from doc where userQuery()` with the parameter query=cancer
[group("lazyvespa")]
lazy: start feed tui
