mod crates

set default-list := true

# TODO: set-up with uv and Vespa container set-up

# Starts the development Vespa Docker service
[group("docker")]
[working-directory: "scripts"]
@start:
  uv run start --restart
