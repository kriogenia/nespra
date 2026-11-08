"""Benchmarks pynespra against the Vespa started with `just start`.

Run it with `just bench` (optimized build, httpx as the baseline). It feeds its own
documents under its own namespace and removes them at the end.

It measures:
  1. decode: cbor2 vs orjson on the same response, no network. Decides the default format.
  2. latency: sequential requests, nespra (cbor and json) vs httpx + orjson and pyvespa.
  3. throughput: concurrent requests, threads for the sync clients, asyncio for the async ones.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import statistics
import time
import urllib.request
from collections.abc import Callable, Coroutine
from concurrent.futures import ThreadPoolExecutor
from typing import Any

import cbor2
import nespra
import orjson

NAMESPACE = "pynespra-bench"
BODY = "Statins are widely prescribed, and many studies look at the risk of cancer. " * 25


def call(url: str, method: str, path: str, body: dict | None = None) -> None:
    request = urllib.request.Request(
        f"{url}{path}",
        method=method,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"content-type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=30):
        pass


def feed(url: str, docs: int) -> None:
    for i in range(docs):
        fields = {"id": str(i), "title": f"Statin use and breast cancer {i}", "body": BODY}
        call(url, "POST", f"/document/v1/{NAMESPACE}/doc/docid/{i}", {"fields": fields})


def cleanup(url: str, docs: int) -> None:
    for i in range(docs):
        call(url, "DELETE", f"/document/v1/{NAMESPACE}/doc/docid/{i}")


def query(hits: int) -> dict[str, Any]:
    return {
        "yql": "select title, body from doc where userQuery()",
        "query": "statin",
        "ranking": "bm25",
        "hits": hits,
    }


def timed(fn: Callable[[], Any], times: int) -> list[float]:
    samples = []
    for _ in range(times):
        start = time.perf_counter()
        fn()
        samples.append(time.perf_counter() - start)
    return samples


def ms(seconds: float) -> str:
    return f"{seconds * 1000:8.3f}"


def latency_row(name: str, samples: list[float]) -> None:
    samples = sorted(samples)
    p95 = samples[int(len(samples) * 0.95) - 1]
    print(
        f"  {name:<22} median {ms(statistics.median(samples))} ms"
        f"   p95 {ms(p95)} ms   mean {ms(statistics.fmean(samples))} ms"
    )


def throughput_row(name: str, requests: int, seconds: float) -> None:
    print(f"  {name:<22} {requests / seconds:9.0f} req/s   ({seconds:.2f} s)")


def bench_decode(url: str, hits: int, repeat: int) -> None:
    print(f"\n[decode] hits={hits}")
    body = query(hits)
    for format, decoder in (("cbor", cbor2.loads), ("json", orjson.loads)):
        transport = nespra.Vespa(url, format=format)._transport  # type: ignore[arg-type]
        _, data = transport.request(orjson.dumps(body))
        samples = timed(lambda decoder=decoder, data=data: decoder(data), repeat)
        library = "cbor2" if format == "cbor" else "orjson"
        print(
            f"  {format + ' + ' + library:<22} {len(data):>8} bytes"
            f"   median {statistics.median(samples) * 1e6:9.1f} us"
        )


def bench_latency(url: str, hits: int, requests: int, httpx: Any, pyvespa: Any) -> None:
    print(f"\n[latency] sequential, hits={hits}, {requests} requests")
    body = query(hits)
    for format in ("cbor", "json"):
        client = nespra.Vespa(url, format=format)  # type: ignore[arg-type]
        client.search(body)
        latency_row(f"nespra {format}", timed(lambda client=client: client.search(body), requests))
    if httpx:
        with httpx.Client(timeout=30) as client:
            run = lambda: orjson.loads(client.post(f"{url}/search/", json=body).content)  # noqa: E731
            run()
            latency_row("httpx + orjson", timed(run, requests))
    if pyvespa:
        with pyvespa.syncio(connections=1) as session:
            run = lambda: session.query(body=body)  # noqa: E731
            run()
            latency_row("pyvespa", timed(run, requests))


def bench_threads(
    url: str, hits: int, requests: int, workers: int, httpx: Any, pyvespa: Any
) -> None:
    print(f"\n[throughput] {workers} threads, hits={hits}, {requests} requests")
    body = query(hits)

    def run(name: str, fn: Callable[[], Any]) -> None:
        fn()
        with ThreadPoolExecutor(workers) as pool:
            start = time.perf_counter()
            list(pool.map(lambda _: fn(), range(requests)))
            throughput_row(name, requests, time.perf_counter() - start)

    for format in ("cbor", "json"):
        client = nespra.Vespa(url, format=format)  # type: ignore[arg-type]
        run(f"nespra {format}", lambda client=client: client.search(body))
    if httpx:
        with httpx.Client(timeout=30) as client:
            run(
                "httpx + orjson",
                lambda: orjson.loads(client.post(f"{url}/search/", json=body).content),
            )
    if pyvespa:
        with pyvespa.syncio(connections=workers) as session:
            run("pyvespa", lambda: session.query(body=body))


async def bench_async(
    url: str, hits: int, requests: int, workers: int, httpx: Any, pyvespa: Any
) -> None:
    print(f"\n[throughput] asyncio, {workers} in flight, hits={hits}, {requests} requests")
    body = query(hits)

    async def run(name: str, fn: Callable[[], Coroutine[Any, Any, Any]]) -> None:
        await fn()
        limit = asyncio.Semaphore(workers)

        async def one() -> None:
            async with limit:
                await fn()

        start = time.perf_counter()
        await asyncio.gather(*(one() for _ in range(requests)))
        throughput_row(name, requests, time.perf_counter() - start)

    for format in ("cbor", "json"):
        client = nespra.AsyncVespa(url, format=format)  # type: ignore[arg-type]
        await run(f"nespra async {format}", lambda client=client: client.search(body))
    if httpx:
        async with httpx.AsyncClient(timeout=30) as client:

            async def fetch() -> Any:
                response = await client.post(f"{url}/search/", json=body)
                return orjson.loads(response.content)

            await run("httpx async + orjson", fetch)
    if pyvespa:
        async with pyvespa.asyncio(connections=workers) as session:
            await run("pyvespa async", lambda: session.query(body=body))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--url", default="http://localhost:8080")
    parser.add_argument("--requests", type=int, default=300, help="requests per measurement")
    parser.add_argument("--workers", type=int, default=16, help="concurrent requests")
    parser.add_argument("--hits", type=int, nargs="+", default=[10, 100])
    parser.add_argument("--docs", type=int, default=150, help="documents to feed")
    parser.add_argument("--no-feed", action="store_true", help="use the documents already there")
    args = parser.parse_args()

    try:
        import httpx
    except ImportError:
        httpx = None
        print("httpx is not installed, the baseline is skipped (`uv run --with httpx`)")

    try:
        from vespa.application import Vespa as PyVespa

        pyvespa = PyVespa(url=args.url)
    except ImportError:
        pyvespa = None
        print("pyvespa is not installed, it is skipped (`uv run --with pyvespa`)")

    if not args.no_feed:
        print(f"Feeding {args.docs} documents")
        feed(args.url, args.docs)
    try:
        for hits in args.hits:
            bench_decode(args.url, hits, repeat=2000)
            bench_latency(args.url, hits, args.requests, httpx, pyvespa)
            bench_threads(args.url, hits, args.requests, args.workers, httpx, pyvespa)
            asyncio.run(bench_async(args.url, hits, args.requests, args.workers, httpx, pyvespa))
    finally:
        if not args.no_feed:
            cleanup(args.url, args.docs)


if __name__ == "__main__":
    main()
