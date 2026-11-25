# lazyvespa

Terminal UI to query and operate [Vespa](https://vespa.ai) deployments, inspired by
lazygit, lazydocker and k9s. It works with self-hosted Vespa and with Vespa Cloud.

> **Disclaimer:** lazyvespa is an independent community project. It is not affiliated with, endorsed by, or sponsored by Vespa.ai. Vespa and related names are the property of their respective owners.

```
 lazyvespa  local  self-hosted  http://localhost:8080           Vespa 8.763.13  ● up 3 ms
┌ Query ─────────────────────────────────────────┐┌ Parameters ──────────────────────────┐
│select * from doc where userQuery()             ││query            cancer               │
│                                                ││hits             5                    │
│                                                ││ranking.profile  bm25                 │
│                                                ││                                      │
└────────────────────────────────────────────────┘└──────────────────────────────────────┘
┌ Results ───────────────────────────────────────────────────────────────────────────────┐
│5 hits · 9 ms round trip                                                                │
│#    relevance  id                      │{                                              │
│1    12.4407    id:tutorial:doc::MED-14 │  "id": "id:tutorial:doc::MED-14",             │
│2    11.9512    id:tutorial:doc::MED-10 │  "relevance": 12.44070815,                    │
│3    10.2283    id:tutorial:doc::MED-12 │  "fields": {                                  │
└────────────────────────────────────────────────────────────────────────────────────────┘
 j next · k previous · Ctrl+D scroll down · Ctrl+U scroll up · r raw · Tab query · ? help
```

## Features

- Write YQL queries with their parameters and run them, then browse the hits and their
  fields, or the raw JSON response.
- History of the queries run against each target, and queries saved by name.
- Several targets, switched without leaving the application.
- Health of the target in the status bar, checked every 5 seconds, and an info panel
  with the Vespa version and the deployment of the application.

## Try it

With [just](https://just.systems), [uv](https://docs.astral.sh/uv) and Docker, from the
root of the repository:

```sh
just try
```

It starts a Vespa container with a sample application, feeds it 100 documents and opens
lazyvespa on it. `Ctrl+R` runs the query in the editor. To search the documents, press `i`
and replace the YQL with `select * from doc where userQuery()`, press `Esc`, then `Tab` to
go to the parameters, and fill `query` with `cancer` and `ranking.profile` with `bm25`
(`j`/`k` to move, `i` to edit, `Esc` to stop). `Ctrl+R` runs it. Once Vespa is running, `just tui` opens lazyvespa again, and `just stop` stops the
container.

## Install

lazyvespa is not published to crates.io yet. Install it from the repository:

```sh
cargo install --locked --git https://github.com/kriogenia/nespra lazyvespa
```

## Usage

```sh
lazyvespa                            # the default target of the configuration
lazyvespa --target prod              # a target of the configuration
lazyvespa --url http://localhost:8080  # any endpoint, without configuration
lazyvespa --config ./config.toml     # another configuration file
```

## Configuration

The targets are configured in `~/.config/lazyvespa/config.toml`, or
`$XDG_CONFIG_HOME/lazyvespa/config.toml`, on every platform. See
[config.example.toml](config.example.toml) for a complete example.

```toml
default_target = "local"

[targets.local]
endpoint = "http://localhost:8080"

[targets.prod]
endpoint = "https://abcdef.ghijkl.z.vespa-app.cloud"
cloud = { tenant = "acme", application = "search" }
auth = { mtls = {} }
```

Each target has:

- `endpoint`: URL of a Vespa container.
- `cloud`: the `tenant`, `application` and `instance` (`default` if not set) of a Vespa
  Cloud deployment. Leave it out for self-hosted Vespa.
- `auth`: how to authenticate, none by default.
  - `{ mtls = { cert = "...", key = "..." } }` uses a client certificate. On Vespa Cloud,
    `{ mtls = {} }` uses the data plane certificate created by `vespa auth cert`, found in
    `~/.vespa/<tenant>.<application>.<instance>/` (or under `$VESPA_CLI_HOME`).
  - `{ token = { env = "VESPA_TOKEN" } }` sends the token of the environment variable, so
    the token is never written in the file.

## Keys

Press `?` to see the keys available at any moment. The footer shows the most useful ones.

| Key | Action |
|---|---|
| `Ctrl+R`, `F5` | Run the query |
| `Tab` | Move between the YQL, the parameters and the results |
| `i`, `Enter` | Edit the YQL or the highlighted parameter, `Esc` to stop |
| `a` / `d` | Add / delete a parameter |
| `j` `k`, `↓` `↑` | Move in lists |
| `Ctrl+D` `Ctrl+U`, `PgDn` `PgUp` | Scroll the fields of the hit |
| `r` | Toggle the raw JSON response |
| `T` | Pick a target |
| `H` | Pick a query from the history of the target |
| `s` | Save the query under a name |
| `S` | Pick a saved query, `d` to delete it |
| `I` | Info about the target: health, Vespa version and deployment |
| `:` | Command palette |
| `q`, `Ctrl+C` | Quit |

The command palette accepts `ctx [target]`, `history`, `save <name>`, `saved`, `info`,
`help` and `quit`.

## Data

The history and the saved queries are kept in `~/.local/share/lazyvespa/history.json`
and `saved.json`, or in `$XDG_DATA_HOME/lazyvespa/`. The history keeps the last 500
queries of each target. When one of these files is not valid, lazyvespa shows the error
and does not save over it.
