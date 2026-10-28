#!/usr/bin/env python3
"""Smoke-test an installed eudkit-lsp binary without building Rust.

Checks the CLI version contract and a real stdio LSP session, including the
first canonical-unit completion after ``KillUnit("``.

Usage:
    python scripts/smoke_eudkit_lsp.py /path/to/eudkit-lsp [--expected-version 0.1.0]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

EXPECTED_UNIT_COUNT = 228


def read_message(stream) -> dict:
    length = None
    while True:
        line = stream.readline()
        if not line:
            raise SystemExit("eudkit-lsp closed stdout unexpectedly")
        line = line.strip()
        if not line:
            break
        if line.lower().startswith(b"content-length:"):
            length = int(line.split(b":", 1)[1])
    if length is None:
        raise SystemExit("eudkit-lsp response did not include Content-Length")
    return json.loads(stream.read(length))


def send_message(stream, message: dict) -> None:
    body = json.dumps(message).encode()
    stream.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    stream.flush()


def read_response(stream, request_id: int) -> dict:
    while True:
        message = read_message(stream)
        if message.get("id") == request_id:
            return message


def check_version(binary: Path, expected: str | None) -> str:
    result = subprocess.run(
        [str(binary), "--version"],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    output = result.stdout.strip()
    if result.returncode != 0:
        raise SystemExit(
            f"{binary} --version failed with exit {result.returncode}: "
            f"{result.stderr.strip()}"
        )
    prefix = "eudkit-lsp "
    if not output.startswith(prefix):
        raise SystemExit(
            f'{binary} --version printed {output!r}; expected "eudkit-lsp <version>"'
        )
    version = output[len(prefix) :].strip()
    if expected is not None and version != expected.lstrip("v"):
        raise SystemExit(
            f"{binary} reports version {version}, but the extension/release expects "
            f"{expected.lstrip('v')}"
        )
    print(f"{binary} reports eudkit-lsp {version}")
    return version


def check_completion(binary: Path) -> None:
    proc = subprocess.Popen(
        [str(binary)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert proc.stdin is not None and proc.stdout is not None
    try:
        send_message(
            proc.stdin,
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"capabilities": {}},
            },
        )
        initialize = read_response(proc.stdout, 1)
        if "result" not in initialize:
            raise SystemExit(f"initialize failed: {initialize}")
        send_message(proc.stdin, {"jsonrpc": "2.0", "method": "initialized", "params": {}})

        text = 'KillUnit("'
        uri = "file:///eudkit-smoke/kill_unit.py"
        send_message(
            proc.stdin,
            {
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "python",
                        "version": 1,
                        "text": text,
                    }
                },
            },
        )
        send_message(
            proc.stdin,
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": 0, "character": len(text)},
                },
            },
        )
        response = read_response(proc.stdout, 2)
        result = response.get("result")
        if not isinstance(result, list):
            raise SystemExit(f"completion returned no list: {response}")
        labels = [item["label"] for item in result]
        if len(labels) != EXPECTED_UNIT_COUNT:
            raise SystemExit(
                f"expected {EXPECTED_UNIT_COUNT} canonical units, got {len(labels)}"
            )
        if "Terran Marine" not in labels:
            raise SystemExit("completion is missing 'Terran Marine'")
        if "Any unit" in labels or any(label.startswith("UNI") for label in labels):
            raise SystemExit("completion contains non-canonical unit filters")
        print(f"eudkit-lsp returned {len(labels)} canonical units")

        send_message(
            proc.stdin,
            {"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": None},
        )
        read_response(proc.stdout, 3)
        send_message(proc.stdin, {"jsonrpc": "2.0", "method": "exit", "params": None})
    finally:
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=10)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path, help="Path to eudkit-lsp")
    parser.add_argument(
        "--expected-version",
        help="Required version without a leading v (for example 0.1.0)",
    )
    args = parser.parse_args()

    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"eudkit-lsp binary not found: {binary}")
    check_version(binary, args.expected_version)
    check_completion(binary)
    print(f"eudkit-lsp smoke test passed for {binary}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
