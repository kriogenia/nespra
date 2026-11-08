"""Errors raised by nespra."""

from __future__ import annotations

from typing import Any

from ._core import BuildError, NespraError, RequestError

__all__ = [
    "BuildError",
    "DecodeError",
    "NespraError",
    "RequestError",
    "StatusError",
    "VespaError",
]

# Statuses and Vespa error codes that usually go away on their own: 429, 502, 503 and 504,
# and NO_BACKENDS_IN_SERVICE (0), BACKEND_COMMUNICATION_ERROR (10) and TIMEOUT (12).
_TRANSIENT_STATUSES = frozenset({429, 502, 503, 504})
_TRANSIENT_VESPA_CODES = frozenset({0, 10, 12})

# `RequestError` sets its own `retryable` when raised from the transport.
NespraError.retryable = False


class DecodeError(NespraError):
    """The response body could not be decoded."""


class StatusError(NespraError):
    """Non-successful HTTP status whose body is not a Vespa error response."""

    def __init__(self, status: int, body: bytes) -> None:
        super().__init__(f"Unexpected HTTP status {status}")
        self.status = status
        self.body = body
        self.retryable = status in _TRANSIENT_STATUSES


class VespaError(NespraError):
    """Errors reported inside the Vespa response."""

    def __init__(self, errors: list[dict[str, Any]], status: int) -> None:
        first = errors[0]
        super().__init__(
            f"Errors in Vespa search response, first error: "
            f"{first.get('code')} - {first.get('summary')}"
        )
        self.errors = errors
        self.status = status
        self.retryable = any(e.get("code") in _TRANSIENT_VESPA_CODES for e in errors)
