"""Tests against a real Vespa, run with `just integration` (they need Docker).

They feed their own documents into the `doc` schema of the sample application started
by `just start`, under their own namespace, and remove them afterwards. Use `VESPA_URL`
to point to another instance.
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from collections.abc import Iterator

import pytest
from nespra import AsyncVespa, StatusError, Vespa, VespaError

pytestmark = pytest.mark.integration

URL = os.environ.get("VESPA_URL", "http://localhost:8080")
NAMESPACE = "pynespra"
DOCS = 25
QUERY = {
    "yql": "select title, body from doc where userQuery()",
    "query": "statin",
    "ranking": "bm25",
}


def call(method: str, path: str, body: dict | None = None) -> None:
    request = urllib.request.Request(
        f"{URL}{path}",
        method=method,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"content-type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=10):
        pass


def doc_path(i: int) -> str:
    return f"/document/v1/{NAMESPACE}/doc/docid/{i}"


@pytest.fixture(scope="module", autouse=True)
def documents() -> Iterator[None]:
    try:
        call("GET", "/state/v1/health")
    except (urllib.error.URLError, OSError) as e:
        pytest.fail(f"Vespa is not reachable at {URL}, start it with `just start`: {e}")

    for i in range(DOCS):
        fields = {
            "id": str(i),
            "title": f"Statin use and breast cancer {i}",
            "body": f"Statins are widely prescribed, study number {i} looks at the risk.",
        }
        call("POST", doc_path(i), {"fields": fields})
    yield
    for i in range(DOCS):
        call("DELETE", doc_path(i))


@pytest.fixture(params=["cbor", "json"])
def format(request: pytest.FixtureRequest) -> str:
    return str(request.param)


def test_search(format: str) -> None:
    response = Vespa(URL, format=format).search({**QUERY, "hits": 5})  # type: ignore[arg-type]

    children = response["root"]["children"]
    assert len(children) == 5
    assert all("statin" in c["fields"]["title"].lower() for c in children)
    assert response["root"]["fields"]["totalCount"] >= DOCS


async def test_async_search(format: str) -> None:
    response = await AsyncVespa(URL, format=format).search({**QUERY, "hits": 5})  # type: ignore[arg-type]

    assert len(response["root"]["children"]) == 5


def test_both_formats_return_the_same_response() -> None:
    body = {**QUERY, "hits": 10}
    cbor = Vespa(URL, format="cbor").search(body)
    json_ = Vespa(URL, format="json").search(body)

    def hits(response: dict) -> list:
        return [(c["id"], c["fields"]) for c in response["root"]["children"]]

    assert hits(cbor) == hits(json_)
    assert cbor["root"]["fields"] == json_["root"]["fields"]


async def test_sync_and_async_return_the_same_response() -> None:
    body = {**QUERY, "hits": 10}
    sync = Vespa(URL).search(body)
    async_ = await AsyncVespa(URL).search(body)

    assert sync["root"]["children"] == async_["root"]["children"]


@pytest.mark.parametrize("http_version", ["HTTP/1.1", "HTTP/2"])
def test_http_versions(http_version: str) -> None:
    response = Vespa(URL, http_version=http_version).search({**QUERY, "hits": 1})  # type: ignore[arg-type]

    assert len(response["root"]["children"]) == 1


def test_timing_and_extra_parameters() -> None:
    response = Vespa(URL).search({**QUERY, "presentation.timing": True, "hits": 1})

    assert response["timing"]["querytime"] >= 0


def test_serialized_body() -> None:
    response = Vespa(URL).search(json.dumps({**QUERY, "hits": 1}))

    assert len(response["root"]["children"]) == 1


def test_vespa_errors_are_reported(format: str) -> None:
    client = Vespa(URL, format=format)  # type: ignore[arg-type]

    with pytest.raises(VespaError) as error:
        client.search({"yql": "select * from nope where true"})

    assert error.value.status == 400
    assert error.value.errors[0]["code"] == 4
    assert not error.value.retryable


async def test_async_vespa_errors_are_reported() -> None:
    with pytest.raises(VespaError) as error:
        await AsyncVespa(URL).search({"yql": "select * from nope where true"})

    assert error.value.errors[0]["code"] == 4


def test_unknown_path_is_a_status_error() -> None:
    with pytest.raises(StatusError) as error:
        Vespa(f"{URL}/not-vespa").search(QUERY)

    assert error.value.status == 404
    assert not error.value.retryable
