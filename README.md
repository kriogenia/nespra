# nespra

[![CI](https://github.com/kriogenia/nespra/actions/workflows/ci.yml/badge.svg)](https://github.com/kriogenia/nespra/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/vespa-client.svg)](https://crates.io/crates/vespa-client)
[![Docs.rs](https://img.shields.io/docsrs/vespa-client)](https://docs.rs/vespa-client)
[![License](https://img.shields.io/crates/l/vespa-client.svg)](#license)


## Releasing

Releases are automated with [release-plz](https://release-plz.dev):

1. Merge PRs into `main` using [Conventional Commit](https://www.conventionalcommits.org) titles (`feat:`, `fix:`, `perf:`...). The title of a squash-merged PR becomes the commit message, and CI enforces its format.
2. After commits land on `main`, release-plz opens or updates a release PR that bumps the version in `Cargo.toml` and updates `crates/client/CHANGELOG.md`.
3. Review and merge that PR. release-plz then tags the commit, creates the GitHub release and publishes the crate to crates.io.

The commit types drive the version bump: `feat` is a minor bump, `fix` and other changes are a patch bump, and a `!` after the type or a `BREAKING CHANGE:` footer is a major bump (minor while below 1.0).

Run `just ci` to execute the same quality checks as CI locally.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
