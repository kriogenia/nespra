"""Pieces shared by the sync and async clients."""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any, Literal

import cbor2
import orjson

from .errors import DecodeError, StatusError, VespaError

Format = Literal["cbor", "json"]
HttpVersion = Literal["HTTP/1.1", "HTTP/2"]
Body = Mapping[str, Any] | bytes | str


def encode_request(body: Body) -> bytes:
    if isinstance(body, bytes):
        return body
    if isinstance(body, str):
        return body.encode()
    return orjson.dumps(body)


_DECODERS: dict[str, Callable[[bytes], Any]] = {
    "cbor": cbor2.loads,
    "json": orjson.loads,
}


def decode_response(format: str, status: int, body: bytes) -> dict[str, Any]:
    """Decodes the body and raises the error that corresponds to a failed response.

    Follows the same rules as the Rust client: errors reported by Vespa come first,
    then non-successful statuses.
    """
    try:
        response: dict[str, Any] = _DECODERS[format](body)
    except Exception as e:
        if not 200 <= status < 300:
            raise StatusError(status, body) from e
        raise DecodeError(f"Failed to decode the response: {e}") from e

    if errors := response.get("root", {}).get("errors"):
        raise VespaError(errors, status)

    if not 200 <= status < 300:
        raise StatusError(status, body)

    return response
