#!/usr/bin/env python3
"""Atomically publish trusted nudge state without following symlinks."""

from __future__ import annotations

import datetime as dt
import fcntl
import hashlib
import importlib.util
import json
import os
import re
import signal
import stat
import subprocess
import sys
import shutil
import tempfile
import tarfile
import time
import tomllib
from pathlib import Path


SAFE_SESSION = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
SAFE_ID = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
SAFE_HASH = re.compile(r"^[0-9a-f]{64}$")
RESERVED_SESSIONS = {"owner", "publish.lock", "executor.lock", "queue.lock", "archive", "tmp-lock"}
CLAIM_NAME = re.compile(r"^[1-9][0-9]*-[A-Za-z0-9][A-Za-z0-9._-]*\.claim$")
CLAIM_TIME = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:Z|[+-]\d{2}:\d{2})$"
)
MAX_STATE_FILE = 1024 * 1024
MAX_FAILURES = 256
MAX_CLAIM_FILE = 64 * 1024
MAX_CLAIMS = 256


def open_directory(path: Path, create: bool = False) -> int:
    """Walk an absolute directory one no-follow component at a time."""
    absolute = path.absolute()
    components = absolute.parts[1:]
    current = os.open("/", os.O_RDONLY | os.O_DIRECTORY)
    try:
        for index, component in enumerate(components):
            try:
                following = os.open(
                    component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                    dir_fd=current,
                )
            except FileNotFoundError:
                if not create or index != len(components) - 1:
                    raise
                os.mkdir(component, 0o700, dir_fd=current)
                following = os.open(
                    component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                    dir_fd=current,
                )
            os.close(current)
            current = following
        info = os.fstat(current)
        if info.st_uid != os.geteuid() or info.st_mode & 0o022:
            raise ValueError(f"untrusted state directory: {path}")
        return current
    except Exception:
        os.close(current)
        raise


def split_state_path(path: Path, create_parent: bool = False) -> tuple[int, str]:
    if not path.name or path.name in {".", ".."}:
        raise ValueError(f"unsafe state path: {path}")
    return open_directory(path.parent, create=create_parent), path.name


def trusted_directory(path: Path, create: bool = False) -> None:
    descriptor = open_directory(path, create=create)
    os.close(descriptor)


def ensure_directory(arguments: list[str]) -> None:
    if len(arguments) != 1:
        raise ValueError("ensure-dir requires a path")
    trusted_directory(Path(arguments[0]), create=True)


def trusted_file_bytes(path: Path) -> tuple[os.stat_result, bytes]:
    parent_fd, name = split_state_path(path)
    descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=parent_fd)
    try:
        info = os.fstat(descriptor)
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                or info.st_mode & 0o022 or info.st_size > MAX_STATE_FILE):
            raise ValueError(f"unsafe state file: {path}")
        chunks = []
        while chunk := os.read(descriptor, 1024 * 1024):
            chunks.append(chunk)
        return info, b"".join(chunks)
    finally:
        os.close(descriptor)
        os.close(parent_fd)


def replace_publish(path: Path, payload: bytes) -> None:
    if len(payload) > MAX_STATE_FILE:
        raise ValueError(f"state payload exceeds size limit: {path}")
    parent_fd, name = split_state_path(path, create_parent=True)
    queue_lock_fd = -1
    temporary = f".{name}.tmp-{os.getpid()}-{os.urandom(4).hex()}"
    try:
        if name == "NEXT.md" and not os.environ.get("NUDGE_QUEUE_LOCK_HELD"):
            queue_lock_fd = os.open(
                ".queue.lock", os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600,
                dir_fd=parent_fd,
            )
            lock_info = os.fstat(queue_lock_fd)
            if (not stat.S_ISREG(lock_info.st_mode) or lock_info.st_uid != os.geteuid()
                    or lock_info.st_mode & 0o022):
                raise ValueError("unsafe shared queue lock")
            fcntl.flock(queue_lock_fd, fcntl.LOCK_EX)
        try:
            existing = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
            if not stat.S_ISREG(existing.st_mode):
                raise ValueError(f"unsafe publication target: {path}")
        except FileNotFoundError:
            pass
        descriptor = os.open(
            temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
            0o600, dir_fd=parent_fd,
        )
        try:
            temporary_info = os.fstat(descriptor)
            published_identity = temporary_info.st_dev, temporary_info.st_ino
            os.write(descriptor, payload)
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
        os.replace(temporary, name, src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
        published_fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=parent_fd)
        try:
            info = os.fstat(published_fd)
            digest = hashlib.sha256()
            total = 0
            while chunk := os.read(published_fd, 64 * 1024):
                total += len(chunk)
                if total > MAX_STATE_FILE:
                    raise ValueError(f"published state exceeds size limit: {path}")
                digest.update(chunk)
        finally:
            os.close(published_fd)
        if (not stat.S_ISREG(info.st_mode)
                or (info.st_dev, info.st_ino) != published_identity
                or digest.digest() != hashlib.sha256(payload).digest()):
            raise ValueError(f"publication target swapped after replace: {path}")
        if name == "NEXT.md":
            try:
                os.unlink("PLAN-COMPLETE", dir_fd=parent_fd)
            except FileNotFoundError:
                pass
        os.fsync(parent_fd)
    finally:
        try:
            os.unlink(temporary, dir_fd=parent_fd)
        except FileNotFoundError:
            pass
        if queue_lock_fd >= 0:
            os.close(queue_lock_fd)
        os.close(parent_fd)


def read_decimal(path: Path, label: str, minimum: int, maximum: int, default: str | None) -> int:
    try:
        _info, raw = trusted_file_bytes(path)
    except FileNotFoundError:
        if default is None:
            raise ValueError(f"missing {label}")
        raw = default.encode()
    try:
        text = raw.decode("ascii").strip()
    except UnicodeError as error:
        raise ValueError(f"invalid {label}: non-ASCII decimal") from error
    if not re.fullmatch(r"[0-9]{1,20}", text):
        raise ValueError(f"invalid {label}: expected bounded unsigned decimal")
    value = int(text, 10)
    if not minimum <= value <= maximum:
        raise ValueError(f"out-of-range {label}: {value}")
    return value


def read_int_command(arguments: list[str]) -> None:
    if len(arguments) != 5:
        raise ValueError("read-int requires path, label, minimum, maximum, default-or-dash")
    default = None if arguments[4] == "-" else arguments[4]
    print(read_decimal(Path(arguments[0]), arguments[1], int(arguments[2]), int(arguments[3]), default))


def read_fields(arguments: list[str]) -> None:
    if len(arguments) != 7:
        raise ValueError("read-fields requires path, two labels, and two min/max pairs")
    _info, raw = trusted_file_bytes(Path(arguments[0]))
    try:
        text = raw.decode("ascii")
    except UnicodeError as error:
        raise ValueError("invalid numeric state fields") from error
    match = re.fullmatch(r"([0-9]{1,20}) ([0-9]{1,20})\n?", text)
    if not match:
        raise ValueError(f"invalid {arguments[1]} or {arguments[2]} numeric state")
    values = [int(match.group(1)), int(match.group(2))]
    bounds = [(int(arguments[3]), int(arguments[4])), (int(arguments[5]), int(arguments[6]))]
    for value, (minimum, maximum), label in zip(values, bounds, arguments[1:3]):
        if not minimum <= value <= maximum:
            raise ValueError(f"out-of-range {label}: {value}")
    print(f"{values[0]} {values[1]}")


def write_text(arguments: list[str]) -> None:
    if len(arguments) != 2:
        raise ValueError("write-text requires path and payload")
    replace_publish(Path(arguments[0]), arguments[1].encode())


def read_text(arguments: list[str]) -> None:
    if len(arguments) != 1:
        raise ValueError("read-text requires a path")
    _info, raw = trusted_file_bytes(Path(arguments[0]))
    sys.stdout.buffer.write(raw)


def create_text(arguments: list[str]) -> None:
    if len(arguments) != 2:
        raise ValueError("create-text requires path and payload")
    publish(Path(arguments[0]), arguments[1].encode())


def append_text(arguments: list[str]) -> None:
    if len(arguments) != 2:
        raise ValueError("append-text requires path and payload")
    path, payload = Path(arguments[0]), arguments[1].encode()
    if len(payload) > 64 * 1024:
        raise ValueError("append payload exceeds size limit")
    parent_fd, name = split_state_path(path, create_parent=True)
    descriptor = os.open(
        name, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW,
        0o600, dir_fd=parent_fd,
    )
    try:
        info = os.fstat(descriptor)
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                or info.st_mode & 0o022 or info.st_size > MAX_STATE_FILE):
            raise ValueError(f"unsafe append target: {path}")
        os.write(descriptor, payload)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
        os.close(parent_fd)


def output_descriptor(path: Path, append: bool) -> int:
    parent_fd, name = split_state_path(path, create_parent=True)
    flags = os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW
    flags |= os.O_APPEND if append else os.O_TRUNC
    try:
        descriptor = os.open(name, flags, 0o600, dir_fd=parent_fd)
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o022:
            os.close(descriptor)
            raise ValueError(f"unsafe output target: {path}")
        return descriptor
    finally:
        os.close(parent_fd)


def run_output(arguments: list[str]) -> None:
    if len(arguments) < 3 or arguments[1] not in {"append", "truncate"}:
        raise ValueError("run-output requires path, append|truncate, and command")
    descriptor = output_descriptor(Path(arguments[0]), arguments[1] == "append")
    try:
        result = subprocess.run(arguments[2:], stdout=descriptor, stderr=subprocess.STDOUT, check=False)
    finally:
        os.close(descriptor)
    if result.returncode:
        raise RuntimeError(f"output command failed rc={result.returncode}")


def exec_output(arguments: list[str]) -> None:
    if len(arguments) < 3 or arguments[1] not in {"append", "truncate"}:
        raise ValueError("exec-output requires path, append|truncate, and command")
    descriptor = output_descriptor(Path(arguments[0]), arguments[1] == "append")
    os.dup2(descriptor, 1)
    os.dup2(descriptor, 2)
    if descriptor > 2:
        os.close(descriptor)
    os.execvpe(arguments[2], arguments[2:], os.environ)


def append_file(arguments: list[str]) -> None:
    if len(arguments) != 2:
        raise ValueError("append-file requires destination and source")
    _info, raw = trusted_file_bytes(Path(arguments[1]))
    append_text([arguments[0], raw.decode("utf-8")])


def retain_tail(arguments: list[str]) -> None:
    if len(arguments) != 2:
        raise ValueError("retain-tail requires path and byte count")
    path = Path(arguments[0])
    keep = int(arguments[1])
    if not 1 <= keep <= MAX_STATE_FILE:
        raise ValueError("retain-tail byte count is out of range")
    _info, raw = trusted_file_bytes(path)
    if len(raw) > keep:
        replace_publish(path, raw[-keep:])


def touch_state(arguments: list[str]) -> None:
    if len(arguments) not in {1, 2, 4}:
        raise ValueError("touch requires path, optional epoch, and optional device/inode")
    path = Path(arguments[0])
    epoch = int(arguments[1]) if len(arguments) == 2 else None
    if len(arguments) == 4:
        epoch = int(arguments[1])
        expected = int(arguments[2]), int(arguments[3])
    else:
        expected = None
    parent_fd, name = split_state_path(path, create_parent=True)
    descriptor = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW, 0o600, dir_fd=parent_fd)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o022:
            raise ValueError(f"unsafe touch target: {path}")
        if expected is not None and (info.st_dev, info.st_ino) != expected:
            raise ValueError(f"state inode changed before touch: {path}")
        os.utime(descriptor, None if epoch is None else (epoch, epoch))
    finally:
        os.close(descriptor)
        os.close(parent_fd)


def unlink_state(arguments: list[str]) -> None:
    if len(arguments) not in {1, 3}:
        raise ValueError("unlink requires path and optional device/inode")
    path = Path(arguments[0])
    parent_fd, name = split_state_path(path)
    try:
        info = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        if len(arguments) == 3:
            expected = (int(arguments[1]), int(arguments[2]))
            if (info.st_dev, info.st_ino) != expected:
                raise ValueError(f"state inode changed before unlink: {path}")
            quarantine = f".{name}.unlink-{os.getpid()}-{os.urandom(4).hex()}"
            os.rename(name, quarantine, src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
            moved = os.stat(quarantine, dir_fd=parent_fd, follow_symlinks=False)
            if (moved.st_dev, moved.st_ino) != expected:
                try:
                    os.rename(quarantine, name, src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
                except FileExistsError:
                    pass
                raise ValueError(f"state inode changed during unlink: {path}")
            os.unlink(quarantine, dir_fd=parent_fd)
        else:
            os.unlink(name, dir_fd=parent_fd)
    except FileNotFoundError:
        pass
    finally:
        os.close(parent_fd)


def quarantine_state(arguments: list[str]) -> None:
    if len(arguments) != 4:
        raise ValueError("quarantine requires source, destination, device, and inode")
    source, destination = Path(arguments[0]), Path(arguments[1])
    if source.parent != destination.parent:
        raise ValueError("quarantine source and destination must share a directory")
    parent_fd, source_name = split_state_path(source)
    destination_name = destination.name
    expected = int(arguments[2]), int(arguments[3])
    try:
        source_info = os.stat(source_name, dir_fd=parent_fd, follow_symlinks=False)
        if (source_info.st_dev, source_info.st_ino) != expected:
            raise ValueError(f"state inode changed before quarantine: {source}")
        try:
            os.stat(destination_name, dir_fd=parent_fd, follow_symlinks=False)
            raise ValueError(f"quarantine destination already exists: {destination}")
        except FileNotFoundError:
            pass
        os.rename(source_name, destination_name, src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
        moved = os.stat(destination_name, dir_fd=parent_fd, follow_symlinks=False)
        if (moved.st_dev, moved.st_ino) != expected:
            try:
                os.rename(destination_name, source_name, src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
            except FileExistsError:
                pass
            raise ValueError(f"state inode changed during quarantine: {source}")
        os.fsync(parent_fd)
    finally:
        os.close(parent_fd)


def validate_verdict(arguments: list[str]) -> None:
    if len(arguments) != 1:
        raise ValueError("validate-verdict requires path")
    _info, raw = trusted_file_bytes(Path(arguments[0]))
    if len(raw) > 4096:
        raise ValueError("invalid verdict size limit")
    fields: dict[str, str] = {}
    for line in raw.decode("ascii").splitlines():
        if "=" not in line:
            raise ValueError("invalid verdict field")
        key, value = line.split("=", 1)
        if key in fields:
            raise ValueError("invalid verdict duplicate field")
        fields[key] = value
    if set(fields) != {"time", "state", "rc", "markers", "cooldown_until"}:
        raise ValueError("invalid verdict fields")
    try:
        parsed = dt.datetime.fromisoformat(fields["time"].replace("Z", "+00:00"))
    except ValueError as error:
        raise ValueError("invalid verdict timestamp") from error
    now = dt.datetime.now(dt.timezone.utc)
    if parsed.tzinfo is None or abs((parsed.astimezone(dt.timezone.utc) - now).total_seconds()) > 366 * 86400:
        raise ValueError("invalid verdict timestamp range")
    for key, maximum in (("rc", 255), ("markers", 100), ("cooldown_until", 2**63 - 1)):
        text = fields[key]
        if not re.fullmatch(r"[0-9]{1,20}", text) or int(text) > maximum:
            raise ValueError(f"invalid verdict {key}")


def publish(path: Path, payload: bytes) -> None:
    if len(payload) > MAX_STATE_FILE:
        raise ValueError(f"state payload exceeds size limit: {path}")
    parent_fd, name = split_state_path(path)
    temporary = f".{name}.tmp-{os.getpid()}-{os.urandom(4).hex()}"
    descriptor = -1
    try:
        try:
            os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
            raise FileExistsError(path)
        except FileNotFoundError:
            pass
        descriptor = os.open(
            temporary,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
            0o600,
            dir_fd=parent_fd,
        )
        os.write(descriptor, payload)
        os.fsync(descriptor)
        temporary_info = os.fstat(descriptor)
        published_identity = temporary_info.st_dev, temporary_info.st_ino
        os.close(descriptor)
        descriptor = -1
        os.link(temporary, name, src_dir_fd=parent_fd, dst_dir_fd=parent_fd, follow_symlinks=False)
        published = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        if (published.st_dev, published.st_ino) != published_identity:
            raise ValueError(f"publication target changed during create: {path}")
        os.fsync(parent_fd)
    finally:
        if descriptor >= 0:
            os.close(descriptor)
        try:
            os.unlink(temporary, dir_fd=parent_fd)
        except FileNotFoundError:
            pass
        os.close(parent_fd)


def publish_claim(arguments: list[str]) -> None:
    if len(arguments) != 13:
        raise ValueError("lock-v2 claim requires body and queue binding fields")
    directory = Path(arguments[0])
    filename, ordinal, item_id, gate, session, claimed_at, unit, pid = arguments[1:9]
    if not CLAIM_NAME.fullmatch(filename) or not ordinal.isdigit() or ordinal.startswith("0"):
        raise ValueError("unsafe claim filename or ordinal")
    if filename != f"{ordinal}-{session}.claim":
        raise ValueError("claim filename does not match identity")
    if not SAFE_ID.fullmatch(item_id) or not SAFE_ID.fullmatch(gate):
        raise ValueError("unsafe claim id or gate")
    if (
        not SAFE_SESSION.fullmatch(session)
        or session in RESERVED_SESSIONS
        or not pid.isdigit()
        or pid.startswith("0")
    ):
        raise ValueError("unsafe claim session or pid")
    if unit != f"bedlam-nudge-item{ordinal}-{session}":
        raise ValueError("claim unit does not match identity")
    if not CLAIM_TIME.fullmatch(claimed_at):
        raise ValueError("unsafe claim timestamp")
    dt.datetime.fromisoformat(claimed_at.replace("Z", "+00:00"))
    body_sha256, queue_device, queue_inode, queue_sha256 = arguments[9:]
    if (
        not SAFE_HASH.fullmatch(body_sha256)
        or not SAFE_HASH.fullmatch(queue_sha256)
        or not queue_device.isdigit()
        or not queue_inode.isdigit()
    ):
        raise ValueError("unsafe queue binding")
    binding = (
        f"body_sha256={body_sha256}\n"
        f"queue_device={queue_device}\n"
        f"queue_inode={queue_inode}\n"
        f"queue_sha256={queue_sha256}\n"
    )
    payload = (
        "lock-v2\n"
        f"ordinal={ordinal}\n"
        f"id={item_id}\n"
        f"gate={gate}\n"
        "owner=worker\n"
        f"session={session}\n"
        f"claimed_at={claimed_at}\n"
        f"unit={unit}\n"
        f"pid={pid}\n"
        f"{binding}"
    ).encode()
    queue_path = directory.parent / "NEXT.md"
    before = queue_snapshot(queue_path)
    if (str(before["device"]) != queue_device or str(before["inode"]) != queue_inode
            or before["sha256"] != queue_sha256):
        raise ValueError("queue changed before claim publication")
    claim_path = directory / filename
    publish(claim_path, payload)
    after = queue_snapshot(queue_path)
    if (str(after["device"]) != queue_device or str(after["inode"]) != queue_inode
            or after["sha256"] != queue_sha256):
        try:
            claim_info = claim_path.lstat()
            unlink_state([str(claim_path), str(claim_info.st_dev), str(claim_info.st_ino)])
        finally:
            raise ValueError("queue changed during claim publication final CAS")


def queue_snapshot(path: Path) -> dict[str, object | None]:
    result: dict[str, object | None] = {
        "sha256": None,
        "device": None,
        "inode": None,
        "error": None,
    }
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except FileNotFoundError:
        result["error"] = "missing"
        return result
    except OSError as error:
        result["error"] = error.strerror or error.__class__.__name__
        return result
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode):
            result["error"] = "not-regular"
            return result
        digest = hashlib.sha256()
        while chunk := os.read(fd, 1024 * 1024):
            digest.update(chunk)
        result.update(sha256=digest.hexdigest(), device=info.st_dev, inode=info.st_ino)
    finally:
        os.close(fd)
    return result


def classify_queue_change(before: dict[str, object | None], after: dict[str, object | None]) -> str:
    if after["error"] == "missing":
        return "missing-after"
    if before["error"] is not None or after["error"] is not None:
        return "snapshot-error"
    if (before["device"], before["inode"]) != (after["device"], after["inode"]):
        if before["sha256"] == after["sha256"]:
            return "replaced-aba"
        return "replaced"
    if before["sha256"] != after["sha256"]:
        return "modified"
    return "unchanged"


def publish_failure(arguments: list[str]) -> None:
    if len(arguments) not in {10, 12}:
        raise ValueError("failure requires directory, record fields, and optional queue snapshot")
    directory = Path(arguments[0])
    ordinal, item_id, gate, session, kind, reason, evidence, unchanged, timestamp = arguments[1:10]
    if not ordinal.isdigit() or ordinal.startswith("0"):
        raise ValueError("unsafe failure ordinal")
    if not SAFE_ID.fullmatch(item_id) or not SAFE_ID.fullmatch(gate):
        raise ValueError("unsafe failure id or gate")
    if not SAFE_SESSION.fullmatch(session):
        raise ValueError("unsafe failure session")
    trusted_directory(directory, create=True)
    before = json.loads(arguments[10]) if len(arguments) == 12 else None
    after = queue_snapshot(Path(arguments[11])) if len(arguments) == 12 else None
    record = {
        "schema": "nudge-failure-v1",
        "version": 1,
        "ordinal": int(ordinal),
        "id": item_id,
        "gate": gate,
        "owner": "worker",
        "session": session,
        "kind": kind,
        "reason": reason,
        "evidence": evidence,
        "time": timestamp,
        "repair": "required",
        "queue_unchanged": unchanged == "true",
    }
    if before is not None and after is not None:
        record["queue_before"] = before
        record["queue_after"] = after
        record["queue_change"] = classify_queue_change(before, after)
        record["queue_unchanged"] = record["queue_change"] == "unchanged"
    payload = (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()
    publish(directory / f"{session}.json", payload)


def list_failures(arguments: list[str]) -> None:
    if len(arguments) != 1:
        raise ValueError("list-failures requires a directory")
    directory = Path(arguments[0])
    if not directory.exists():
        return
    trusted_directory(directory)
    paths = sorted(directory.glob("*.json"))
    if len(paths) > MAX_FAILURES:
        raise ValueError("failure artifact count exceeds limit")
    for path in paths:
        try:
            info, raw = trusted_file_bytes(path)
            value = json.loads(raw)
            if value.get("schema") != "nudge-failure-v1" or value.get("repair") != "required":
                raise ValueError("invalid schema")
            digest = hashlib.sha256(raw).hexdigest()
            print(
                f"{path.name} session={value['session']} kind={value['kind']} "
                f"id={value['id']} gate={value['gate']} ordinal={value['ordinal']} "
                f"sha256={digest} identity={info.st_dev}:{info.st_ino}"
            )
        except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
            raise ValueError(f"failure artifact {path.name} invalid, size limit, or malformed: {error}") from error


def snapshot_failures(arguments: list[str]) -> None:
    if len(arguments) != 2:
        raise ValueError("snapshot-failures requires a directory and snapshot path")
    directory, snapshot_path = Path(arguments[0]), Path(arguments[1])
    trusted_directory(directory)
    records = []
    for path in sorted(directory.glob("*.json")):
        try:
            info, raw = trusted_file_bytes(path)
        except (OSError, ValueError):
            continue
        try:
            value = json.loads(raw)
        except json.JSONDecodeError:
            continue
        records.append({
            "name": path.name,
            "device": info.st_dev,
            "inode": info.st_ino,
            "sha256": hashlib.sha256(raw).hexdigest(),
            "ordinal": value.get("ordinal"),
            "id": value.get("id"),
            "gate": value.get("gate"),
        })
    payload = (json.dumps(records, sort_keys=True, separators=(",", ":")) + "\n").encode()
    replace_publish(snapshot_path, payload)


def load_queue_items(queue_path: Path) -> set[tuple[int, str, str]]:
    parser_path = Path(__file__).with_name("nudge-free-items.py")
    spec = importlib.util.spec_from_file_location("nudge_free_items", parser_path)
    if spec is None or spec.loader is None:
        raise ValueError("cannot load queue parser")
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
    items = parser.validate_queue(queue_path.read_text(encoding="utf-8"), queue_path)
    return {(int(ordinal), item_id, gate) for ordinal, _status, item_id, gate, _meta in items}


def archive_failures(arguments: list[str]) -> None:
    if len(arguments) != 5:
        raise ValueError("archive-failures requires directory, snapshot, queue, acknowledgement, and remediation commit")
    directory, snapshot_path, queue_path, acknowledgement_path = map(Path, arguments[:4])
    remediation_commit = arguments[4]
    if not re.fullmatch(r"[0-9a-f]{40,64}", remediation_commit):
        raise ValueError("invalid remediation commit")
    root = queue_path.parent.parent
    changed = subprocess.run(
        ["/usr/bin/git", "-C", str(root), "diff-tree", "--no-commit-id", "--name-only", "-r", remediation_commit],
        check=True, capture_output=True, text=True, timeout=30,
    ).stdout.splitlines()
    queue_relative = queue_path.relative_to(root).as_posix()
    if queue_relative not in changed:
        raise ValueError("remediation commit does not establish the queue postcondition")
    if not directory.exists():
        return
    trusted_directory(directory)
    archive = directory / "archive"
    if not archive.exists():
        archive.mkdir(mode=0o700)
    trusted_directory(archive)
    _snapshot_info, snapshot_raw = trusted_file_bytes(snapshot_path)
    snapshots = {entry["name"]: entry for entry in json.loads(snapshot_raw)}
    active_items = load_queue_items(queue_path)
    _acknowledgement_info, acknowledgement_raw = trusted_file_bytes(acknowledgement_path)
    acknowledgement = json.loads(acknowledgement_raw)
    if acknowledgement.get("schema") != "nudge-failure-ack-v1" or not isinstance(acknowledgement.get("records"), list):
        raise ValueError("invalid failure acknowledgement schema")
    acknowledged = {record.get("name"): record for record in acknowledgement["records"]}
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    for path in sorted(directory.glob("*.json")):
        info, raw = trusted_file_bytes(path)
        expected = snapshots.get(path.name)
        ack = acknowledged.get(path.name)
        if expected is None or ack is None or (
            info.st_dev != expected["device"]
            or info.st_ino != expected["inode"]
            or hashlib.sha256(raw).hexdigest() != expected["sha256"]
        ):
            continue
        for field in ("name", "device", "inode", "sha256", "ordinal", "id", "gate"):
            if ack.get(field) != expected.get(field):
                raise ValueError(f"failure acknowledgement mismatch for {path.name}: {field}")
        if ack.get("remediation_commit") != remediation_commit:
            raise ValueError(f"failure acknowledgement remediation commit mismatch for {path.name}")
        identity = (expected["ordinal"], expected["id"], expected["gate"])
        resolution = ack.get("resolution")
        if resolution == "required-empty":
            if active_items:
                raise ValueError(f"required-empty postcondition not met for {path.name}")
        elif resolution == "replaced-task":
            if identity in active_items:
                raise ValueError(f"replaced-task postcondition not met for {path.name}")
        else:
            raise ValueError(f"unsupported failure resolution for {path.name}")
        destination = archive / f"{stamp}-{os.getpid()}-{path.name}"
        os.link(path, destination, follow_symlinks=False)
        linked_info, linked_raw = trusted_file_bytes(destination)
        if ((linked_info.st_dev, linked_info.st_ino) != (expected["device"], expected["inode"])
                or hashlib.sha256(linked_raw).hexdigest() != expected["sha256"]):
            destination.unlink()
            raise ValueError(f"failure artifact raced during archive: {path.name}")
        current_info, current_raw = trusted_file_bytes(path)
        if ((current_info.st_dev, current_info.st_ino) != (expected["device"], expected["inode"])
                or hashlib.sha256(current_raw).hexdigest() != expected["sha256"]):
            destination.unlink()
            raise ValueError(f"failure artifact changed before unlink: {path.name}")
        path.unlink()


def read_claim(arguments: list[str]) -> None:
    if len(arguments) != 1:
        raise ValueError("read-claim requires a path")
    path = Path(arguments[0])
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(descriptor)
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                or info.st_mode & 0o022 or info.st_size > MAX_CLAIM_FILE):
            raise ValueError("unsafe or oversized claim file (size limit)")
        try:
            fcntl.flock(descriptor, fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError:
            pass
        chunks = []
        total = 0
        while chunk := os.read(descriptor, 64 * 1024):
            total += len(chunk)
            if total > MAX_CLAIM_FILE:
                raise ValueError("claim exceeds size limit")
            chunks.append(chunk)
        sys.stdout.buffer.write(b"".join(chunks))
    finally:
        os.close(descriptor)


def claim_owner_exec(arguments: list[str]) -> None:
    if len(arguments) < 6:
        raise ValueError("claim-owner-exec requires directory, reservation, owner, ordinal, session, and command")
    directory = Path(arguments[0])
    reservation, owner, ordinal, session = arguments[1:5]
    if (reservation != f"{ordinal}-{session}.claim" or owner != f"{ordinal}-owner.claim"
            or not CLAIM_NAME.fullmatch(reservation) or not re.fullmatch(r"[1-9][0-9]*", ordinal)
            or not SAFE_SESSION.fullmatch(session)):
        raise ValueError("unsafe owner claim identity")
    directory_fd = open_directory(directory)
    claim_fd = -1
    owner_fd = -1
    reservation_identity: tuple[int, int] | None = None
    try:
        claim_fd = os.open(reservation, os.O_RDWR | os.O_NOFOLLOW, dir_fd=directory_fd)
        info = os.fstat(claim_fd)
        reservation_identity = info.st_dev, info.st_ino
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                or info.st_mode & 0o022 or info.st_size > MAX_CLAIM_FILE):
            raise ValueError("unsafe reservation claim")
        fcntl.flock(claim_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)

        # Keep the reservation inode and directory pinned while publication is
        # deliberately exposed to the cooperative `ln` test seam.
        result = subprocess.run(
            ["ln", reservation, owner],
            cwd=f"/proc/self/fd/{directory_fd}", pass_fds=(claim_fd, directory_fd),
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, check=False,
        )
        if result.returncode:
            raise ValueError("canonical owner claim already exists")
        owner_fd = os.open(owner, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_fd)
        owner_info = os.fstat(owner_fd)
        if ((owner_info.st_dev, owner_info.st_ino) != (info.st_dev, info.st_ino)
                or not stat.S_ISREG(owner_info.st_mode)):
            raise ValueError("owner claim changed before authoritative open")
        os.unlink(reservation, dir_fd=directory_fd)
        os.fsync(directory_fd)
        if claim_fd != 8:
            os.dup2(claim_fd, 8, inheritable=True)
            os.close(claim_fd)
            claim_fd = 8
        else:
            os.set_inheritable(claim_fd, True)
        environment = dict(os.environ)
        environment["NUDGE_OWNER_FD"] = "8"
        environment["NUDGE_CLAIM_IDENTITY"] = f"{info.st_dev}:{info.st_ino}"
        os.execvpe(arguments[5], arguments[5:], environment)
    finally:
        if owner_fd >= 0:
            os.close(owner_fd)
        if claim_fd >= 0:
            os.close(claim_fd)
        if reservation_identity is not None:
            try:
                current = os.stat(reservation, dir_fd=directory_fd, follow_symlinks=False)
                if (current.st_dev, current.st_ino) == reservation_identity:
                    os.unlink(reservation, dir_fd=directory_fd)
            except FileNotFoundError:
                pass
        os.close(directory_fd)


def _claim_body_valid(raw: bytes, name: str) -> bool:
    try:
        lines = raw.decode("ascii").splitlines()
    except UnicodeError:
        return False
    if lines and lines[0].startswith("lock-v1 "):
        return bool(re.search(r"^lock-v1 worker [A-Za-z0-9][A-Za-z0-9._-]* owns queue item [1-9][0-9]*$", raw.decode("ascii"), re.MULTILINE))
    if not lines or lines[0] != "lock-v2" or lines.count("lock-v2") != 1:
        return False
    fields: dict[str, str] = {}
    for line in lines[1:]:
        if "=" not in line:
            return False
        key, value = line.split("=", 1)
        if not value or key in fields:
            return False
        fields[key] = value
    required = {"ordinal", "id", "gate", "owner", "session", "claimed_at", "unit", "pid",
                "body_sha256", "queue_device", "queue_inode", "queue_sha256"}
    if set(fields) != required:
        return False
    try:
        claimed = dt.datetime.fromisoformat(fields["claimed_at"].replace("Z", "+00:00"))
    except ValueError:
        return False
    ordinal = name.split("-", 1)[0]
    return (
        fields["ordinal"] == ordinal and fields["owner"] == "worker"
        and SAFE_ID.fullmatch(fields["id"]) is not None
        and SAFE_ID.fullmatch(fields["gate"]) is not None
        and SAFE_SESSION.fullmatch(fields["session"]) is not None
        and fields["unit"] == f"bedlam-nudge-item{ordinal}-{fields['session']}"
        and re.fullmatch(r"[1-9][0-9]*", fields["pid"]) is not None
        and SAFE_HASH.fullmatch(fields["body_sha256"]) is not None
        and SAFE_HASH.fullmatch(fields["queue_sha256"]) is not None
        and claimed.tzinfo is not None
    )


def reap_claims(arguments: list[str]) -> None:
    if len(arguments) != 6:
        raise ValueError("reap-claims requires directory, log, and four TTL values")
    directory, log_path = Path(arguments[0]), Path(arguments[1])
    dead_ttl, reservation_ttl, legacy_ttl, malformed_ttl = map(int, arguments[2:])
    if min(dead_ttl, reservation_ttl, legacy_ttl, malformed_ttl) < 0:
        raise ValueError("claim TTL must be nonnegative")
    directory_fd = open_directory(directory)
    try:
        entries = list(os.scandir(directory_fd))
        claim_names = sorted(entry.name for entry in entries if entry.name.endswith(".claim"))
        if len(claim_names) > MAX_CLAIMS:
            raise ValueError("claim count exceeds limit")
        now = time.time()
        messages: list[str] = []
        for name in claim_names:
            fd = -1
            try:
                fd = os.open(name, os.O_RDWR | os.O_NOFOLLOW, dir_fd=directory_fd)
                info = os.fstat(fd)
                if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_CLAIM_FILE:
                    raise ValueError("unsafe or oversized claim")
                age = now - info.st_mtime
                locked = False
                try:
                    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    locked = True
                if locked:
                    raw = os.pread(fd, min(info.st_size, MAX_CLAIM_FILE + 1), 0)
                    if name.endswith("-owner.claim") and raw.startswith(b"lock-v2") \
                            and not _claim_body_valid(raw, name):
                        messages.append(f"malformed lock-v2 claim {name} remains actively locked; preserving for repair")
                    os.utime(fd, (int(now), int(now)))
                    continue
                raw = b""
                while chunk := os.read(fd, 64 * 1024):
                    raw += chunk
                    if len(raw) > MAX_CLAIM_FILE:
                        raise ValueError("oversized claim")
                is_owner = name.endswith("-owner.claim")
                valid = _claim_body_valid(raw, name)
                if age < -300:
                    ttl, kind = 0, "future-dated"
                elif not valid and is_owner:
                    ttl, kind = malformed_ttl, "malformed"
                elif is_owner and raw.startswith((b"lock-v1", b"lock-v2")):
                    ttl, kind = dead_ttl, "dead-worker"
                elif is_owner:
                    ttl, kind = legacy_ttl, "legacy-owner"
                else:
                    ttl, kind = reservation_ttl, "reservation"
                if age > ttl or age < -300:
                    expected = info.st_dev, info.st_ino
                    quarantine = f".quarantine-{os.getpid()}-{os.urandom(8).hex()}-{name}"
                    os.rename(name, quarantine, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
                    moved = os.stat(quarantine, dir_fd=directory_fd, follow_symlinks=False)
                    locked = os.fstat(fd)
                    if ((moved.st_dev, moved.st_ino) != expected
                            or (locked.st_dev, locked.st_ino) != expected):
                        try:
                            os.rename(
                                quarantine, name,
                                src_dir_fd=directory_fd, dst_dir_fd=directory_fd,
                            )
                        except FileExistsError as restore_error:
                            raise ValueError(
                                f"claim {name} raced and replacement could not be restored"
                            ) from restore_error
                        raise ValueError(f"claim {name} changed during quarantine")
                    if kind == "malformed":
                        messages.append(f"quarantined stale malformed claim {name} (age {int(age)}s)")
                    else:
                        os.unlink(quarantine, dir_fd=directory_fd)
                        messages.append(f"reaped stale {kind} claim {name} (age {int(age)}s)")
            except OSError as error:
                messages.append(f"unable to safely reap invalid claim {name}: {error}")
            finally:
                if fd >= 0:
                    os.close(fd)
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)
    for message in messages:
        append_text([str(log_path), f"{dt.datetime.now().astimezone().isoformat()} {message}\n"])


def signal_descendants(arguments: list[str]) -> None:
    if len(arguments) != 2 or not re.fullmatch(r"[1-9][0-9]*", arguments[0]):
        raise ValueError("signal-descendants requires PID and TERM|KILL|INT|HUP")
    signum = {
        "TERM": signal.SIGTERM,
        "KILL": signal.SIGKILL,
        "INT": signal.SIGINT,
        "HUP": signal.SIGHUP,
    }.get(arguments[1])
    if signum is None:
        raise ValueError("unsupported descendant signal")
    root = int(arguments[0])
    descendants: dict[int, int] = {}
    for _ in range(4):
        pending = [(root, 0)]
        seen = {root}
        while pending:
            parent, depth = pending.pop()
            try:
                raw = Path(f"/proc/{parent}/task/{parent}/children").read_text(
                    encoding="ascii"
                )
            except (FileNotFoundError, ProcessLookupError, PermissionError):
                continue
            for value in raw.split():
                pid = int(value)
                if pid in seen:
                    continue
                seen.add(pid)
                descendants[pid] = max(descendants.get(pid, 0), depth + 1)
                if len(descendants) > 4096:
                    raise ValueError("descendant process count exceeds limit")
                pending.append((pid, depth + 1))
        if len(seen) == 1:
            break
    for pid, _depth in sorted(descendants.items(), key=lambda item: item[1], reverse=True):
        try:
            os.kill(pid, signum)
        except ProcessLookupError:
            pass


def verify_completion(arguments: list[str]) -> None:
    if len(arguments) != 2:
        raise ValueError("verify-completion requires artifact and project root")
    artifact, root = Path(arguments[0]), Path(arguments[1])
    info = artifact.lstat()
    if (
        artifact.is_symlink()
        or not stat.S_ISREG(info.st_mode)
        or info.st_uid != os.geteuid()
        or stat.S_IMODE(info.st_mode) != 0o600
    ):
        raise ValueError("unsafe completion artifact")
    value = json.loads(artifact.read_text(encoding="utf-8"))
    head = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    gates_path = root / "docs/required-gates.toml"
    if not gates_path.is_file() or gates_path.is_symlink():
        raise ValueError("required-gates manifest is missing or unsafe")
    gates = hashlib.sha256(gates_path.read_bytes()).hexdigest()
    validation = value.get("offline_validation", {})
    if (
        value.get("schema") != "plan-complete-v1"
        or value.get("producer") != "controller"
        or value.get("head") != head
        or value.get("required_gates_sha256") != gates
        or validation.get("status") != "passed"
        or validation.get("validated_at_head") != head
        or validation.get("bounded") is not True
    ):
        raise ValueError("completion proof is stale or validation failed")


def print_queue_snapshot(arguments: list[str]) -> None:
    if len(arguments) != 1:
        raise ValueError("queue-snapshot requires a path")
    print(json.dumps(queue_snapshot(Path(arguments[0])), sort_keys=True, separators=(",", ":")))


def claims_snapshot(path: Path) -> dict[str, object]:
    directory_fd = open_directory(path)
    try:
        directory_info = os.fstat(directory_fd)
        names = sorted(entry.name for entry in os.scandir(directory_fd) if entry.name.endswith(".claim"))
        if len(names) > MAX_CLAIMS:
            raise ValueError("claim count exceeds limit")
        entries = []
        for name in names:
            info = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            entries.append([name, info.st_dev, info.st_ino])
        final_info = os.fstat(directory_fd)
        if (directory_info.st_dev, directory_info.st_ino) != (final_info.st_dev, final_info.st_ino):
            raise ValueError("claims directory identity changed during snapshot")
        return {
            "device": final_info.st_dev,
            "inode": final_info.st_ino,
            "mtime_ns": final_info.st_mtime_ns,
            "ctime_ns": final_info.st_ctime_ns,
            "entries": entries,
        }
    finally:
        os.close(directory_fd)


def bounded_file_sha256(path: Path, maximum: int) -> str:
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_size > maximum:
            raise ValueError(f"unsafe or oversized proof input: {path}")
        digest = hashlib.sha256()
        total = 0
        while chunk := os.read(descriptor, 1024 * 1024):
            total += len(chunk)
            if total > maximum:
                raise ValueError(f"proof input exceeds size limit: {path}")
            digest.update(chunk)
        return digest.hexdigest()
    finally:
        os.close(descriptor)


def source_git_directory(root: Path) -> tuple[Path, Path]:
    marker = root / ".git"
    marker_info = marker.lstat()
    if stat.S_ISDIR(marker_info.st_mode):
        git_directory = marker
    elif stat.S_ISREG(marker_info.st_mode) and marker_info.st_size <= 4096:
        raw = marker.read_text(encoding="utf-8").strip()
        if not raw.startswith("gitdir: "):
            raise ValueError("invalid Git directory marker")
        git_directory = (root / raw[8:]).resolve()
    else:
        raise ValueError("unsafe Git directory marker")
    git_info = git_directory.lstat()
    if git_directory.is_symlink() or not stat.S_ISDIR(git_info.st_mode):
        raise ValueError("unsafe source Git directory")
    common_marker = git_directory / "commondir"
    if common_marker.exists():
        raw = common_marker.read_text(encoding="utf-8").strip()
        common_directory = (git_directory / raw).resolve()
    else:
        common_directory = git_directory
    common_info = common_directory.lstat()
    if common_directory.is_symlink() or not stat.S_ISDIR(common_info.st_mode):
        raise ValueError("unsafe common Git directory")
    return git_directory, common_directory


def read_git_head(root: Path) -> tuple[str, Path]:
    git_directory, common_directory = source_git_directory(root)
    head_raw = (git_directory / "HEAD").read_text(encoding="ascii").strip()
    if re.fullmatch(r"[0-9a-f]{40,64}", head_raw):
        return head_raw, common_directory / "objects"
    if not head_raw.startswith("ref: "):
        raise ValueError("invalid source HEAD")
    reference = head_raw[5:]
    if not re.fullmatch(r"refs/[A-Za-z0-9._/-]+", reference) or ".." in Path(reference).parts:
        raise ValueError("unsafe source HEAD reference")
    loose = git_directory / reference
    if not loose.exists() and common_directory != git_directory:
        loose = common_directory / reference
    if loose.exists():
        head = loose.read_text(encoding="ascii").strip()
    else:
        head = ""
        packed = common_directory / "packed-refs"
        for line in packed.read_text(encoding="ascii").splitlines():
            if line.startswith(("#", "^")):
                continue
            fields = line.split(" ", 1)
            if len(fields) == 2 and fields[1] == reference:
                head = fields[0]
                break
    if not re.fullmatch(r"[0-9a-f]{40,64}", head):
        raise ValueError("source HEAD reference is missing or invalid")
    objects = common_directory / "objects"
    objects_info = objects.lstat()
    if objects.is_symlink() or not stat.S_ISDIR(objects_info.st_mode):
        raise ValueError("unsafe source object directory")
    return head, objects


def inert_git(checkout: Path, environment: dict[str, str], *arguments: str) -> str:
    result = subprocess.run(
        ["/usr/bin/git", "-c", "core.hooksPath=/dev/null", "-C", str(checkout), *arguments],
        check=True, capture_output=True, text=True, timeout=120, env=environment,
    )
    return result.stdout.strip()


def complete_from_head(arguments: list[str]) -> None:
    if len(arguments) != 3:
        raise ValueError("complete-from-head requires root, report, and completion output")
    root, report_path, completion_path = map(Path, arguments)
    queue_path = root / ".state/NEXT.md"
    claims_path = root / ".state/claims"
    parser_path = Path(__file__).with_name("nudge-free-items.py")
    spec = importlib.util.spec_from_file_location("nudge_free_items", parser_path)
    if spec is None or spec.loader is None:
        raise ValueError("cannot load queue parser")
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)

    before = queue_snapshot(queue_path)
    claims_before = claims_snapshot(claims_path)
    _queue_info, queue_raw = trusted_file_bytes(queue_path)
    items = parser.validate_queue(queue_raw.decode("utf-8"), queue_path)
    if items or claims_before["entries"]:
        raise ValueError("queued work or claims block completion validation")
    head, objects = read_git_head(root)
    checkout = Path(tempfile.mkdtemp(prefix="bedlam-completion-", dir="/tmp/opencode"))
    output = checkout.parent / f".{checkout.name}-output"
    output.mkdir(mode=0o700)
    archive_path = output / "head.tar"
    temporary_report = output / "report.json"
    temporary_completion = output / "completion.json"
    validation_completion: dict[str, object] | None = None
    corpus_proof: list[dict[str, str]] = []
    try:
        git_environment = {
            "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_TERMINAL_PROMPT": "0", "HOME": "/tmp/opencode", "LC_ALL": "C",
        }
        subprocess.run(
            ["/usr/bin/git", "-c", "core.hooksPath=/dev/null", "init", "-q",
             "--template=", str(checkout)],
            check=True, timeout=30, env=git_environment,
        )
        alternates = checkout / ".git/objects/info/alternates"
        alternates.write_text(str(objects.resolve()) + "\n", encoding="utf-8")
        inert_git(checkout, git_environment, "update-ref", "HEAD", head)
        tree = inert_git(checkout, git_environment, "rev-parse", "HEAD^{tree}")
        inert_git(
            checkout, git_environment, "archive", "--format=tar", "-o",
            str(archive_path), head,
        )
        with tarfile.open(archive_path, "r") as archive:
            for member in archive.getmembers():
                destination = (checkout / member.name).resolve()
                if not destination.is_relative_to(checkout) or member.issym() or member.islnk():
                    raise ValueError("unsafe path or link in committed HEAD archive")
            archive.extractall(checkout, filter="data")
        manifest_raw = (checkout / "MANIFEST.sha256").read_bytes()
        source_bindings: list[tuple[Path, str]] = []
        for line_number, line in enumerate(manifest_raw.decode("utf-8").splitlines(), 1):
            match = re.fullmatch(r"([0-9a-f]{64})  (\S(?:.*\S)?)", line)
            if not match:
                raise ValueError(f"MANIFEST.sha256 line {line_number} is malformed")
            expected, relative = match.groups()
            corpus_proof.append({"path": relative, "sha256": expected})
            relative_path = Path(relative)
            if relative_path.is_absolute() or ".." in relative_path.parts:
                raise ValueError(f"unsafe corpus path in MANIFEST.sha256: {relative}")
            destination = checkout / relative_path
            if destination.exists():
                if hashlib.sha256(destination.read_bytes()).hexdigest() != expected:
                    raise ValueError(f"committed MANIFEST mismatch: {relative}")
                continue
            source = root / relative_path
            source_fd = os.open(source, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                source_info = os.fstat(source_fd)
                if not stat.S_ISREG(source_info.st_mode) or source_info.st_size > 128 * 1024 * 1024:
                    raise ValueError(f"unsafe or oversized external corpus: {relative}")
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination_fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o400)
                digest = hashlib.sha256()
                try:
                    while chunk := os.read(source_fd, 1024 * 1024):
                        digest.update(chunk)
                        os.write(destination_fd, chunk)
                    os.fsync(destination_fd)
                finally:
                    os.close(destination_fd)
                if digest.hexdigest() != expected:
                    raise ValueError(f"external source corpus mismatch: {relative}")
                source_bindings.append((source, expected))
            finally:
                os.close(source_fd)

        target = checkout / "target"
        target.mkdir(mode=0o700)
        # Gate-declared writable scratch directories must exist before the
        # checkout is sealed read-only: bwrap can only bind over mountpoints
        # that exist, and the validator re-validates each declaration
        # (gitignored, untracked, no tracked content beneath) itself. A
        # checkout without the tracked manifest has no declarations (and
        # the validator fails closed on its own terms in that case).
        gates_manifest_path = checkout / "docs" / "required-gates.toml"
        if gates_manifest_path.is_file():
            gates_manifest = tomllib.loads(
                gates_manifest_path.read_text(encoding="utf-8")
            )
            for gate in gates_manifest.get("gate", []):
                for relative in gate.get("writable", []):
                    writable_path = Path(relative)
                    if writable_path.is_absolute() or ".." in writable_path.parts:
                        raise ValueError(f"unsafe writable path in required-gates: {relative}")
                    (checkout / writable_path).mkdir(mode=0o700, parents=True, exist_ok=True)

        # The copied validation basis is read-only; writable outputs live outside it.
        for current_root, directories, files in os.walk(checkout, topdown=False):
            for filename in files:
                path = Path(current_root) / filename
                mode = stat.S_IMODE(path.stat().st_mode)
                path.chmod(mode & ~0o222)
            for directory in directories:
                path = Path(current_root) / directory
                mode = stat.S_IMODE(path.stat().st_mode)
                path.chmod(mode & ~0o222)
        validator = checkout / "tools/validate-required-gates.py"
        if not stat.S_ISREG(validator.lstat().st_mode):
            raise ValueError("HEAD validator is not a regular file")
        result = subprocess.run(
            [str(validator), "--root", str(checkout), "--report", str(temporary_report),
             "--completion-output", str(temporary_completion)],
            stdin=subprocess.DEVNULL, timeout=1800, check=False,
            env={"HOME": "/tmp/opencode", "LANG": "C", "LC_ALL": "C", "TZ": "UTC"},
        )
        required_gates_sha256 = hashlib.sha256(
            (checkout / "docs/required-gates.toml").read_bytes()
        ).hexdigest()
        validator_sha256 = hashlib.sha256(validator.read_bytes()).hexdigest()
        for source, expected in source_bindings:
            source_fd = os.open(source, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                digest = hashlib.sha256()
                while chunk := os.read(source_fd, 1024 * 1024):
                    digest.update(chunk)
                if digest.hexdigest() != expected:
                    raise ValueError(f"external source corpus changed during validation: {source}")
            finally:
                os.close(source_fd)
        after = queue_snapshot(queue_path)
        _queue_after_info, queue_after_raw = trusted_file_bytes(queue_path)
        items_after = parser.validate_queue(queue_after_raw.decode("utf-8"), queue_path)
        current_head, _current_objects = read_git_head(root)
        current_tree = inert_git(checkout, git_environment, "rev-parse", f"{current_head}^{{tree}}")
        current_claims = claims_snapshot(claims_path)
        if (after != before or items_after or current_claims != claims_before
                or current_head != head or current_tree != tree):
            raise ValueError("completion basis changed during validation")
        if temporary_report.exists():
            replace_publish(report_path, temporary_report.read_bytes())
        if result.returncode == 0 and temporary_completion.exists():
            completion = json.loads(temporary_completion.read_bytes())
            validation_completion = dict(completion)
            completion["decision_basis"] = {
                "queue": before,
                "claims": claims_before,
                "head": head,
                "tree": tree,
            }
            replace_publish(
                completion_path,
                (json.dumps(completion, sort_keys=True, separators=(",", ":")) + "\n").encode(),
            )
        if result.returncode != 0:
            raise ValueError(f"HEAD required-gates validator failed rc={result.returncode}")
        final_queue = queue_snapshot(queue_path)
        _final_queue_info, final_queue_raw = trusted_file_bytes(queue_path)
        final_items = parser.validate_queue(final_queue_raw.decode("utf-8"), queue_path)
        final_head, _final_objects = read_git_head(root)
        final_tree = inert_git(checkout, git_environment, "rev-parse", f"{final_head}^{{tree}}")
        final_claims = claims_snapshot(claims_path)
        if (final_queue != before or final_items or final_claims != claims_before
                or final_head != head or final_tree != tree):
            if completion_path.exists() or completion_path.is_symlink():
                completion_info = completion_path.lstat()
                unlink_state([
                    str(completion_path),
                    str(completion_info.st_dev),
                    str(completion_info.st_ino),
                ])
            raise ValueError("completion basis changed during artifact publication")
        if validation_completion is None:
            raise ValueError("HEAD validator did not produce an in-process completion proof")
        completion_info, completion_raw = trusted_file_bytes(completion_path)
        proof = {
            "schema": "completion-proof-v1",
            "decision_generation": os.urandom(32).hex(),
            "queue": before,
            "claims": claims_before,
            "head": head,
            "tree": tree,
            "manifest_sha256": hashlib.sha256(manifest_raw).hexdigest(),
            "required_gates_sha256": required_gates_sha256,
            "validator_sha256": validator_sha256,
            "corpus": corpus_proof,
            "validation": validation_completion,
            "artifact": {
                "device": completion_info.st_dev,
                "inode": completion_info.st_ino,
                "sha256": hashlib.sha256(completion_raw).hexdigest(),
            },
        }
        print(json.dumps(proof, sort_keys=True, separators=(",", ":")))
    finally:
        for current_root, directories, files in os.walk(checkout):
            for name in [*directories, *files]:
                try:
                    (Path(current_root) / name).chmod(0o700 if name in directories else 0o600)
                except OSError:
                    pass
        shutil.rmtree(checkout, ignore_errors=True)
        shutil.rmtree(output, ignore_errors=True)


def accept_completion(arguments: list[str]) -> None:
    if len(arguments) != 3:
        raise ValueError("accept-completion requires root, completion artifact, and log")
    root, completion_path, log_path = map(Path, arguments)
    queue_path = root / ".state/NEXT.md"
    claims_path = root / ".state/claims"
    proof_raw = sys.stdin.buffer.read(MAX_STATE_FILE + 1)
    if len(proof_raw) > MAX_STATE_FILE:
        raise ValueError("completion proof exceeds size limit")
    proof = json.loads(proof_raw)
    required_fields = {
        "schema", "decision_generation", "queue", "claims", "head", "tree",
        "manifest_sha256", "required_gates_sha256", "validator_sha256", "corpus",
        "validation", "artifact",
    }
    if (not isinstance(proof, dict) or set(proof) != required_fields
            or proof.get("schema") != "completion-proof-v1"
            or not re.fullmatch(r"[0-9a-f]{64}", str(proof.get("decision_generation", "")))):
        raise ValueError("invalid in-process completion proof")
    completion_info, completion_raw = trusted_file_bytes(completion_path)
    artifact = proof.get("artifact")
    if not isinstance(artifact, dict) or artifact != {
        "device": completion_info.st_dev,
        "inode": completion_info.st_ino,
        "sha256": hashlib.sha256(completion_raw).hexdigest(),
    }:
        unlink_state([
            str(completion_path), str(completion_info.st_dev), str(completion_info.st_ino),
        ])
        raise ValueError("completion output artifact changed after validation")

    parser_path = Path(__file__).with_name("nudge-free-items.py")
    spec = importlib.util.spec_from_file_location("nudge_free_items", parser_path)
    if spec is None or spec.loader is None:
        raise ValueError("cannot load queue parser")
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
    _queue_info, queue_raw = trusted_file_bytes(queue_path)
    current_items = parser.validate_queue(queue_raw.decode("utf-8"), queue_path)
    current_claims = claims_snapshot(claims_path)
    if current_items or current_claims["entries"]:
        raise ValueError("queued work or claims block completion acceptance")

    current_head, objects = read_git_head(root)
    git_environment = {
        "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_TERMINAL_PROMPT": "0", "HOME": "/tmp/opencode", "LC_ALL": "C",
    }
    with tempfile.TemporaryDirectory(prefix="bedlam-completion-accept-", dir="/tmp/opencode") as temporary:
        inert = Path(temporary)
        subprocess.run(
            ["/usr/bin/git", "-c", "core.hooksPath=/dev/null", "init", "-q",
             "--template=", str(inert)],
            check=True, timeout=30, env=git_environment,
        )
        alternates = inert / ".git/objects/info/alternates"
        alternates.write_text(str(objects.resolve()) + "\n", encoding="utf-8")
        current_tree = inert_git(inert, git_environment, "rev-parse", f"{current_head}^{{tree}}")
    validation = proof.get("validation")
    offline = validation.get("offline_validation") if isinstance(validation, dict) else None
    if (
        not isinstance(validation, dict)
        or validation.get("schema") != "plan-complete-v1"
        or validation.get("producer") != "controller"
        or validation.get("head") != proof.get("head")
        or validation.get("head_tree") != proof.get("tree")
        or validation.get("required_gates_sha256") != proof.get("required_gates_sha256")
        or validation.get("validator_sha256") != proof.get("validator_sha256")
        or not isinstance(offline, dict)
        or offline.get("status") != "passed"
        or offline.get("bounded") is not True
        or offline.get("validated_at_head") != proof.get("head")
    ):
        raise ValueError("invalid HEAD validator completion proof")
    if bounded_file_sha256(root / "MANIFEST.sha256", MAX_STATE_FILE) != proof.get("manifest_sha256"):
        raise ValueError("completion manifest changed after validation")
    if bounded_file_sha256(root / "docs/required-gates.toml", MAX_STATE_FILE) != proof.get("required_gates_sha256"):
        raise ValueError("required gates changed after validation")
    if bounded_file_sha256(root / "tools/validate-required-gates.py", 16 * 1024 * 1024) != proof.get("validator_sha256"):
        raise ValueError("required-gates validator changed after validation")
    corpus = proof.get("corpus")
    if not isinstance(corpus, list):
        raise ValueError("invalid completion corpus proof")
    for entry in corpus:
        if not isinstance(entry, dict) or set(entry) != {"path", "sha256"}:
            raise ValueError("invalid completion corpus entry")
        relative = entry.get("path")
        expected = entry.get("sha256")
        if (not isinstance(relative, str) or not isinstance(expected, str)
                or not SAFE_HASH.fullmatch(expected)):
            raise ValueError("invalid completion corpus binding")
        relative_path = Path(relative)
        if relative_path.is_absolute() or ".." in relative_path.parts:
            raise ValueError("unsafe completion corpus path")
        if bounded_file_sha256(root / relative_path, 128 * 1024 * 1024) != expected:
            raise ValueError(f"completion corpus changed after validation: {relative}")
    final_claims = claims_snapshot(claims_path)
    accepted = (
        proof.get("queue") == queue_snapshot(queue_path)
        and not final_claims["entries"] and proof.get("claims") == final_claims
        and proof.get("head") == current_head
        and proof.get("tree") == current_tree
    )
    if not accepted:
        unlink_state([
            str(completion_path), str(completion_info.st_dev), str(completion_info.st_ino),
        ])
        raise ValueError("completion decision basis changed after helper publication")
    append_text([
        str(log_path),
        f"{dt.datetime.now().astimezone().isoformat()} all required P0-P7 gates passed fresh bounded offline validation\n",
    ])
    print(json.dumps({"schema": "completion-decision-v1", "status": "accepted", "head": current_head}))


def main(arguments: list[str]) -> int:
    if len(arguments) < 2:
        return 64
    actions = {
        "append-text": append_text,
        "append-file": append_file,
        "claim-owner-exec": claim_owner_exec,
        "publish-claim": publish_claim,
        "publish-failure": publish_failure,
        "quarantine": quarantine_state,
        "list-failures": list_failures,
        "snapshot-failures": snapshot_failures,
        "archive-failures": archive_failures,
        "complete-from-head": complete_from_head,
        "accept-completion": accept_completion,
        "create-text": create_text,
        "ensure-dir": ensure_directory,
        "exec-output": exec_output,
        "read-claim": read_claim,
        "reap-claims": reap_claims,
        "signal-descendants": signal_descendants,
        "verify-completion": verify_completion,
        "queue-snapshot": print_queue_snapshot,
        "read-int": read_int_command,
        "read-text": read_text,
        "retain-tail": retain_tail,
        "read-fields": read_fields,
        "touch": touch_state,
        "unlink": unlink_state,
        "validate-verdict": validate_verdict,
        "write-text": write_text,
        "run-output": run_output,
    }
    try:
        actions[arguments[1]](arguments[2:])
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        print(f"nudge state error: {error}", file=sys.stderr)
        return 75
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
