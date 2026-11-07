"""Scripted HTTP/1.1 server to test the clients without a Vespa."""

from __future__ import annotations

import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest


class Mock:
    def __init__(self, replies):
        self.replies = replies
        self.received: list[dict] = []
        mock = self

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                body = self.rfile.read(int(self.headers.get("content-length", 0)))
                index = min(len(mock.received), len(mock.replies) - 1)
                mock.received.append(
                    {"headers": {k.lower(): v for k, v in self.headers.items()}, "body": body}
                )
                status, content_type, payload = mock.replies[index]
                self.send_response(status)
                self.send_header("content-type", content_type)
                self.send_header("content-length", str(len(payload)))
                self.send_header("connection", "close")
                self.end_headers()
                self.wfile.write(payload)

            def log_message(self, *args):
                pass

        self.server = HTTPServer(("127.0.0.1", 0), Handler)
        self.url = f"http://127.0.0.1:{self.server.server_port}"
        threading.Thread(
            target=self.server.serve_forever, kwargs={"poll_interval": 0.01}, daemon=True
        ).start()

    def close(self):
        self.server.shutdown()
        self.server.server_close()


@pytest.fixture
def mock():
    mocks = []

    def start(*replies):
        m = Mock(replies)
        mocks.append(m)
        return m

    yield start
    for m in mocks:
        m.close()
