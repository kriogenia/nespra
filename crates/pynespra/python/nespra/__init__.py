"""Unofficial Python client for the Vespa search engine, built on the `nespra` Rust crate."""

from .client import AsyncVespa, Vespa
from .errors import (
    BuildError,
    DecodeError,
    NespraError,
    RequestError,
    StatusError,
    VespaError,
)

__all__ = [
    "AsyncVespa",
    "BuildError",
    "DecodeError",
    "NespraError",
    "RequestError",
    "StatusError",
    "Vespa",
    "VespaError",
]
