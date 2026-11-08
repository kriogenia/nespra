import cbor2
import orjson
import pytest
from nespra import (
    AsyncVespa,
    BuildError,
    DecodeError,
    RequestError,
    StatusError,
    Vespa,
    VespaError,
)
from tenacity import (
    AsyncRetrying,
    Retrying,
    retry_if_exception,
    stop_after_attempt,
    wait_none,
)

RESPONSE = {"root": {"id": "toplevel", "relevance": 1.0, "children": [{"id": "a"}]}}
ERROR = {"root": {"relevance": 0.0, "errors": [{"code": 12, "summary": "timeout"}]}}


def retrying(cls, attempts=3):
    return cls(
        stop=stop_after_attempt(attempts),
        wait=wait_none(),
        retry=retry_if_exception(lambda e: getattr(e, "retryable", False)),
        reraise=True,
    )


QUERY = {"yql": "select * from doc where true"}


def as_cbor(status, value):
    return (status, "application/cbor", cbor2.dumps(value))


def as_json(status, value):
    return (status, "application/json", orjson.dumps(value))


@pytest.fixture(params=["sync", "async"])
def search(request):
    """Runs a search with either client, always as a coroutine function.

    `attempts` builds the client-level tenacity retry (none when `None`) and
    `call_attempts` the one passed to the call.
    """
    sync = request.param == "sync"
    cls = Retrying if sync else AsyncRetrying

    async def run(url, body=QUERY, *, attempts=3, call_attempts=None, search_kwargs=None, **kwargs):
        kwargs.setdefault("http_version", "HTTP/1.1")
        search_kwargs = dict(search_kwargs or {})
        if attempts is not None:
            kwargs["retry"] = retrying(cls, attempts)
        if call_attempts is not None:
            search_kwargs["retry"] = retrying(cls, call_attempts)
        if sync:
            return Vespa(url, **kwargs).search(body, **search_kwargs)
        return await AsyncVespa(url, **kwargs).search(body, **search_kwargs)

    return run


async def test_search_decodes_cbor_by_default(mock, search):
    m = mock(as_cbor(200, RESPONSE))

    assert await search(m.url) == RESPONSE

    received = m.received[0]
    assert orjson.loads(received["body"]) == QUERY
    assert received["headers"]["accept"] == "application/cbor"
    assert received["headers"]["content-type"] == "application/json"


async def test_search_can_use_json(mock, search):
    m = mock(as_json(200, RESPONSE))

    assert await search(m.url, format="json") == RESPONSE
    assert m.received[0]["headers"]["accept"] == "application/json"


async def test_body_can_be_already_serialized(mock, search):
    m = mock(as_cbor(200, RESPONSE))

    await search(m.url, orjson.dumps(QUERY))

    assert orjson.loads(m.received[0]["body"]) == QUERY


async def test_headers_token_and_user_agent_are_sent(mock, search):
    m = mock(as_cbor(200, RESPONSE))

    await search(m.url, bearer_token="secret", headers={"x-test": "1"}, user_agent="me/1")

    headers = m.received[0]["headers"]
    assert headers["authorization"] == "Bearer secret"
    assert headers["x-test"] == "1"
    assert headers["user-agent"] == "me/1"


async def test_retries_transient_failures(mock, search):
    m = mock((503, "text/plain", b""), as_cbor(504, ERROR), as_cbor(200, RESPONSE))

    assert await search(m.url) == RESPONSE
    assert len(m.received) == 3


async def test_gives_up_after_the_max_attempts(mock, search):
    m = mock((503, "text/plain", b"down"))

    with pytest.raises(StatusError) as error:
        await search(m.url)

    assert error.value.status == 503
    assert error.value.body == b"down"
    assert error.value.retryable
    assert len(m.received) == 3


async def test_does_not_retry_permanent_failures(mock, search):
    m = mock(as_cbor(400, {"root": {"relevance": 0.0, "errors": [{"code": 3, "summary": "bad"}]}}))

    with pytest.raises(VespaError) as error:
        await search(m.url)

    assert error.value.errors[0]["code"] == 3
    assert not error.value.retryable
    assert len(m.received) == 1


async def test_vespa_errors_in_a_successful_status(mock, search):
    m = mock(as_cbor(200, ERROR))

    with pytest.raises(VespaError) as error:
        await search(m.url, attempts=None)

    assert error.value.retryable
    assert error.value.status == 200
    assert len(m.received) == 1


async def test_retry_can_be_overridden_per_call(mock, search):
    m = mock((503, "text/plain", b""))

    with pytest.raises(StatusError):
        await search(m.url, call_attempts=1)

    assert len(m.received) == 1


async def test_undecodable_success_is_a_decode_error(mock, search):
    m = mock((200, "application/cbor", b"\xff\x00\x13"))

    with pytest.raises(DecodeError):
        await search(m.url)
    assert len(m.received) == 1


async def test_nothing_is_retried_by_default(mock, search):
    m = mock((503, "text/plain", b""))

    with pytest.raises(StatusError):
        await search(m.url, attempts=None)

    assert len(m.received) == 1


async def test_unreachable_server_is_a_retryable_request_error(search):
    with pytest.raises(RequestError) as error:
        await search("http://127.0.0.1:1", attempts=None)

    assert error.value.retryable


async def test_timeout_is_not_retried(mock, search):
    import time

    m = mock((200, "application/cbor", cbor2.dumps(RESPONSE)))
    original = m.server.RequestHandlerClass.do_POST

    def slow(self):
        time.sleep(0.5)
        original(self)

    m.server.RequestHandlerClass.do_POST = slow

    with pytest.raises(RequestError) as error:
        await search(m.url, timeout=5, search_kwargs={"timeout": 0.05})

    assert not error.value.retryable


def test_invalid_configuration():
    with pytest.raises(BuildError):
        Vespa("not a url")
    with pytest.raises(BuildError):
        Vespa("http://localhost:8080", format="xml")  # type: ignore[arg-type]
    with pytest.raises(BuildError):
        Vespa("http://localhost:8080", http_version="3")  # type: ignore[arg-type]
    with pytest.raises(BuildError):
        Vespa("http://localhost:8080", timeout=-1)
    with pytest.raises(BuildError):
        Vespa("http://localhost:8080", client_cert=b"cert")
    with pytest.raises(BuildError):
        Vespa("http://localhost:8080", bearer_token="bad\ntoken")
