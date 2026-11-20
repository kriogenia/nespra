# nespra_admin

Unofficial async Rust client for the operational APIs of [Vespa](https://vespa.ai), the
open-source search and big data serving engine. It complements [`nespra`](https://crates.io/crates/nespra),
which covers the data APIs, and can be used on its own or through the `admin` feature of `nespra`.

> **Disclaimer:** nespra is an independent community project. It is not affiliated with, endorsed by, or sponsored by Vespa.ai. Vespa and related names are the property of their respective owners.

It currently supports:

- `/state/v1/health`: health of a node.
- `/ApplicationStatus`: Vespa version and generation of the application running in a container.

## Usage

```rust
use nespra_admin::Admin;

let admin = Admin::new("http://localhost:8080")?;

let health = admin.health().await?;
println!("The node is {}", health.status.code);

let status = admin.application_status().await?;
println!("Vespa {} (generation {:?})", status.vespa_version, status.generation);
```

Vespa Cloud endpoints are reached with the data plane certificate or a token:

```rust
let admin = Admin::builder("https://my-endpoint.vespa-app.cloud")
    .client_pem(&cert, &key)
    .build()?;
```

The same `reqwest::Client` can be shared with `nespra` with `Admin::with_client` and
`nespra::Vespa::with_client`, so both reuse the connections and the TLS configuration.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kriogenia/nespra/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/kriogenia/nespra/blob/main/LICENSE-MIT) at your option.
