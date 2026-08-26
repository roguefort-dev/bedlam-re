#!/usr/bin/python3
"""Containment evidence for the required-gates validator (gate env-probe).

This runs INSIDE the validator's bwrap containment, as a gate command, and
fails closed unless the environment matches the documented contract:

  - no PATH and none of the proxy/wrapper/network injection variables,
  - HOME is the per-command scratch home under target/ (writable, and not
    a host-shared path),
  - /tmp is a fresh private tmpfs holding nothing but (when the
    invocation root itself lives under /tmp, as the controller's sealed
    validation basis does) that root's own path chain re-exposed
    read-only,
  - the repository root is read-only,
  - the gate's declared writable directory is writable.

On success it writes the exact observed environment as JSON into the
declared writable directory (runtime/env-probe-out, gitignored scratch),
so a phase report carries the containment facts that produced it.
"""

from __future__ import annotations

import json
import os
import pathlib
import sys

FORBIDDEN = [
    "PATH",
    "PYTHONPATH",
    "PYTHONHOME",
    "BASH_ENV",
    "ENV",
    "RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
]

REPO = pathlib.Path(__file__).resolve().parent.parent
OUT = REPO / "runtime" / "env-probe-out"


def fail(message: str) -> None:
    print(f"check-gates-env: FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    leaks = sorted(name for name in FORBIDDEN if os.environ.get(name) is not None)
    if leaks:
        fail(f"environment variables leaked into the gate sandbox: {leaks}")

    home = pathlib.Path(os.environ.get("HOME", ""))
    expected_parent = REPO / "target" / ".gate-home"
    if home.parent != expected_parent or not home.is_dir():
        fail(f"HOME is not the per-command scratch home under {expected_parent}")
    probe = home / ".writable"
    try:
        probe.write_text("ok")
        if probe.read_text() != "ok":
            raise OSError("read-back mismatch")
        probe.unlink()
    except OSError as error:
        fail(f"the scratch home is not writable: {error}")

    tmp = pathlib.Path("/tmp")
    try:
        top = sorted(item.name for item in tmp.iterdir())
    except OSError as error:
        fail(f"/tmp is not listable: {error}")
    # The invocation root itself may live under /tmp (the controller
    # roots its sealed validation basis in /tmp/opencode); the validator
    # re-exposes exactly that path chain read-only, so it is the one
    # tolerated presence. Anything else under /tmp is host state and
    # fails the contract.
    chain: list[str] = []
    try:
        chain = list(REPO.relative_to(tmp).parts)
    except ValueError:
        chain = []
    if chain:
        walked = tmp
        for component in chain:
            try:
                present = sorted(item.name for item in walked.iterdir())
            except OSError as error:
                fail(f"{walked} is not listable: {error}")
            if present != [component]:
                fail(
                    f"/tmp holds more than the invocation basis chain "
                    f"(found {present})"
                )
            walked = walked / component
    elif top:
        fail(f"/tmp is not a fresh private tmpfs (found {top})")

    sentinel = REPO / ".env-probe-readonly-check"
    try:
        sentinel.write_text("x")
    except OSError:
        pass
    else:
        sentinel.unlink()
        fail("the repository root is writable inside the gate sandbox")

    try:
        OUT.mkdir(parents=True, exist_ok=True)
        snapshot = {
            "environment": dict(sorted(os.environ.items())),
            "home": str(home),
            "tmp_entries_at_start": top,
            "writable_root": str(OUT.relative_to(REPO)),
        }
        (OUT / "environment.json").write_text(
            json.dumps(snapshot, indent=1, sort_keys=True) + "\n"
        )
        back = json.loads((OUT / "environment.json").read_text())
        if back != snapshot:
            raise OSError("snapshot read-back mismatch")
    except OSError as error:
        fail(f"the declared writable directory is not usable: {error}")

    print("check-gates-env: containment contract holds", file=sys.stderr)


if __name__ == "__main__":
    main()
