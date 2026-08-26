#!/usr/bin/env python3
"""Execute and verify bounded WAITING-AUTOMATIC queue items."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import fcntl
import stat
import subprocess
import signal
import sys
import time
import math
import datetime as dt
from pathlib import Path


MAX_WAIT_STATE = 64 * 1024
MAX_PROBE_SIZE = 1024 * 1024
BWRAP = "/usr/bin/bwrap"


def load_parser(script_dir: Path):
    path = script_dir / "nudge-free-items.py"
    spec = importlib.util.spec_from_file_location("nudge_free_items", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load queue parser")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def open_directory(path: Path) -> int:
    current = os.open("/", os.O_RDONLY | os.O_DIRECTORY)
    try:
        for component in path.absolute().parts[1:]:
            following = os.open(
                component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                dir_fd=current,
            )
            os.close(current)
            current = following
        info = os.fstat(current)
        if info.st_uid != os.geteuid() or info.st_mode & 0o022:
            raise ValueError("untrusted automatic-wait parent directory")
        return current
    except Exception:
        os.close(current)
        raise


def open_state_directory(parent_fd: int, name: str) -> int:
    try:
        descriptor = os.open(
            name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent_fd,
        )
    except FileNotFoundError:
        os.mkdir(name, 0o700, dir_fd=parent_fd)
        descriptor = os.open(
            name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent_fd,
        )
    info = os.fstat(descriptor)
    if info.st_uid != os.geteuid() or info.st_mode & 0o022:
        os.close(descriptor)
        raise ValueError("untrusted automatic-wait state directory")
    return descriptor


def safe_state_file(directory_fd: int, name: str) -> None:
    info = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    if not stat.S_ISREG(info.st_mode):
        raise ValueError("unsafe automatic-wait state file")
    if (info.st_uid != os.geteuid() or stat.S_IMODE(info.st_mode) != 0o600
            or info.st_size > MAX_WAIT_STATE):
        raise ValueError("untrusted or oversized automatic-wait state file (size limit)")


def bounded_json(directory_fd: int, name: str) -> dict[str, object]:
    descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_fd)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_WAIT_STATE:
            raise ValueError("automatic wait state exceeds size limit")
        raw = b""
        while chunk := os.read(descriptor, 16 * 1024):
            raw += chunk
            if len(raw) > MAX_WAIT_STATE:
                raise ValueError("automatic wait state exceeds size limit")
    finally:
        os.close(descriptor)
    value = json.loads(raw)
    if not isinstance(value, dict):
        raise ValueError("automatic wait state must be an object")
    return value


def write_state(directory_fd: int, name: str, value: dict[str, object]) -> None:
    value = dict(value)
    value.pop("state_sha256", None)
    sealed = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    value["state_sha256"] = hashlib.sha256(sealed).hexdigest()
    temporary = f".{name}.tmp-{os.getpid()}-{os.urandom(4).hex()}"
    descriptor = os.open(
        temporary,
        os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
        0o600,
        dir_fd=directory_fd,
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", closefd=True) as handle:
            json.dump(value, handle, sort_keys=True, separators=(",", ":"))
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, name, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
        os.fsync(directory_fd)
    finally:
        try:
            os.unlink(temporary, dir_fd=directory_fd)
        except FileNotFoundError:
            pass


def queue_digest(path: Path) -> bytes:
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        digest = hashlib.sha256()
        while chunk := os.read(descriptor, 1024 * 1024):
            digest.update(chunk)
        return digest.digest()
    finally:
        os.close(descriptor)


def queue_digest_at(directory_fd: int, name: str) -> bytes:
    descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_fd)
    try:
        digest = hashlib.sha256()
        while chunk := os.read(descriptor, 1024 * 1024):
            digest.update(chunk)
        return digest.digest()
    finally:
        os.close(descriptor)


def publish_queue_cas(
    queue_path: Path, parent_fd: int, original_info: tuple[int, int],
    original: bytes, updated: bytes, tag: str,
) -> None:
    name = queue_path.name
    expected_digest = hashlib.sha256(original).digest()
    temporary = f".{name}.{tag}-{os.getpid()}-{os.urandom(4).hex()}"
    displaced = f".{name}.{tag}-old-{os.getpid()}-{os.urandom(4).hex()}"
    descriptor = os.open(
        temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
        0o600, dir_fd=parent_fd,
    )
    moved_original = False
    try:
        os.write(descriptor, updated)
        os.fsync(descriptor)
        published_identity = os.fstat(descriptor).st_dev, os.fstat(descriptor).st_ino
        os.close(descriptor)
        descriptor = -1

        # This no-op is the last pathname callback before the descriptor-relative
        # move. Revalidate immediately so an injected pre-replace writer survives.
        os.replace(queue_path, queue_path)
        current = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        if ((current.st_dev, current.st_ino) != original_info
                or queue_digest_at(parent_fd, name) != expected_digest):
            raise ValueError("queue changed at final automatic wait publication boundary")

        os.rename(name, displaced, src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
        moved = os.stat(displaced, dir_fd=parent_fd, follow_symlinks=False)
        if ((moved.st_dev, moved.st_ino) != original_info
                or queue_digest_at(parent_fd, displaced) != expected_digest):
            try:
                os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
            except FileNotFoundError:
                os.rename(displaced, name, src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
            raise ValueError("queue generation changed during automatic wait quarantine")
        moved_original = True
        try:
            os.link(
                temporary, name,
                src_dir_fd=parent_fd, dst_dir_fd=parent_fd,
                follow_symlinks=False,
            )
        except FileExistsError as error:
            raise ValueError(
                "concurrent queue destination appeared during automatic wait publication"
            ) from error
        os.unlink(temporary, dir_fd=parent_fd)
        published = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        if ((published.st_dev, published.st_ino) != published_identity
                or queue_digest_at(parent_fd, name) != hashlib.sha256(updated).digest()):
            raise ValueError("queue destination swapped after automatic wait publication")
        os.unlink(displaced, dir_fd=parent_fd)
        moved_original = False
        os.fsync(parent_fd)
    finally:
        if descriptor >= 0:
            os.close(descriptor)
        try:
            os.unlink(temporary, dir_fd=parent_fd)
        except FileNotFoundError:
            pass
        if moved_original:
            try:
                os.link(
                    displaced, name,
                    src_dir_fd=parent_fd, dst_dir_fd=parent_fd,
                    follow_symlinks=False,
                )
            except FileExistsError:
                pass
            os.unlink(displaced, dir_fd=parent_fd)


def transition_ready(queue_path: Path, queue_parent_fd: int, ordinal: str, parser, original: bytes) -> None:
    text = original.decode("utf-8")
    lines = text.splitlines(keepends=True)
    prefix = f"{ordinal}. "
    active_lines = parser.active_now_lines(text)
    active_numbers = {number for number, _line in active_lines}
    matches = [index for index, line in enumerate(lines)
               if index + 1 in active_numbers and line.startswith(prefix)]
    if len(matches) != 1:
        raise ValueError("automatic wait item is not a canonical single-line item")
    index = matches[0]
    end = index + 1
    while end < len(lines) and end + 1 in active_numbers and lines[end][:1].isspace():
        end += 1
    body = " ".join(part.strip() for part in lines[index:end])
    line = body.replace("[WAITING-AUTOMATIC]", "[READY]", 1)
    for key in ("probe", "retry", "timeout", "deadline"):
        line = parser.re.sub(rf"\s*\[{key}=[^\]]+\]", "", line)
    lines[index:end] = [line.rstrip() + "\n"]
    updated = "".join(lines).encode("utf-8")
    parser.validate_queue(updated.decode("utf-8"), queue_path)
    current_info = queue_path.lstat()
    original_info = transition_ready.original_info
    if (
        (current_info.st_dev, current_info.st_ino) != original_info
        or queue_digest(queue_path) != hashlib.sha256(original).digest()
    ):
        raise ValueError("queue changed during automatic wait transition")
    info = os.stat(queue_path.name, dir_fd=queue_parent_fd, follow_symlinks=False)
    if not stat.S_ISREG(info.st_mode):
        raise ValueError("unsafe queue file")
    if pause_blocks(queue_path.parent / "PAUSE"):
        raise ValueError("PAUSE appeared during automatic wait transition")
    publish_queue_cas(queue_path, queue_parent_fd, original_info, original, updated, "wait")


def materialize_timeout_deadlines(queue_path: Path, queue_parent_fd: int, parser) -> None:
    text, info, _digest = parser.read_queue(queue_path)
    items = parser.validate_queue(text, queue_path)
    pending = [item for item in items if item[1] == "WAITING-AUTOMATIC" and "timeout" in item[4] and "deadline" not in item[4]]
    if not pending:
        return
    updated = text
    now = time.time()
    for _ordinal, _status, _item_id, _gate, metadata in pending:
        timeout_tag = f"[timeout={metadata['timeout']}]"
        deadline = dt.datetime.fromtimestamp(
            now + parser.duration_seconds(metadata["timeout"]), dt.timezone.utc
        ).strftime("%Y-%m-%dT%H:%M:%SZ")
        updated = updated.replace(timeout_tag, f"{timeout_tag} [deadline={deadline}]", 1)
    parser.validate_queue(updated, queue_path)
    payload = updated.encode("utf-8")
    original = text.encode("utf-8")
    if pause_blocks(queue_path.parent / "PAUSE"):
        raise ValueError("PAUSE appeared during deadline publication")
    current = queue_path.lstat()
    if ((current.st_dev, current.st_ino) != (info.st_dev, info.st_ino)
            or queue_digest(queue_path) != hashlib.sha256(original).digest()):
        raise ValueError("queue changed before deadline publication")
    publish_queue_cas(
        queue_path, queue_parent_fd, (info.st_dev, info.st_ino), original, payload,
        "deadline",
    )


def wait_items(queue_path: Path, parser):
    text, _info, _digest = parser.read_queue(queue_path)
    return [item for item in parser.validate_queue(text, queue_path) if item[1] == "WAITING-AUTOMATIC"]


def boot_id() -> str:
    try:
        return Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    except OSError:
        return "unknown"


def pause_blocks(path: Path) -> bool:
    if not path.exists():
        return False
    allowed = os.environ.get("NUDGE_WAIT_ALLOW_PAUSE_TOKEN")
    if allowed:
        try:
            return path.read_text(encoding="utf-8").strip() != allowed
        except OSError:
            return True
    return True


def trusted_probe_fd(queue_path: Path, probe: str, expected_sha256: str) -> tuple[int, tuple[int, int]]:
    root = queue_path.parent.parent
    root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        root_info = os.fstat(root_fd)
        if root_info.st_uid != os.geteuid() or root_info.st_mode & 0o022:
            raise ValueError("unsafe project root owner or mode")
        current = root_fd
        opened: list[int] = []
        parts = probe.split("/")
        for component in parts[:-1]:
            next_fd = os.open(
                component,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                dir_fd=current,
            )
            info = os.fstat(next_fd)
            if info.st_uid != os.geteuid() or info.st_mode & 0o022:
                os.close(next_fd)
                raise ValueError("unsafe probe directory owner or mode")
            opened.append(next_fd)
            current = next_fd
        fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW, dir_fd=current)
        info = os.fstat(fd)
        if (
            not stat.S_ISREG(info.st_mode)
            or info.st_uid != os.geteuid()
            or not info.st_mode & stat.S_IXUSR
            or info.st_mode & 0o022
            or info.st_size > MAX_PROBE_SIZE
        ):
            raise ValueError("unsafe probe owner, mode, or type")
        digest = hashlib.sha256()
        while chunk := os.read(fd, 1024 * 1024):
            digest.update(chunk)
        os.lseek(fd, 0, os.SEEK_SET)
        if digest.hexdigest() != expected_sha256:
            os.close(fd)
            raise ValueError("probe differs from authorized digest")
        return fd, (info.st_dev, info.st_ino)
    finally:
        for opened_fd in reversed(locals().get("opened", [])):
            os.close(opened_fd)
        os.close(root_fd)


def cleanup_process_group(process: subprocess.Popen[bytes]) -> None:
    for signum in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, signum)
        except ProcessLookupError:
            pass
        if signum == signal.SIGTERM:
            time.sleep(0.05)


def execute_probe(queue_path: Path, probe: str, expected_sha256: str, timeout: float) -> int:
    root = queue_path.parent.parent
    probe_path = root / probe
    fd, identity = trusted_probe_fd(queue_path, probe, expected_sha256)
    try:
        if pause_blocks(queue_path.parent / "PAUSE"):
            raise ValueError("PAUSE present before automatic probe")
        if not Path(BWRAP).is_file() or not os.access(BWRAP, os.X_OK):
            raise ValueError("automatic probe PID/network containment unavailable")
        process = subprocess.Popen(
            [BWRAP, "--unshare-net", "--unshare-pid", "--die-with-parent",
             "--new-session", "--bind", "/", "/", "--dev-bind", "/dev", "/dev",
             "--proc", "/proc",
             "--chdir", str(root), "--", f"/proc/self/fd/{fd}"],
            cwd=root,
            pass_fds=(fd,),
            start_new_session=True,
        )
        try:
            try:
                return_code = process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                return_code = 124
            cleanup_process_group(process)
            try:
                process.wait(timeout=0.5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            after = probe_path.lstat()
        except OSError as error:
            raise ValueError(f"probe changed during execution: {error}") from error
        if probe_path.is_symlink() or (after.st_dev, after.st_ino) != identity:
            raise ValueError("probe identity/inode changed during execution")
        return return_code
    finally:
        os.close(fd)


def item_configuration(parser, queue_path: Path, ordinal: str) -> str:
    text, _info, _digest = parser.read_queue(queue_path)
    bodies = {number: body for number, _line, body in parser.parse_items(parser.active_now_lines(text))}
    return hashlib.sha256(bodies[ordinal].encode("utf-8")).hexdigest()


def validate_state_seal(state: dict[str, object]) -> None:
    expected = state.get("state_sha256")
    unsealed = dict(state)
    unsealed.pop("state_sha256", None)
    actual = hashlib.sha256(
        json.dumps(unsealed, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    if expected != actual:
        raise ValueError("automatic wait state digest mismatch")


def finite_number(value: object, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"automatic wait {name} must be a finite number")
    converted = float(value)
    if not math.isfinite(converted):
        raise ValueError(f"automatic wait {name} must be finite")
    return converted


def validate_wait_state(
    state: dict[str, object], ordinal: str, item_id: str, gate: str,
    metadata: dict[str, str], config_sha256: str, retry: float,
    current_boot: str, probe_sha256: str,
) -> None:
    validate_state_seal(state)
    if state.get("schema") != "nudge-wait-v1" or state.get("version") != 1 or isinstance(state.get("version"), bool):
        raise ValueError("automatic wait state schema/version mismatch")
    if state.get("ordinal") != int(ordinal) or isinstance(state.get("ordinal"), bool):
        raise ValueError("automatic wait ordinal mismatch")
    expected = (item_id, gate, metadata["probe"], metadata.get("probe_path", metadata["probe"]))
    actual = (state.get("id"), state.get("gate"), state.get("probe"), state.get("probe_path"))
    if actual != expected or state.get("state") != "waiting" or state.get("config_sha256") != config_sha256:
        raise ValueError("automatic wait state identity mismatch")
    if state.get("probe_sha256") != probe_sha256:
        raise ValueError("automatic wait probe digest mismatch")
    attempts = state.get("attempts")
    if isinstance(attempts, bool) or not isinstance(attempts, int) or not 0 <= attempts <= 1_000_000:
        raise ValueError("automatic wait attempts must be a bounded integer")
    started = finite_number(state.get("started_at"), "started_at")
    deadline = finite_number(state.get("deadline_at"), "deadline_at")
    next_attempt = finite_number(state.get("next_attempt_at"), "next_attempt_at")
    started_mono = finite_number(state.get("started_monotonic"), "started_monotonic")
    deadline_mono = finite_number(state.get("deadline_monotonic"), "deadline_monotonic")
    if started < 0 or deadline <= started or next_attempt < 0 or started_mono < 0 or deadline_mono <= started_mono:
        raise ValueError("automatic wait numeric state is negative or reversed")
    configured = []
    if "timeout" in metadata:
        configured.append(started + parser_duration(metadata["timeout"]))
    if "deadline" in metadata:
        configured.append(deadline_epoch_value(metadata["deadline"]))
    expected_deadline = min(configured)
    if abs(deadline - expected_deadline) > 1.0 or abs(deadline_mono - (started_mono + deadline - started)) > 1.0:
        raise ValueError("automatic wait persisted deadline exceeds queue configuration")
    maximum_attempts = math.ceil(max(0.0, deadline - started) / retry) + 2
    if attempts > maximum_attempts or next_attempt > deadline + retry:
        raise ValueError("automatic wait cadence or attempts exceed queue configuration")
    if state.get("boot_id") != current_boot:
        raise ValueError("automatic wait crossed boot without monotonic continuity")


# Set by run/verify before strict cache validation to avoid passing parser modules
# through every scalar helper.
parser_duration = lambda value: 0.0
deadline_epoch_value = lambda value: 0.0


def run(queue_path: Path, queue_parent_fd: int, state_fd: int, parser, force: bool = False) -> int:
    global parser_duration, deadline_epoch_value
    parser_duration = parser.duration_seconds
    deadline_epoch_value = parser.deadline_epoch
    materialize_timeout_deadlines(queue_path, queue_parent_fd, parser)
    original_text, queue_info, _queue_sha = parser.read_queue(queue_path)
    original = original_text.encode("utf-8")
    if queue_path.is_symlink() or not stat.S_ISREG(queue_info.st_mode):
        raise ValueError("unsafe queue file")
    transition_ready.original_info = (queue_info.st_dev, queue_info.st_ino)
    items = wait_items(queue_path, parser)
    now = time.time()
    monotonic_now = time.monotonic()
    current_boot = boot_id()
    promoted = False
    for ordinal, _status, item_id, gate, metadata in items:
        state_name = f"{item_id}.json"
        config_sha256 = item_configuration(parser, queue_path, ordinal)
        probe_path = metadata.get("probe_path", metadata["probe"])
        probe_file = queue_path.parent.parent / probe_path
        probe_sha256 = metadata.get("probe_sha256")
        if probe_sha256 is None:
            raise ValueError("automatic probe lacks committed digest authorization")
        retry = parser.duration_seconds(metadata["retry"])
        bounds = []
        if "timeout" in metadata:
            bounds.append(now + parser.duration_seconds(metadata["timeout"]))
        if "deadline" in metadata:
            bounds.append(parser.deadline_epoch(metadata["deadline"]))
        configured_deadline = min(bounds)
        try:
            os.stat(state_name, dir_fd=state_fd, follow_symlinks=False)
            state_exists = True
        except FileNotFoundError:
            state_exists = False
        if state_exists:
            safe_state_file(state_fd, state_name)
            state = bounded_json(state_fd, state_name)
            validate_wait_state(state, ordinal, item_id, gate, metadata, config_sha256, retry, current_boot, probe_sha256)
        else:
            state = {
                "schema": "nudge-wait-v1",
                "version": 1,
                "ordinal": int(ordinal),
                "id": item_id,
                "gate": gate,
                "probe": metadata["probe"],
                "probe_path": probe_path,
                "probe_sha256": probe_sha256,
                "config_sha256": config_sha256,
                "started_at": now,
                "deadline_at": configured_deadline,
                "next_attempt_at": now,
                "attempts": 0,
                "state": "waiting",
                "boot_id": current_boot,
                "started_monotonic": monotonic_now,
                "deadline_monotonic": monotonic_now + (configured_deadline - now),
            }
            write_state(state_fd, state_name, state)
        if now + 1 < float(state["started_at"]):
            raise ValueError("wall clock rollback detected against automatic wait state")
        if state.get("boot_id") not in {None, current_boot}:
            raise ValueError("automatic wait crossed boot without monotonic continuity")
        monotonic_expired = (
            state.get("deadline_monotonic") is not None
            and monotonic_now >= float(state["deadline_monotonic"])
        )
        if now >= float(state["deadline_at"]) or monotonic_expired:
            kind = "deadline-expired" if "deadline" in metadata else "wait-timeout"
            print(f"INVALID-DEADLOCKED automatic wait timeout kind={kind} id={item_id}")
            return 2
        if not force and now < float(state["next_attempt_at"]):
            continue
        remaining = max(0.1, float(state["deadline_at"]) - now)
        timeout = min(60.0, max(0.1, retry), remaining)
        if pause_blocks(queue_path.parent / "PAUSE"):
            raise ValueError("PAUSE present before automatic probe")
        current_queue = queue_path.lstat()
        if (current_queue.st_dev, current_queue.st_ino) != transition_ready.original_info:
            raise ValueError("queue identity changed before automatic probe")
        queue_before_probe = queue_digest(queue_path)
        return_code = execute_probe(queue_path, probe_path, probe_sha256, timeout)
        if queue_digest(queue_path) != queue_before_probe:
            raise ValueError("queue changed during automatic probe execution")
        finished = time.time()
        state["attempts"] = int(state["attempts"]) + 1
        state["last_attempt_at"] = finished
        state["last_rc"] = return_code
        state["next_attempt_at"] = finished + retry
        write_state(state_fd, state_name, state)
        if return_code == 0:
            if finished >= float(state["deadline_at"]):
                kind = "deadline-expired" if "deadline" in metadata else "wait-timeout"
                print(f"INVALID-DEADLOCKED automatic wait timeout kind={kind} id={item_id}")
                return 2
            transition_ready(queue_path, queue_parent_fd, ordinal, parser, original)
            os.unlink(state_name, dir_fd=state_fd)
            promoted = True
            break
    print("RUNNABLE" if promoted else "AUTOMATIC-WAIT")
    return 0


def verify(queue_path: Path, _queue_parent_fd: int, state_fd: int, parser) -> int:
    global parser_duration, deadline_epoch_value
    parser_duration = parser.duration_seconds
    deadline_epoch_value = parser.deadline_epoch
    items = wait_items(queue_path, parser)
    if not items:
        return 1
    current_boot = boot_id()
    for ordinal, _status, item_id, gate, metadata in items:
        name = f"{item_id}.json"
        try:
            safe_state_file(state_fd, name)
            value = bounded_json(state_fd, name)
            retry = parser.duration_seconds(metadata["retry"])
            config = item_configuration(parser, queue_path, ordinal)
            probe_path = metadata.get("probe_path", metadata["probe"])
            probe_sha = metadata.get("probe_sha256")
            if probe_sha is None:
                raise ValueError("automatic probe lacks committed digest authorization")
            validate_wait_state(value, ordinal, item_id, gate, metadata, config, retry, current_boot, probe_sha)
            if time.time() >= float(value["deadline_at"]) or time.monotonic() >= float(value["deadline_monotonic"]):
                return 1
        except (OSError, ValueError, json.JSONDecodeError):
            return 1
    # Verification is evidence inspection only: never execute or promote.
    return 0


def main(arguments: list[str]) -> int:
    if len(arguments) != 4 or arguments[1] not in {"run", "verify"}:
        return 64
    parser = load_parser(Path(__file__).resolve().parent)
    try:
        queue_path = Path(arguments[2])
        state_dir = Path(arguments[3])
        if queue_path.parent.absolute() != state_dir.parent.absolute():
            raise ValueError("automatic wait state must be inside the queue state directory")
        queue_parent_fd = open_directory(queue_path.parent)
        state_fd = open_state_directory(queue_parent_fd, state_dir.name)
        lock_descriptor = os.open(
            ".executor.lock",
            os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW,
            0o600,
            dir_fd=state_fd,
        )
        try:
            lock_info = os.fstat(lock_descriptor)
            if (
                not stat.S_ISREG(lock_info.st_mode)
                or lock_info.st_uid != os.geteuid()
                or stat.S_IMODE(lock_info.st_mode) != 0o600
            ):
                raise ValueError("unsafe automatic-wait executor lock")
            fcntl.flock(lock_descriptor, fcntl.LOCK_EX)
            queue_fd = os.open(
                ".queue.lock", os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600,
                dir_fd=queue_parent_fd,
            )
            try:
                lock_info = os.fstat(queue_fd)
                if not stat.S_ISREG(lock_info.st_mode) or lock_info.st_uid != os.geteuid() or stat.S_IMODE(lock_info.st_mode) != 0o600:
                    raise ValueError("unsafe queue transition lock")
                fcntl.flock(queue_fd, fcntl.LOCK_EX)
                return globals()[arguments[1]](queue_path, queue_parent_fd, state_fd, parser)
            finally:
                os.close(queue_fd)
        finally:
            os.close(lock_descriptor)
            os.close(state_fd)
            os.close(queue_parent_fd)
    except (OSError, UnicodeError, ValueError, KeyError, json.JSONDecodeError, parser.QueueError) as error:
        print(f"INVALID-DEADLOCKED automatic wait: {error}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
