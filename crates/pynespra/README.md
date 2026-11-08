# pynespra

Unofficial Python client for [Vespa](https://vespa.ai), the open-source search and big data serving engine. It is a thin binding of the [`nespra`](https://crates.io/crates/nespra) Rust client: the HTTP work is done in Rust (HTTP/2, rustls, mTLS, gzip) and the responses come back as plain `dict`s.

> **Disclaimer:** pynespra is an independent community project. It is not affiliated with, endorsed by, or sponsored by Vespa.ai.

```sh
pip install pynespra
```

Requires Python 3.10 or later. The package is imported as `nespra`.

## Usage

```python
from nespra import Vespa

vespa = Vespa("http://localhost:8080")
response = vespa.search({
    "yql": "select title, body from doc where userQuery()",
    "query": "Is statin use connected to breast cancer?",
})
print(response["root"]["children"])
```

`AsyncVespa` has the same constructor and an awaitable `search`:

```python
from nespra import AsyncVespa

vespa = AsyncVespa("https://my.endpoint", bearer_token="...", timeout=10)
response = await vespa.search({"yql": "select * from doc where true"})
```

Responses are decoded from CBOR by default, pass `format="json"` to use JSON instead. Requests are always sent as JSON.

### Errors

Every error derives from `nespra.NespraError` and has a `retryable` attribute: `RequestError` (connection and timeouts), `StatusError` (unexpected HTTP status), `VespaError` (errors inside the Vespa response) and `DecodeError`. Invalid configuration raises `BuildError`.

### Retries

Nothing is retried by default. Pass any [tenacity](https://tenacity.readthedocs.io) object to the client, or to a single `search` call:

```python
from tenacity import Retrying, retry_if_exception, stop_after_attempt, wait_random_exponential

vespa = Vespa(
    "http://localhost:8080",
    retry=Retrying(
        retry=retry_if_exception(lambda e: e.retryable),
        stop=stop_after_attempt(3),
        wait=wait_random_exponential(max=5),
        reraise=True,
    ),
)
```

`AsyncVespa` takes an `AsyncRetrying` instead.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
