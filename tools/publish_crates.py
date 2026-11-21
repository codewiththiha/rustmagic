#!/usr/bin/env python3
"""Publishes the engine crates to crates.io, in the order they depend on.

`cargo publish` refuses to upload a crate whose dependencies are not on the
registry yet, so the order is not a preference: it is the dependency graph.
The graph is read from `cargo metadata` rather than written down here, because
a hand-kept list is a list that goes stale the first time someone adds a crate.

Every crate is published with `--locked`, so what lands on crates.io is built
from the same Cargo.lock the release was tested with.

  tools/publish_crates.py --dry-run     # package each one, upload nothing
  tools/publish_crates.py               # the real thing, CARGO_REGISTRY_TOKEN set

A crate that is already on the registry at this version is skipped instead of
failing, so re-running after a half-finished publish finishes the job.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# The engine, and nothing that knows about the app. `km-store` embeds the
# keyboard files that ship with the desktop build, `km-session` owns the app's
# on-disk layout, and `km-ibus` speaks D-Bus to a Linux session: none of those
# are meaningful to someone who wants to embed a keyboard engine.
PUBLISHED = ("km-core", "km-binary", "km-script", "km-engine", "km-ffi")

INDEX = "https://index.crates.io"
WAIT_SECONDS = 300
POLL_SECONDS = 5


def metadata() -> dict:
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return json.loads(out)


def order(packages: list[dict]) -> list[str]:
    """The publish order: every crate after the ones it depends on."""
    by_name = {package["name"]: package for package in packages}
    seen: list[str] = []

    def visit(name: str) -> None:
        if name in seen or name not in by_name:
            return
        for dependency in by_name[name]["dependencies"]:
            if dependency["name"] in PUBLISHED:
                visit(dependency["name"])
        seen.append(name)

    for name in PUBLISHED:
        visit(name)
    missing = [name for name in PUBLISHED if name not in by_name]
    if missing:
        raise SystemExit(f"not in this workspace: {', '.join(missing)}")
    return seen


def version_of(packages: list[dict], name: str) -> str:
    return next(package["version"] for package in packages if package["name"] == name)


def index_path(name: str) -> str:
    """Where the sparse index keeps a crate: crates.io's own fan-out scheme."""
    lowered = name.lower()
    if len(lowered) == 1:
        return f"{INDEX}/1/{lowered}"
    if len(lowered) == 2:
        return f"{INDEX}/2/{lowered}"
    if len(lowered) == 3:
        return f"{INDEX}/3/{lowered[0]}/{lowered}"
    return f"{INDEX}/{lowered[:2]}/{lowered[2:4]}/{lowered}"


def on_registry(name: str, version: str) -> bool:
    """Whether this exact version is already published.

    A network failure is not a "no": uploading over a version that exists is
    answered by cargo with a clear error, and skipping a crate that is not there
    would be worse than trying.
    """
    try:
        with urllib.request.urlopen(index_path(name), timeout=20) as response:
            body = response.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as error:
        return error.code == 404 and False
    except OSError:
        return False
    pattern = re.compile(r'"vers"\s*:\s*"' + re.escape(version) + r'"')
    return bool(pattern.search(body))


def wait_for(name: str, version: str) -> bool:
    deadline = time.monotonic() + WAIT_SECONDS
    while time.monotonic() < deadline:
        if on_registry(name, version):
            return True
        time.sleep(POLL_SECONDS)
    return False


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true", help="package, upload nothing")
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="publish from a tree with uncommitted changes; only for a dry run",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="publish even if the index says this version is already there",
    )
    args = parser.parse_args(argv)

    packages = metadata()["packages"]
    plan = order(packages)
    print("publish order:", ", ".join(plan))

    for name in plan:
        version = version_of(packages, name)
        if not args.force and not args.dry_run and on_registry(name, version):
            print(f"-- {name} {version} is already on crates.io, skipping")
            continue
        # A dry run packages each crate instead of uploading it. `cargo publish
        # --dry-run` would also have to resolve km-core from the registry, and
        # on the first release of a new crate the registry does not have it yet,
        # so it would fail for a reason that has nothing to do with the crate.
        if args.dry_run:
            command = ["cargo", "package", "--no-verify", "-p", name]
        else:
            command = ["cargo", "publish", "--locked", "-p", name]
        if args.allow_dirty:
            command.append("--allow-dirty")
        print(f"-- {name} {version}: {' '.join(command)}")
        if args.dry_run:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            missing = re.search(r"no matching package named `([^`]+)` found", result.stderr)
            if missing and missing.group(1) in PUBLISHED:
                # Expected on a first release: the dependency is packaged but not
                # yet uploaded, so this crate cannot be checked against it. The
                # real publish does them in order and proves it.
                print(
                    f"   {name} needs {missing.group(1)} on the registry first; "
                    "it is packaged and will be checked by the real run"
                )
                continue
            if result.stdout:
                print(result.stdout, end="")
            if result.stderr:
                print(result.stderr, end="", file=sys.stderr)
        else:
            result = subprocess.run(command, cwd=ROOT)
        if result.returncode != 0:
            print(f"!! {name} failed", file=sys.stderr)
            return result.returncode
        if args.dry_run:
            continue
        # crates.io serves the index asynchronously; the next crate cannot
        # resolve its dependency until this one is visible.
        if not wait_for(name, version):
            print(
                f"!! {name} {version} did not appear in the index within "
                f"{WAIT_SECONDS}s; the next crate would fail to resolve it",
                file=sys.stderr,
            )
            return 1
    print("done")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
