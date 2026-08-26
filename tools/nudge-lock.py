#!/usr/bin/env python3
"""Acquire a trusted advisory lock, then execute a command while holding it."""

from __future__ import annotations

import fcntl
import os
import signal
import stat
import subprocess
import sys
from pathlib import Path


def main(argv: list[str]) -> int:
    if len(argv) < 5 or argv[1] != "lock-run" or argv[3] not in {"blocking", "nonblocking"}:
        print("usage: nudge-lock.py lock-run PATH blocking|nonblocking COMMAND...", file=sys.stderr)
        return 64

    path = Path(argv[2])
    try:
        fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o022:
            raise ValueError(f"unsafe lock file: {path}")
        if stat.S_IMODE(info.st_mode) != 0o600:
            os.fchmod(fd, 0o600)
        operation = fcntl.LOCK_EX
        if argv[3] == "nonblocking":
            operation |= fcntl.LOCK_NB
        try:
            fcntl.flock(fd, operation)
        except BlockingIOError:
            return 0
        environment = dict(os.environ)
        if path.name == ".queue.lock":
            environment["NUDGE_QUEUE_LOCK_HELD"] = f"{info.st_dev}:{info.st_ino}"
        child = subprocess.Popen(argv[4:], close_fds=True, env=environment)

        def forward(signum: int, _frame: object) -> None:
            try:
                child.send_signal(signum)
            except ProcessLookupError:
                pass

        for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(signum, forward)
        return child.wait()
    except (OSError, ValueError) as error:
        print(f"nudge lock error: {error}", file=sys.stderr)
        return 73
    return 73


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
