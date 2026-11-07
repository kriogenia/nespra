"""Sync and async clients."""

from __future__ import annotations

from typing import Any, Generic, TypeVar

from tenacity import AsyncRetrying, BaseRetrying, Retrying

from ._core import Transport
from ._shared import (
    Body,
    Format,
    HttpVersion,
    decode_response,
    encode_request,
)

R = TypeVar("R", bound=BaseRetrying)


class _Base(Generic[R]):
    def __init__(
        self,
        url: str,
        *,
        bearer_token: str | None = None,
        client_cert: bytes | None = None,
        client_key: bytes | None = None,
        headers: dict[str, str] | None = None,
        user_agent: str | None = None,
        timeout: float | None = None,
        connect_timeout: float | None = None,
        http_version: HttpVersion = "HTTP/2",
        format: Format = "cbor",
        retry: R | None = None,
    ) -> None:
        self._format = format
        self._retry = retry
        self._transport = Transport(
            url,
            bearer_token=bearer_token,
            client_cert=client_cert,
            client_key=client_key,
            headers=headers,
            user_agent=user_agent,
            timeout=timeout,
            connect_timeout=connect_timeout,
            http_version=http_version,
            format=format,
        )


class Vespa(_Base[Retrying]):
    """Blocking Vespa client.

    `retry` is any tenacity `Retrying` you build, used for every search unless the
    call passes its own. Without it nothing is retried. Failures raised by the client
    carry a `retryable` attribute you can use in the retry condition.
    """

    def search(
        self,
        body: Body,
        *,
        timeout: float | None = None,
        retry: Retrying | None = None,
    ) -> dict[str, Any]:
        """Runs a search and returns the decoded response.

        Raises a `NespraError`: `RequestError`, `StatusError`, `VespaError` or
        `DecodeError`.
        """
        payload = encode_request(body)

        def attempt() -> dict[str, Any]:
            status, data = self._transport.request(payload, timeout)
            return decode_response(self._format, status, data)

        retrying = retry or self._retry
        return attempt() if retrying is None else retrying.copy()(attempt)


class AsyncVespa(_Base[AsyncRetrying]):
    """Asyncio Vespa client, `retry` is any tenacity `AsyncRetrying`."""

    async def search(
        self,
        body: Body,
        *,
        timeout: float | None = None,
        retry: AsyncRetrying | None = None,
    ) -> dict[str, Any]:
        """Same as `Vespa.search`, awaitable."""
        payload = encode_request(body)

        async def attempt() -> dict[str, Any]:
            status, data = await self._transport.request_async(payload, timeout)
            return decode_response(self._format, status, data)

        retrying = retry or self._retry
        return await attempt() if retrying is None else await retrying.copy()(attempt)
