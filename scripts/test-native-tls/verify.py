"""Verify the isolated native-TLS helper against the published Qbot image."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
import tomllib
import urllib.parse
import urllib.request

SOURCE = "0f0e708b6087dee1c3aa02db6566c2abdc574dc2"
REPOSITORY = "thatxliner/qbot"
URLS = [
    "https://dv-g3b-emudhra.we-have-a-problem.com/",
    "https://dv-g3b-aaa-emudhra.we-have-a-problem.com/",
    "https://example.com/",
]
VERSIONS = {
    "reqwest": "0.12.28", "native-tls": "0.2.14", "openssl": "0.10.81",
    "openssl-sys": "0.9.117", "tokio": "1.47.1",
}


def docker(*args):
    return subprocess.check_output(["docker", *args], text=True)


def check_locks(app):
    probe = Path(__file__).parent
    packages = lambda path: tomllib.loads(path.read_text())["package"]
    app_packages = packages(Path(app) / "Cargo.lock")
    helper_packages = packages(probe / "Cargo.lock")
    for name, version in VERSIONS.items():
        for graph in (app_packages, helper_packages):
            assert [p["version"] for p in graph if p["name"] == name] == [version], name
    app_set = {(p["name"], p["version"], p.get("checksum")) for p in app_packages}
    assert all((p["name"], p["version"], p.get("checksum")) in app_set
               for p in helper_packages if p.get("source")), "helper dependency drift"
    return VERSIONS


def verify_image(args):
    versions = check_locks(args.app)
    info = json.loads(docker("image", "inspect", args.image))[0]
    assert info["Architecture"] == args.arch
    labels = info["Config"]["Labels"]
    assert labels["org.opencontainers.image.revision"] == SOURCE
    assert labels["org.opencontainers.image.source"] == "https://github.com/ThatXliner/qbot"
    container = docker("create", "--entrypoint", "/bin/sleep", args.image, "240").strip()
    attempts, verified = [], {}
    try:
        docker("cp", args.helper, f"{container}:/tmp/tls-probe")
        docker("start", container)
        runtime = docker("exec", container, "sh", "-ec", """
            dpkg-query -W ca-certificates libssl3t64
            dpkg --compare-versions "$(dpkg-query -W -f='${Version}' ca-certificates)" ge 20250419+deb13u1
            sha256sum /app/qbot /etc/ssl/certs/ca-certificates.crt
            ldd /app/qbot
            ldd /tmp/tls-probe
        """)
        # Both executables load the same OpenSSL libraries from the published image.
        for library in ("libssl.so.3", "libcrypto.so.3"):
            lines = [line.strip() for line in runtime.splitlines() if library in line]
            assert len(lines) == 2, (library, runtime)
            paths = [line.split("=>", 1)[1].split()[0] for line in lines]
            assert paths[0] == paths[1], paths
        for attempt in range(1, 4):
            output = docker("exec", container, "/tmp/tls-probe")
            for line in output.splitlines():
                row = json.loads(line)
                row["attempt"] = attempt
                attempts.append(row)
                url = row["url"]
                assert url in URLS, row
                if url == URLS[-1]:
                    if row.get("status") == 200 and row["outcome"] == "accepted":
                        verified[url] = row
                else:
                    assert row["outcome"] != "accepted", f"untrusted certificate accepted: {row}"
                    if "certificate verify failed" in row.get("error", ""):
                        verified[url] = row
            if len(verified) == 3:
                break
            time.sleep(2)
        evidence = dict(source=SOURCE, arch=args.arch, image=args.image,
                        image_id=info["Id"], tls_versions=versions,
                        runtime=runtime, attempts=attempts,
                        verified=[verified[url] for url in URLS if url in verified],
                        helper_sha256=hashlib.sha256(Path(args.helper).read_bytes()).hexdigest())
        Path(args.output).write_text(json.dumps(evidence, indent=2) + "\n")
        print(json.dumps(evidence, indent=2))
        assert len(verified) == 3, "TLS proof incomplete: DNS/timeout does not prove rejection"
    finally:
        docker("rm", "-f", container)


def public_manifest(reference):
    # Anonymous registry access proves that no user's Docker credentials are needed.
    url = "https://ghcr.io/token?" + urllib.parse.urlencode({
        "service": "ghcr.io", "scope": f"repository:{REPOSITORY}:pull"})
    with urllib.request.urlopen(url, timeout=30) as response:
        token = json.load(response)["token"]
    request = urllib.request.Request(
        f"https://ghcr.io/v2/{REPOSITORY}/manifests/{reference}",
        headers={"Authorization": f"Bearer {token}", "Accept": ", ".join([
            "application/vnd.oci.image.index.v1+json",
            "application/vnd.docker.distribution.manifest.list.v2+json",
            "application/vnd.oci.image.manifest.v1+json",
            "application/vnd.docker.distribution.manifest.v2+json"])})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.headers["Docker-Content-Digest"], json.load(response)


def verify_public(args):
    proofs = [json.loads(path.read_text()) for path in Path(args.proofs).glob("proof-*.json")]
    assert sorted(p["arch"] for p in proofs) == ["amd64", "arm64"]
    expected = {}
    for proof in proofs:
        assert proof["source"] == SOURCE and proof["tls_versions"] == VERSIONS
        assert [r["url"] for r in proof["verified"]] == URLS
        for row in proof["verified"][:2]:
            assert row["outcome"] == "error" and "certificate verify failed" in row["error"]
        assert proof["verified"][-1]["status"] == 200
        digest = proof["image"].split("@", 1)[1]
        child_digest, child = public_manifest(digest)
        assert child_digest == digest
        # Buildx may wrap the single-platform image in an attestation index.
        if "manifests" in child:
            platform_images = [m for m in child["manifests"] if m.get("platform", {}).get("architecture") == proof["arch"]]
            assert len(platform_images) == 1
            digest = platform_images[0]["digest"]
        expected[proof["arch"]] = digest
    if args.tag:
        digest, manifest = public_manifest(args.tag)
        actual = {m["platform"]["architecture"]: m["digest"] for m in manifest["manifests"]
                  if m["platform"].get("os") == "linux"}
        assert actual == expected, (actual, expected)
        print(json.dumps(dict(tag=args.tag, digest=digest, platforms=actual, public=True), indent=2))
    else:
        print(json.dumps(dict(verified_public_children=expected), indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    locks = commands.add_parser("locks")
    locks.add_argument("--app", required=True)
    image = commands.add_parser("image")
    for argument in ("image", "arch", "helper", "app", "output"):
        image.add_argument(f"--{argument}", required=True)
    public = commands.add_parser("public")
    public.add_argument("--proofs", required=True)
    public.add_argument("--tag")
    args = parser.parse_args()
    if args.command == "locks":
        print(json.dumps(check_locks(args.app), indent=2))
    elif args.command == "image":
        verify_image(args)
    else:
        verify_public(args)
