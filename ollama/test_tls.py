#!/usr/bin/env python3
"""Check the image's real Ollama registry HTTP path without downloading models."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request


def docker(*args):
    return subprocess.check_output(["docker", *args], text=True).strip()


def go_version(binary):
    # Go 1.18+ embeds length-prefixed build information in the executable.
    start = binary.index(b"\xff Go buildinf:")
    if not binary[start + 15] & 2:
        raise ValueError("expected inline Go build information")
    offset = start + 32
    length = shift = 0
    while True:
        byte = binary[offset]
        offset += 1
        length |= (byte & 127) << shift
        if byte < 128:
            return binary[offset:offset + length].decode()
        shift += 7


def main():
    image = sys.argv[1]
    container = docker(
        "run", "--rm", "--detach", "--publish", "127.0.0.1::11434",
        "--env", "OLLAMA_HOST=0.0.0.0:11434", "--env", "OLLAMA_MODELS=/tmp/models",
        "--entrypoint", "/bin/ollama", image, "serve",
    )
    result = {"image": image, "probes": []}
    try:
        port = docker("port", container, "11434/tcp").rsplit(":", 1)[1]
        base = "http://127.0.0.1:" + port
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        for attempt in range(60):
            try:
                with opener.open(base + "/api/version", timeout=2) as response:
                    result["ollama"] = json.load(response)
                break
            except (OSError, urllib.error.URLError):
                if attempt == 59:
                    raise
                time.sleep(1)

        result["ca_certificates"] = docker(
            "exec", container, "dpkg-query", "-W", "-f=${Version}", "ca-certificates"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "ollama"
            docker("cp", container + ":/bin/ollama", str(path))
            binary = path.read_bytes()
            result["binary_sha256"] = hashlib.sha256(binary).hexdigest()
            result["go_version"] = go_version(binary)

        for host, expected in (
            ("dv-g3b-emudhra.we-have-a-problem.com", "certificate_rejected"),
            ("dv-g3b-aaa-emudhra.we-have-a-problem.com", "certificate_rejected"),
            ("example.com", "trusted_http_response"),
        ):
            # These hosts are not model registries. A 404 manifest response from
            # example.com proves the TLS handshake succeeded, without any blobs.
            request = urllib.request.Request(
                base + "/api/pull",
                data=json.dumps({"model": host + "/library/tls-probe:latest", "stream": False}).encode(),
                headers={"Content-Type": "application/json"},
            )
            try:
                try:
                    response = opener.open(request, timeout=35)
                except urllib.error.HTTPError as error:
                    response = error
                with response:
                    body = json.load(response)
                error = body.get("error", "")
                if "x509: certificate signed by unknown authority" in error:
                    outcome = "certificate_rejected"
                elif error == "pull model manifest: file does not exist":
                    outcome = "trusted_http_response"
                elif any(message in error.lower() for message in (
                    "timeout", "timed out", "no such host", "network is unreachable",
                    "temporary failure in name resolution", "connection refused",
                )):
                    outcome = "inconclusive"
                else:
                    outcome = "unexpected_response"
                result["probes"].append({"host": host, "expected": expected,
                                         "outcome": outcome, "response": body})
            except (OSError, urllib.error.URLError) as error:
                result["probes"].append({"host": host, "expected": expected,
                                         "outcome": "inconclusive", "error": str(error)})
        print(json.dumps(result, indent=2), flush=True)
        return int(any(probe["outcome"] != probe["expected"] for probe in result["probes"]))
    finally:
        docker("rm", "--force", container)


if __name__ == "__main__":
    sys.exit(main())
