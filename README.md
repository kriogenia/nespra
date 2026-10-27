# nespra

[![CI](https://github.com/kriogenia/nespra/actions/workflows/ci.yml/badge.svg)](https://github.com/kriogenia/nespra/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/nespra.svg)](https://crates.io/crates/nespra)
[![Docs.rs](https://img.shields.io/docsrs/nespra)](https://docs.rs/nespra)
[![License](https://img.shields.io/crates/l/nespra.svg)](#license)

Unofficial async Rust client for [Vespa](https://vespa.ai), the open-source search and big data serving engine. It supports the search API, with responses in CBOR (default) or JSON.

> **Disclaimer:** nespra is an independent community project. It is not affiliated with, endorsed by, or sponsored by Vespa.ai. Vespa and related names are the property of their respective owners.

## Usage

```rust
use nespra::{Vespa, search::Response};
use serde_json::{Value, json};

let vespa = Vespa::new("http://localhost:8080")?;
let response: Response<Value> = vespa
    .search(&json!({
        "yql": "select title, body from doc where userQuery()",
        "query": "Is statin use connected to breast cancer?",
    }))
    .await?;
```

## Releasing

Releases are automated with [release-plz](https://release-plz.dev):

1. Merge PRs into `main` using [Conventional Commit](https://www.conventionalcommits.org) titles (`feat:`, `fix:`, `perf:`...). The title of a squash-merged PR becomes the commit message, and CI enforces its format.
2. After commits land on `main`, release-plz opens or updates a release PR that bumps the version in `Cargo.toml` and updates `crates/client/CHANGELOG.md`.
3. Review and merge that PR. release-plz then tags the commit, creates the GitHub release and publishes the crate to crates.io.

The commit types drive the version bump: `feat` is a minor bump, `fix` and other changes are a patch bump, and a `!` after the type or a `BREAKING CHANGE:` footer is a major bump (minor while below 1.0).

Run `just ci` to execute the same quality checks as CI locally.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kriogenia/nespra/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/kriogenia/nespra/blob/main/LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
