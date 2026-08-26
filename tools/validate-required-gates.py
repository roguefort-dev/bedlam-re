#!/usr/bin/python3
"""Run the HEAD-bound required-gates contract offline and fail closed."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import pwd
import re
import signal
import stat
import subprocess
import tempfile
import time
import tomllib
from pathlib import Path


MAX_TIMEOUT = 1800
MAX_FILE_SIZE = 16 * 1024 * 1024
GIT = "/usr/bin/git"
BWRAP = "/usr/bin/bwrap"
EXECUTABLES = {
    "/bin/bash",
    "/usr/bin/bash",
    "/usr/bin/cargo",
    "/usr/bin/cmp",
    "/usr/bin/grep",
    "/usr/bin/python3",
    "/usr/bin/test",
    "/usr/bin/true",
}
PROXY_NAMES = {
    "ALL_PROXY", "BASH_ENV", "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_HOME",
    "GIT_CONFIG_COUNT", "GIT_CONFIG_GLOBAL", "GIT_CONFIG_SYSTEM",
    "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "PATH", "PYTHONHOME",
    "PYTHONPATH", "RUSTC_WRAPPER", "http_proxy", "https_proxy", "no_proxy",
}


class ValidationError(Exception):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git(root: Path, *arguments: str) -> str:
    result = subprocess.run(
        [GIT, "-C", str(root), *arguments],
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
        env={"LC_ALL": "C", "GIT_TERMINAL_PROMPT": "0"},
    )
    return result.stdout.strip()


def relative_path(root: Path, value: str) -> Path:
    path = Path(value)
    if path.is_absolute() or not value or ".." in path.parts:
        raise ValidationError(f"unsafe repository path: {value!r}")
    resolved = root.joinpath(path)
    if resolved.is_symlink():
        raise ValidationError(f"symlink is forbidden in required path: {value}")
    return resolved


def head_bytes(root: Path, relative: str) -> bytes:
    try:
        return subprocess.run(
            [GIT, "-C", str(root), "show", f"HEAD:{relative}"],
            check=True,
            capture_output=True,
            timeout=30,
            env={"LC_ALL": "C", "GIT_TERMINAL_PROMPT": "0"},
        ).stdout
    except subprocess.CalledProcessError as error:
        raise ValidationError(f"required path is not tracked at HEAD: {relative}") from error


def tracked_at_head(root: Path, relative: str) -> bytes:
    expected = head_bytes(root, relative)
    path = relative_path(root, relative)
    try:
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_FILE_SIZE:
            raise ValidationError(f"required tracked path is unsafe or oversized: {relative}")
        actual = path.read_bytes()
    except OSError as error:
        raise ValidationError(f"required tracked path is missing: {relative}") from error
    if actual != expected:
        raise ValidationError(f"required tracked path differs from HEAD: {relative}")
    return actual


def tracked_tree_fingerprint(root: Path) -> str:
    names = git(root, "ls-tree", "-r", "--name-only", "HEAD").splitlines()
    digest = hashlib.sha256()
    for name in names:
        raw = tracked_at_head(root, name)
        digest.update(name.encode() + b"\0" + hashlib.sha256(raw).digest())
    return digest.hexdigest()


def check_manifest(root: Path) -> str:
    raw = tracked_at_head(root, "MANIFEST.sha256")
    digest = hashlib.sha256(raw)
    for line_number, line in enumerate(raw.decode("utf-8").splitlines(), 1):
        match = re.fullmatch(r"([0-9a-f]{64})  (\S(?:.*\S)?)", line)
        if not match:
            raise ValidationError(f"MANIFEST.sha256 line {line_number} is malformed")
        expected, relative = match.groups()
        path = relative_path(root, relative)
        try:
            info = path.lstat()
            if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_FILE_SIZE:
                raise ValidationError(f"MANIFEST.sha256 path is unsafe: {relative}")
            actual = sha256(path.read_bytes())
        except OSError as error:
            raise ValidationError(f"MANIFEST.sha256 path is missing: {relative}") from error
        if actual != expected:
            raise ValidationError(f"MANIFEST.sha256 mismatch: {relative}")
        digest.update(relative.encode() + b"\0" + bytes.fromhex(actual))
    return digest.hexdigest()


def atomic_json(path: Path, value: dict[str, object]) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    if path.is_symlink():
        raise ValidationError(f"unsafe report target: {path}")
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        os.fchmod(descriptor, 0o600)
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, sort_keys=True, separators=(",", ":"), allow_nan=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def command_arrays(gate: dict[str, object], root: Path) -> list[list[str]]:
    raw = gate.get("commands")
    if raw is None and "command" in gate:
        raw = [gate["command"]]
    if not isinstance(raw, list):
        return []
    commands: list[list[str]] = []
    for command in raw:
        if not isinstance(command, list) or not command or not all(isinstance(part, str) and part for part in command):
            raise ValidationError(f"gate {gate.get('id')} command must be a non-empty argv array")
        executable = command[0]
        if not Path(executable).is_absolute() or executable not in EXECUTABLES:
            raise ValidationError(f"gate {gate.get('id')} executable is not absolute and allowlisted: {executable}")
        if executable == "/usr/bin/cargo" and not {"--locked", "--offline"}.issubset(command):
            raise ValidationError(f"gate {gate.get('id')} cargo command requires --locked and --offline")
        for argument in command[1:]:
            candidate = root / argument
            if not argument.startswith("-") and candidate.exists() and candidate.is_file():
                script = tracked_at_head(root, argument)
                if executable in {"/bin/bash", "/usr/bin/bash"}:
                    text = script.decode("utf-8")
                    for line in text.splitlines():
                        if re.search(r"(^|[;&|()]\s*)(?:cargo|/usr/bin/cargo)\s", line) and not (
                            "--locked" in line and "--offline" in line
                        ):
                            raise ValidationError(f"gate {gate.get('id')} reachable script has unlocked or online cargo")
        commands.append(command)
    return commands


def clean_environment() -> dict[str, str]:
    # No PATH means every executable, including nested script tools, must be explicit.
    return {
        "CARGO_NET_OFFLINE": "true",
        "GIT_TERMINAL_PROMPT": "0",
        "HOME": "/tmp/opencode",
        "LANG": "C",
        "LC_ALL": "C",
        "TZ": "UTC",
    }


def cleanup_group(process: subprocess.Popen[bytes]) -> None:
    for signum in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, signum)
        except ProcessLookupError:
            pass
        if signum == signal.SIGTERM:
            time.sleep(0.05)
    try:
        process.wait(timeout=1)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def run_command(command: list[str], root: Path, timeout: int) -> int:
    if not Path(BWRAP).is_file() or not os.access(BWRAP, os.X_OK):
        raise ValidationError("required network/PID containment is unavailable")
    with tempfile.TemporaryDirectory(prefix="required-gate-build-", dir="/tmp/opencode") as scratch:
        scratch_path = Path(scratch)
        target = scratch_path / "target"
        target.mkdir()
        environment = clean_environment()
        if command[0] == "/usr/bin/cargo":
            account_home = Path(pwd.getpwuid(os.geteuid()).pw_dir)
            environment.update({
                "PATH": "/usr/bin:/bin",
                "RUSTUP_HOME": str(account_home / ".rustup"),
                "TMPDIR": str(root / "target"),
            })
        sandbox = [
            BWRAP,
            "--unshare-net", "--unshare-pid", "--die-with-parent", "--new-session",
            "--ro-bind", "/", "/",
            "--dev-bind", "/dev", "/dev", "--proc", "/proc",
        ]
        if (root / "target").is_dir():
            sandbox.extend(["--bind", str(target), str(root / "target")])
        sandbox.extend(["--chdir", str(root), "--clearenv"])
        for name, value in environment.items():
            sandbox.extend(["--setenv", name, value])
        sandbox.extend(["--", "/usr/bin/env", "-u", "PWD", *command])
        process = subprocess.Popen(
            sandbox,
            cwd=root,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            env=environment,
            start_new_session=True,
        )
        try:
            try:
                process.communicate(timeout=timeout)
                return_code = process.returncode
            except subprocess.TimeoutExpired:
                return_code = 124
            return return_code
        finally:
            cleanup_group(process)


def run_validation(root: Path, manifest_path: Path, selected_phase: str | None) -> tuple[dict[str, object], bool]:
    raw = tracked_at_head(root, manifest_path.relative_to(root).as_posix())
    value = tomllib.loads(raw.decode("utf-8"))
    if value.get("schema") != "required-gates-v1":
        raise ValidationError("required-gates manifest schema is not required-gates-v1")
    head = git(root, "rev-parse", "HEAD")
    head_tree = git(root, "rev-parse", "HEAD^{tree}")
    tree_fingerprint = tracked_tree_fingerprint(root)
    manifest_corpus = check_manifest(root)
    gates = value.get("gate", [])
    phases = value.get("phase", [])
    if not isinstance(gates, list) or not gates or not isinstance(phases, list):
        raise ValidationError("required-gates manifest requires gate and phase arrays")
    phase_by_id = {phase.get("id"): phase for phase in phases if isinstance(phase, dict)}
    if phases and set(phase_by_id) != {f"P{number}" for number in range(8)}:
        raise ValidationError("global manifest must enumerate exactly P0 through P7")
    selected_ids: set[str] | None = None
    if selected_phase is not None:
        phase = phase_by_id.get(selected_phase)
        if phase is None:
            raise ValidationError(f"unknown phase {selected_phase}")
        selected_ids = set(phase.get("required_gates", []))

    results: list[dict[str, object]] = []
    passed: dict[str, bool] = {}
    corpus_hashes: dict[str, str] = {}
    for gate in gates:
        if not isinstance(gate, dict) or not isinstance(gate.get("id"), str):
            raise ValidationError("gate entries require stable string ids")
        gate_id = gate["id"]
        if selected_ids is not None and gate_id not in selected_ids:
            continue
        timeout = gate.get("timeout_seconds")
        if isinstance(timeout, bool) or not isinstance(timeout, int) or not 1 <= timeout <= MAX_TIMEOUT:
            raise ValidationError(f"gate {gate_id} timeout must be 1..{MAX_TIMEOUT} seconds")
        paths = gate.get("tracked_paths", [])
        corpus = gate.get("corpus", [])
        if not isinstance(paths, list) or not isinstance(corpus, list):
            raise ValidationError(f"gate {gate_id} path policies must be arrays")
        for relative in [*paths, *corpus]:
            if not isinstance(relative, str):
                raise ValidationError(f"gate {gate_id} path policy must contain strings")
            raw_path = tracked_at_head(root, relative)
            if relative in corpus:
                corpus_hashes[relative] = sha256(raw_path)
        dependencies = gate.get("depends", [])
        if not isinstance(dependencies, list) or not all(isinstance(item, str) for item in dependencies):
            raise ValidationError(f"gate {gate_id} dependencies must be ids")
        commands = command_arrays(gate, root)
        if not commands and not dependencies:
            raise ValidationError(f"gate {gate_id} has neither commands nor dependencies")
        ok = all(passed.get(dependency, False) for dependency in dependencies)
        command_results: list[dict[str, object]] = []
        if ok or not dependencies:
            ok = True
            for command in commands:
                rc = run_command(command, root, timeout)
                command_results.append({"argv": command, "rc": rc})
                # Every command boundary revalidates the immutable basis.
                if git(root, "rev-parse", "HEAD") != head:
                    raise ValidationError("HEAD changed during required-gates validation")
                if tracked_tree_fingerprint(root) != tree_fingerprint:
                    raise ValidationError("tracked tree changed during required-gates validation")
                if check_manifest(root) != manifest_corpus:
                    raise ValidationError("MANIFEST.sha256 corpus changed during required-gates validation")
                for relative, expected in corpus_hashes.items():
                    if sha256(tracked_at_head(root, relative)) != expected:
                        raise ValidationError(f"tracked corpus changed during validation: {relative}")
                ok = ok and rc == 0
        passed[gate_id] = ok
        results.append({"commands": command_results, "id": gate_id, "passed": ok})

    if selected_ids is not None:
        complete = selected_ids == set(passed) and all(passed.values())
    elif phases:
        complete = all(
            phase.get("status") == "green"
            and all(passed.get(gate_id, False) for gate_id in phase.get("required_gates", []))
            for phase in phases
        )
    else:
        complete = all(passed.values())
    report: dict[str, object] = {
        "bounded": True,
        "containment": "bwrap-unshare-net-pid-ro",
        "corpus_sha256": sha256(json.dumps(corpus_hashes, sort_keys=True).encode()),
        "gates": results,
        "head": head,
        "head_tree": head_tree,
        "manifest_sha256": sha256(raw),
        "offline": True,
        "plan_complete": complete if selected_phase is None else False,
        "schema": "required-gates-report-v1",
        "selected_phase": selected_phase,
        "status": "passed" if complete else "failed",
        "tracked_tree_sha256": tree_fingerprint,
        "validator_sha256": sha256(Path(__file__).read_bytes()),
    }
    return report, complete


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--completion-output", type=Path)
    parser.add_argument("--phase", choices=[f"P{number}" for number in range(8)])
    parser.add_argument("--phase-output", type=Path)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    manifest = root / "docs/required-gates.toml"
    try:
        initial_head = git(root, "rev-parse", "HEAD")
    except Exception:
        initial_head = "unknown"
    try:
        report, complete = run_validation(root, manifest, arguments.phase)
    except (OSError, UnicodeError, ValueError, subprocess.SubprocessError, ValidationError, tomllib.TOMLDecodeError) as error:
        report = {
            "bounded": True,
            "error": str(error),
            "head": initial_head,
            "offline": True,
            "plan_complete": False,
            "schema": "required-gates-report-v1",
            "status": "failed",
        }
        complete = False
    atomic_json(arguments.report, report)
    if complete and arguments.phase is None and arguments.completion_output:
        atomic_json(arguments.completion_output, {
            "head": report["head"],
            "head_tree": report["head_tree"],
            "offline_validation": {"bounded": True, "status": "passed", "validated_at_head": report["head"]},
            "producer": "controller",
            "required_gates_sha256": report["manifest_sha256"],
            "schema": "plan-complete-v1",
            "validator_sha256": report["validator_sha256"],
        })
    if complete and arguments.phase is not None and arguments.phase_output:
        atomic_json(arguments.phase_output, {
            "head": report["head"], "phase": arguments.phase,
            "producer": "required-gates-validator",
            "required_gates_sha256": report["manifest_sha256"],
            "schema": "phase-complete-v1",
        })
    return 0 if complete else 1


if __name__ == "__main__":
    raise SystemExit(main())
