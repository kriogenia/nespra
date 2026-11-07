from collections.abc import Awaitable

class NespraError(Exception):
    retryable: bool

class BuildError(NespraError): ...
class RequestError(NespraError): ...

class Transport:
    def __init__(
        self,
        url: str,
        *,
        bearer_token: str | None = None,
        client_cert: bytes | None = None,
        client_key: bytes | None = None,
        headers: list[tuple[str, str]] | None = None,
        user_agent: str | None = None,
        timeout: float | None = None,
        connect_timeout: float | None = None,
        http_version: str = "2",
        format: str = "cbor",
    ) -> None: ...
    def request(self, body: bytes, timeout: float | None = None) -> tuple[int, bytes]: ...
    def request_async(
        self, body: bytes, timeout: float | None = None
    ) -> Awaitable[tuple[int, bytes]]: ...
