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
response = vespa.search(
    {
        "yql": "select title, body from doc where userQuery()",
        "query": "Is statin use connected to breast cancer?",
    }
)
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

## Performance

Numbers from [`benches/bench.py`](https://github.com/kriogenia/nespra/blob/main/crates/pynespra/benches/bench.py) (`just bench` in `crates/pynespra`), against a single local Vespa container with small text documents, 300 requests per measurement and 16 concurrent requests. They compare pynespra with [pyvespa](https://github.com/vespa-engine/pyvespa) 1.2.4 and a plain [httpx](https://www.python-httpx.org) client decoding with orjson. Treat them as indicative: run the script against your own documents and queries, as latency is dominated by Vespa itself and the gap depends on the size of the responses.

**10 hits per query** (about 22 KB responses):

| Client | Sequential (median) | Threads (16) | Asyncio (16 in flight) |
|---|---:|---:|---:|
| pynespra (CBOR) | 1.50 ms | 6,070 req/s | 6,020 req/s |
| pynespra (JSON) | 1.36 ms | 5,550 req/s | 6,060 req/s |
| pyvespa | 1.48 ms | 4,800 req/s | 4,410 req/s |
| httpx + orjson | 1.86 ms | 1,300 req/s | 640 req/s |

**100 hits per query** (about 228 KB responses):

| Client | Sequential (median) | Threads (16) | Asyncio (16 in flight) |
|---|---:|---:|---:|
| pynespra (CBOR) | 9.19 ms | 870 req/s | 930 req/s |
| pynespra (JSON) | 9.15 ms | 920 req/s | 960 req/s |
| pyvespa | 9.46 ms | 815 req/s | 890 req/s |
| httpx + orjson | 9.77 ms | 740 req/s | 550 req/s |

What to take from it:

- Under concurrency pynespra is about 10 to 40% ahead of pyvespa on small responses, and 5 to 10% ahead on large ones. pyvespa is also built on a Rust HTTP client, so it is much closer than a pure Python stack, which pynespra beats by several times on small responses.
- Sequentially all the clients are within a few percent of each other: the time is spent in Vespa.
- CBOR is not faster than JSON in these runs. Decoding the 22 KB response takes about 8.5 us with cbor2 and 6.4 us with orjson, and the CBOR payload is only about 1% smaller because the documents are mostly text. CBOR is the default as it is smaller for documents with numeric fields or tensors, use `format="json"` if your responses look like the benchmark ones.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
