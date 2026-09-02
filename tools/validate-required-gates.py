#!/usr/bin/python3
"""Run the HEAD-bound required-gates contract offline and fail closed."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pwd
import re
import shutil
import signal
import stat
import subprocess
import tempfile
import time
import tomllib
from pathlib import Path


MAX_TIMEOUT = 1800
MAX_FILE_SIZE = 16 * 1024 * 1024
# required-gates-v2 (D238): every gate carries an evidence classification.
# Only "product" evidence may certify product (phase-green / plan) completion;
# the six non-product classes are preserved supporting evidence that can never
# flip a phase to green. The v1 manifest — 37 gates, none a natural
# product-path journey — certified P0-P7 green and a complete plan without a
# single product gate, which is why v1 is revoked outright below.
MANIFEST_SCHEMA = "required-gates-v2"
EVIDENCE_PRODUCT = "product"
EVIDENCE_CLASSES = (
    "product",          # natural product-path journey evidence
    "supporting",       # real code/evidence supporting the port, not a journey
    "static",           # static/differential oracle pins
    "paperwork",        # contract/registry/definition grading
    "synthetic",        # synthetic inputs or synthetic journeys
    "corpus-required",  # replay/oracle evidence that requires the corpus
    "infrastructure",   # environment/validator machinery evidence
)
PHASE_STATUSES = ("pending", "engineering-green", "green")
# Per-gate verdict cache (D239): an OPT-IN accelerator, never an authority.
# A cached green is only ever reused when the gate's full basis fingerprint
# -- HEAD commit, the whole tracked tree, the required-gates manifest, the
# MANIFEST.sha256 corpus digest, the validator bytes, and every per-gate
# input (commands, tracked paths, command scripts, bounds) -- is exactly
# the basis this run computes. Anything else (missing, malformed, foreign,
# corrupt entry) fails closed to a re-run.
GATE_CACHE_SCHEMA = "required-gate-cache-v1"
MAX_CACHE_ENTRY_BYTES = 4096
# MANIFEST.sha256 corpus files (the read-only game-data corpus) are bound
# into the sealed completion basis by the controller at 128 MiB -- the
# exact cap complete_from_head enforces for external corpus paths. The
# smaller tracked-content cap above must not leak into that contract:
# the corpus legitimately carries 17-40 MB originals (BEDLAM0*.WAV,
# GAMEGFX/TITLE.SMK).
MAX_MANIFEST_FILE_SIZE = 128 * 1024 * 1024
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
GATE_KEYS = {
    "id", "timeout_seconds", "tracked_paths", "corpus", "depends",
    "commands", "command", "writable", "evidence",
}
PHASE_KEYS = {"id", "status", "required_gates"}
SCRATCH_BASE = Path("/tmp/opencode")


class ValidationError(Exception):
    pass


def scratch_base() -> Path:
    """Host-side scratch root for gate containment.

    The shared /tmp/opencode staging area is the controller contract; a
    validator nested inside a gate command runs after --tmpfs /tmp, so
    that path does not exist there and its scratch must live under the
    per-command HOME (a writable repo-anchored bind) instead -- never by
    creating anything under the gate's own /tmp, which the containment
    contract requires to start empty.
    """
    if SCRATCH_BASE.is_dir():
        return SCRATCH_BASE
    home = os.environ.get("HOME") or pwd.getpwuid(os.geteuid()).pw_dir
    fallback = Path(home) / ".required-gate-scratch"
    fallback.mkdir(mode=0o700, parents=True, exist_ok=True)
    return fallback


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    # Streamed: MANIFEST corpus files may reach the 128 MiB binding cap
    # and this digest re-runs at every command boundary.
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


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


def tracked_in_head(root: Path, relative: str) -> bool:
    return subprocess.run(
        [GIT, "-C", str(root), "cat-file", "-e", f"HEAD:{relative}"],
        capture_output=True,
        timeout=30,
        env={"LC_ALL": "C", "GIT_TERMINAL_PROMPT": "0"},
    ).returncode == 0


def gitignored(root: Path, relative: str) -> bool:
    return subprocess.run(
        [GIT, "-C", str(root), "check-ignore", "-q", "--no-index", relative],
        cwd=root,
        capture_output=True,
        timeout=30,
        env={"LC_ALL": "C", "GIT_TERMINAL_PROMPT": "0"},
    ).returncode == 0


def head_paths_under(root: Path, relative: str) -> list[str]:
    listed = git(root, "ls-tree", "-r", "--name-only", "HEAD", "--", relative)
    return listed.splitlines() if listed else []


def ensure_target_dir(root: Path) -> None:
    """Guarantee the target mountpoint exists before the sandbox binds it.

    A detached read-only checkout (the controller's completion basis)
    pre-creates target/ because bwrap cannot make mountpoints on a
    read-only root; a live writable root gets it created here.
    """
    target = root / "target"
    if target.is_dir():
        return
    if target.exists():
        raise ValidationError("target path is not a directory")
    try:
        target.mkdir(mode=0o755)
    except OSError as error:
        raise ValidationError(
            "the invocation root has no target directory and is read-only"
        ) from error


def ensure_writable_dir(root: Path, relative: str) -> None:
    """Validate and create a gate-declared writable scratch directory.

    Writable binds are how a contained gate writes inside the read-only
    repository, so the declaration must be fail-closed: the path is
    repository-relative, never a symlink (at any depth), untracked at
    HEAD with no tracked content beneath it, and covered by .gitignore
    so the bind can never cover or fabricate tracked evidence. A
    read-only invocation root must have pre-created the directory
    (bwrap can only bind over mountpoints that exist).
    """
    path = relative_path(root, relative)
    if path.exists() and not path.is_dir():
        raise ValidationError(f"writable path is not a directory: {relative}")
    if not gitignored(root, relative):
        raise ValidationError(f"writable path must be gitignored: {relative}")
    if head_paths_under(root, relative):
        raise ValidationError(f"writable path must not contain tracked files: {relative}")
    walked = root
    for part in Path(relative).parts:
        walked = walked / part
        if walked.is_symlink():
            raise ValidationError(f"writable path traverses a symlink: {relative}")
    try:
        path.mkdir(mode=0o700, parents=True, exist_ok=True)
    except OSError:
        if not path.is_dir():
            raise ValidationError(
                f"writable path cannot be created and does not exist: {relative}"
            ) from None


def command_script_arguments(commands: list[list[str]], root: Path) -> list[str]:
    """Every command argument that names a tracked script the gate reaches.

    This mirrors the predicate command_arrays() already validates: an
    argument that is not a flag, exists under the root, and is a regular
    file (the validator has already fail-closed on such paths being
    tracked and clean at HEAD, so re-reading them here is safe).
    """
    scripts: list[str] = []
    for command in commands:
        for argument in command[1:]:
            if argument.startswith("-"):
                continue
            candidate = root / argument
            if candidate.exists() and candidate.is_file():
                scripts.append(argument)
    return scripts


def gate_cache_key(gate_id: str) -> str:
    return sha256(gate_id.encode("utf-8")) + ".json"


def gate_cache_basis(
    *,
    gate_id: str,
    evidence: str,
    timeout: int,
    head: str,
    tree_fingerprint: str,
    manifest_sha: str,
    corpus_digest: str,
    validator_sha: str,
    path_digests: dict[str, str],
    script_digests: dict[str, str],
    commands: list[list[str]],
    writable: list[str],
    depends: list[str],
) -> str:
    """The per-gate basis fingerprint a cached green is keyed on.

    Covers everything that can change the gate's verdict: the HEAD commit
    and the whole tracked tree (any tracked change re-runs, by design --
    the scheduler's pain was repeat full-battery runs at an UNCHANGED
    HEAD), the required-gates manifest bytes, the MANIFEST.sha256 corpus
    digest, the validator's own bytes, and the gate's own slice -- its
    commands, tracked paths, command scripts, declared bounds, and
    dependency edges.
    """
    digest = hashlib.sha256()

    def field(label: str, payload: bytes) -> None:
        digest.update(label.encode("utf-8") + b"\0" + payload + b"\0")

    field("schema", GATE_CACHE_SCHEMA.encode())
    field("head", head.encode())
    field("tracked-tree", tree_fingerprint.encode())
    field("manifest", manifest_sha.encode())
    field("manifest-corpus", corpus_digest.encode())
    field("validator", validator_sha.encode())
    field("gate-id", gate_id.encode())
    field("evidence", evidence.encode())
    field("timeout", str(timeout).encode())
    field("commands", json.dumps(commands, separators=(",", ":")).encode())
    field(
        "tracked-paths",
        json.dumps(
            [[relative, path_digests[relative]] for relative in sorted(path_digests)],
            separators=(",", ":"),
        ).encode(),
    )
    field(
        "command-scripts",
        json.dumps(
            [[relative, script_digests[relative]] for relative in sorted(script_digests)],
            separators=(",", ":"),
        ).encode(),
    )
    field("writable", json.dumps(sorted(writable), separators=(",", ":")).encode())
    field("depends", json.dumps(sorted(depends), separators=(",", ":")).encode())
    return digest.hexdigest()


def ensure_gate_cache_dir(root: Path, value: Path | None) -> Path | None:
    """Resolve and validate the optional per-gate verdict cache directory.

    The cache is runtime state, never evidence. A path inside the
    invocation root must be gitignored with no tracked content beneath it
    and must not be reached through a symlink (the same discipline as a
    gate-declared writable) so the cache can never become, cover, or
    fabricate tracked evidence; a path outside the root is host-side
    runtime state with the same trust as --report. Configuration errors
    fail closed and loudly; the cache is never silently disabled here.
    """
    if value is None:
        return None
    path = Path(value)
    resolved = path if path.is_absolute() else root / path
    try:
        relative = resolved.relative_to(root).as_posix()
    except ValueError:
        relative = None
    if relative is not None:
        if not relative:
            raise ValidationError("gate cache path must not be the invocation root")
        if not gitignored(root, relative):
            raise ValidationError(f"gate cache path must be gitignored: {relative}")
        if head_paths_under(root, relative):
            raise ValidationError(f"gate cache path must not contain tracked files: {relative}")
        walked = root
        for part in Path(relative).parts:
            walked = walked / part
            if walked.is_symlink():
                raise ValidationError(f"gate cache path traverses a symlink: {relative}")
    if resolved.exists():
        if resolved.is_symlink() or not resolved.is_dir():
            raise ValidationError(f"gate cache path is not a real directory: {value}")
    else:
        try:
            resolved.mkdir(mode=0o700, parents=True, exist_ok=True)
        except OSError:
            # A read-only root that still exposes an existing cache
            # directory (pre-sealed) may read from it; entries are only
            # ever written best-effort.
            if not resolved.is_dir():
                raise ValidationError(f"gate cache path cannot be created: {value}") from None
    return resolved


def cached_green_verdict(cache: Path, gate_id: str, basis: str) -> bool:
    """Strict, fail-closed cache lookup: anything but an exact match is a miss."""
    entry_path = cache / gate_cache_key(gate_id)
    try:
        info = entry_path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_CACHE_ENTRY_BYTES:
            return False
        raw = entry_path.read_bytes()
    except OSError:
        return False
    try:
        entry = json.loads(raw.decode("utf-8"))
    except (UnicodeError, ValueError):
        return False
    if not isinstance(entry, dict) or set(entry) != {"schema", "id", "basis_sha256"}:
        return False
    if entry["schema"] != GATE_CACHE_SCHEMA or entry["id"] != gate_id:
        return False
    return entry["basis_sha256"] == basis


def remember_green_verdict(cache: Path, gate_id: str, basis: str) -> None:
    """Persist a green verdict best-effort: the cache accelerates, never validates."""
    try:
        entry_path = cache / gate_cache_key(gate_id)
        try:
            # A poisoned or odd entry (symlink, fifo) is never written
            # THROUGH -- it is cleared and replaced by a fresh regular
            # file, so the next run can hit honestly.
            if not stat.S_ISREG(entry_path.lstat().st_mode):
                entry_path.unlink()
        except OSError:
            pass
        atomic_json(entry_path, {
            "basis_sha256": basis,
            "id": gate_id,
            "schema": GATE_CACHE_SCHEMA,
        })
    except (OSError, ValidationError):
        pass


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
            if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_MANIFEST_FILE_SIZE:
                raise ValidationError(f"MANIFEST.sha256 path is unsafe or oversized: {relative}")
            actual = sha256_file(path)
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


def command_environment(home: str) -> dict[str, str]:
    # No PATH means every executable, including nested script tools, must be
    # explicit. HOME is the per-command scratch home: a repo-anchored unique
    # path inside the target bind (never under /tmp, so nested validators
    # inside a gate command can neither see nor lose their fixtures to a
    # sandbox-private tmpfs).
    return {
        "CARGO_NET_OFFLINE": "true",
        "GIT_TERMINAL_PROMPT": "0",
        "HOME": home,
        "LANG": "C",
        "LC_ALL": "C",
        "TZ": "UTC",
    }


def cargo_environment(root: Path) -> dict[str, str]:
    """Resolve the account cargo/rustup homes for offline --locked builds.

    The /usr/bin/cargo on this host is a rustup proxy: under containment
    it refuses to run with an unwritable default home and offline crate
    resolution fails without the account registry cache. Pin CARGO_HOME
    (and RUSTUP_HOME when it exists) at the account's real, read-only
    homes so the proxy resolves the installed toolchain and crates
    without writing anything. Fail closed when the cache is missing:
    an offline gate that cannot resolve crates must not run at all.
    """
    account = pwd.getpwuid(os.geteuid()).pw_dir
    cargo_home = os.environ.get("CARGO_HOME") or os.path.join(account, ".cargo")
    if not Path(cargo_home).is_dir():
        raise ValidationError(
            "the account cargo cache is unavailable for offline gate commands"
        )
    rustup_home = os.environ.get("RUSTUP_HOME") or os.path.join(account, ".rustup")
    environment = {
        "PATH": "/usr/bin:/bin",
        "CARGO_HOME": cargo_home,
        "TMPDIR": str(root / "target"),
    }
    if Path(rustup_home).is_dir():
        environment["RUSTUP_HOME"] = rustup_home
    return environment


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


def run_command(
    command: list[str],
    root: Path,
    timeout: int,
    *,
    cargo_environment_vars: dict[str, str] | None = None,
    writable: tuple[str, ...] = (),
) -> int:
    if not Path(BWRAP).is_file() or not os.access(BWRAP, os.X_OK):
        raise ValidationError("required network/PID containment is unavailable")
    # Host-side scratch lives outside the invocation root (a detached
    # read-only checkout cannot host it); every bind below maps it in.
    scratch = Path(tempfile.mkdtemp(prefix="required-gate-", dir=scratch_base()))
    try:
        target_scratch = scratch / "target"
        target_scratch.mkdir(mode=0o700)
        # The per-command home rides INSIDE the target bind at a unique
        # repo-anchored path: HOME's parent is <root>/target/.gate-home,
        # exactly what the env-probe containment contract pins, and a
        # validator nested inside a gate command binds only its own
        # root's paths, so this never nests with the outer mount.
        home_root = target_scratch / ".gate-home"
        home_root.mkdir(mode=0o700)
        home_name = Path(tempfile.mkdtemp(dir=home_root)).name
        ensure_target_dir(root)
        home_mount = root / "target" / ".gate-home" / home_name
        environment = command_environment(str(home_mount))
        if command[0] == "/usr/bin/cargo":
            environment.update(cargo_environment_vars or cargo_environment(root))
        sandbox = [
            BWRAP,
            "--unshare-net", "--unshare-pid", "--die-with-parent", "--new-session",
            "--ro-bind", "/", "/",
            "--dev-bind", "/dev", "/dev", "--proc", "/proc",
            # A fresh private /tmp: gate commands never see host /tmp
            # state, the env-probe contract requires it empty, and no
            # gate scratch may be allocated there (see scratch_base()).
            "--tmpfs", "/tmp",
            # The controller may root the invocation itself under /tmp
            # (complete_from_head seals its detached basis in
            # /tmp/opencode); the tmpfs above would hide that basis and
            # leave only an empty auto-created directory, so the root is
            # re-exposed read-only at its own path. check-gates-env
            # tolerates exactly this chain and nothing else under /tmp.
            "--ro-bind", str(root), str(root),
            # Writable scratch through the read-only root: target/ is the
            # pre-existing mountpoint and carries the per-command home.
            "--bind", str(target_scratch), str(root / "target"),
        ]
        for index, relative in enumerate(writable):
            writable_scratch = scratch / f"w{index}"
            writable_scratch.mkdir(mode=0o700)
            sandbox.extend(["--bind", str(writable_scratch), str(root / relative)])
        sandbox.extend(["--chdir", str(root), "--clearenv"])
        for name, value in sorted(environment.items()):
            sandbox.extend(["--setenv", name, value])
        sandbox.extend(["--", "/usr/bin/env", "-u", "PWD", *command])
        process = subprocess.Popen(
            sandbox,
            cwd=root,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            env={"LC_ALL": "C"},
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
    finally:
        shutil.rmtree(scratch, ignore_errors=True)


def run_validation(
    root: Path,
    manifest_path: Path,
    selected_phase: str | None,
    gate_cache: Path | None = None,
) -> tuple[dict[str, object], bool]:
    raw = tracked_at_head(root, manifest_path.relative_to(root).as_posix())
    value = tomllib.loads(raw.decode("utf-8"))
    if value.get("schema") != MANIFEST_SCHEMA:
        raise ValidationError(
            "required-gates manifest schema is not required-gates-v2 (v1 is "
            "revoked: its non-product evidence certified product completion; "
            "re-classify every gate under required-gates-v2 evidence classes)"
        )
    head = git(root, "rev-parse", "HEAD")
    head_tree = git(root, "rev-parse", "HEAD^{tree}")
    tree_fingerprint = tracked_tree_fingerprint(root)
    manifest_corpus = check_manifest(root)
    manifest_sha = sha256(raw)
    validator_sha = sha256(Path(__file__).read_bytes())
    # The optional per-gate verdict cache: resolved (and fail-closed on a
    # violating path) only after the immutable basis is established.
    cache_dir = ensure_gate_cache_dir(root, gate_cache)
    gates = value.get("gate", [])
    phases = value.get("phase", [])
    if (
        not isinstance(gates, list) or not gates
        or not isinstance(phases, list) or not phases
    ):
        raise ValidationError(
            "required-gates manifest requires non-empty gate and phase "
            "arrays (a product plan must enumerate its phases)"
        )
    gate_evidence: dict[str, str] = {}
    for gate in gates:
        if not isinstance(gate, dict) or not isinstance(gate.get("id"), str):
            raise ValidationError("gate entries require stable string ids")
        gate_id = gate["id"]
        evidence = gate.get("evidence")
        if not isinstance(evidence, str) or evidence not in EVIDENCE_CLASSES:
            raise ValidationError(
                f"gate {gate_id} evidence classification must be one of "
                f"{list(EVIDENCE_CLASSES)}, found {evidence!r}"
            )
        if gate_id in gate_evidence:
            raise ValidationError(f"duplicate gate id: {gate_id}")
        gate_evidence[gate_id] = evidence
    product_gates = sorted(
        gate_id for gate_id, evidence in gate_evidence.items()
        if evidence == EVIDENCE_PRODUCT
    )
    seen_phase_ids: set[str] = set()
    for phase in phases:
        if not isinstance(phase, dict) or set(phase) - PHASE_KEYS:
            raise ValidationError("phase entries have unknown keys")
        phase_id = phase.get("id")
        if phase_id in seen_phase_ids:
            raise ValidationError(f"duplicate phase id: {phase_id}")
        seen_phase_ids.add(phase_id)
        status = phase.get("status")
        if status not in PHASE_STATUSES:
            raise ValidationError(
                f"phase {phase.get('id')} status must be one of "
                f"{list(PHASE_STATUSES)}, found {status!r}"
            )
        required = phase.get("required_gates", [])
        if status == "engineering-green" and not required:
            raise ValidationError(
                f"phase {phase.get('id')} is engineering-green but wires no "
                "required gate (use pending until engineering evidence exists)"
            )
        if status == "green":
            wired_product = [
                gate_id for gate_id in required
                if gate_evidence.get(gate_id) == EVIDENCE_PRODUCT
            ]
            if not wired_product:
                raise ValidationError(
                    f"phase {phase.get('id')} status is green but no "
                    "product gate is wired: non-product evidence (supporting, "
                    "static, paperwork, synthetic, corpus-required, "
                    "infrastructure) can never certify product completion"
                )
    phase_by_id = {phase.get("id"): phase for phase in phases if isinstance(phase, dict)}
    if set(phase_by_id) != {f"P{number}" for number in range(8)}:
        raise ValidationError("global manifest must enumerate exactly P0 through P7")
    selected_ids: set[str] | None = None
    if selected_phase is not None:
        phase = phase_by_id.get(selected_phase)
        if phase is None:
            raise ValidationError(f"unknown phase {selected_phase}")
        selected_ids = set(phase.get("required_gates", []))

    cargo_environment_vars: dict[str, str] | None = None
    results: list[dict[str, object]] = []
    passed: dict[str, bool] = {}
    corpus_hashes: dict[str, str] = {}
    for gate in gates:
        if not isinstance(gate, dict) or not isinstance(gate.get("id"), str):
            raise ValidationError("gate entries require stable string ids")
        unknown = set(gate) - GATE_KEYS
        if unknown:
            raise ValidationError(f"gate {gate.get('id')} has unknown keys: {sorted(unknown)}")
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
        path_digests: dict[str, str] = {}
        for relative in [*paths, *corpus]:
            if not isinstance(relative, str):
                raise ValidationError(f"gate {gate_id} path policy must contain strings")
            raw_path = tracked_at_head(root, relative)
            path_digests[relative] = sha256(raw_path)
            if relative in corpus:
                corpus_hashes[relative] = sha256(raw_path)
        writable = gate.get("writable", [])
        if not isinstance(writable, list) or not all(isinstance(item, str) for item in writable):
            raise ValidationError(f"gate {gate_id} writable policy must be an array of strings")
        for relative in writable:
            ensure_writable_dir(root, relative)
        dependencies = gate.get("depends", [])
        if not isinstance(dependencies, list) or not all(isinstance(item, str) for item in dependencies):
            raise ValidationError(f"gate {gate_id} dependencies must be ids")
        commands = command_arrays(gate, root)
        if not commands and not dependencies:
            raise ValidationError(f"gate {gate_id} has neither commands nor dependencies")
        if cargo_environment_vars is None and any(command[0] == "/usr/bin/cargo" for command in commands):
            cargo_environment_vars = cargo_environment(root)
        ok = all(passed.get(dependency, False) for dependency in dependencies)
        command_results: list[dict[str, object]] = []
        gate_basis: str | None = None
        if cache_dir is not None and commands and (ok or not dependencies):
            # Only a gate that would actually execute now computes its
            # basis: dependency-failed gates are red as a live consequence
            # and never consult the cache at all.
            script_digests = {
                relative: sha256(tracked_at_head(root, relative))
                for relative in sorted(set(command_script_arguments(commands, root)))
            }
            gate_basis = gate_cache_basis(
                commands=commands,
                corpus_digest=manifest_corpus,
                depends=dependencies,
                evidence=gate_evidence[gate_id],
                gate_id=gate_id,
                head=head,
                manifest_sha=manifest_sha,
                path_digests=path_digests,
                script_digests=script_digests,
                timeout=timeout,
                tree_fingerprint=tree_fingerprint,
                validator_sha=validator_sha,
                writable=writable,
            )
        if ok or not dependencies:
            ok = True
            if gate_basis is not None and cached_green_verdict(cache_dir, gate_id, gate_basis):
                # Cached green reused: this exact basis was proven green by
                # this exact validator at a prior run, so the green command
                # verdicts replay byte-identically (a remembered green
                # implies rc=0 for every command) and nothing re-executes --
                # no command boundaries run, so the per-boundary basis
                # revalidation has nothing to recheck.
                command_results = [{"argv": command, "rc": 0} for command in commands]
            else:
                for command in commands:
                    rc = run_command(
                        command,
                        root,
                        timeout,
                        cargo_environment_vars=cargo_environment_vars,
                        writable=tuple(writable),
                    )
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
                if ok and gate_basis is not None:
                    remember_green_verdict(cache_dir, gate_id, gate_basis)
        passed[gate_id] = ok
        results.append({
            "commands": command_results,
            "evidence": gate_evidence[gate_id],
            "id": gate_id,
            "passed": ok,
            "writable": writable,
        })

    if selected_ids is not None:
        complete = selected_ids == set(passed) and all(passed.values())
    else:
        # Product completion requires every phase product-green: a "green"
        # phase is structurally impossible without a wired product gate
        # (rejected above), so all-green plus all-gates-passed is the only
        # shape that can ever complete. engineering-green and pending
        # phases always leave the plan incomplete. There is deliberately
        # no phase-less completion branch: the phase array is required
        # non-empty above (a zero-phase manifest with one product gate
        # must never complete the plan — the review-reproduced bypass).
        complete = all(
            phase.get("status") == "green"
            and all(passed.get(gate_id, False) for gate_id in phase.get("required_gates", []))
            for phase in phases
        )
    why_incomplete: list[str] = []
    if selected_phase is None:
        for phase in phases:
            if phase.get("status") != "green":
                why_incomplete.append(
                    f"phase {phase.get('id')} status is {phase.get('status')!r},"
                    " not product-green"
                )
        if not product_gates:
            why_incomplete.append(
                "manifest wires no product gate; non-product evidence can "
                "never certify product completion"
            )
        failing_gates = sorted(gate_id for gate_id, ok in passed.items() if not ok)
        if failing_gates:
            why_incomplete.append("failing gates: " + ", ".join(failing_gates))
    report: dict[str, object] = {
        "bounded": True,
        "containment": "bwrap-unshare-net-pid-ro",
        "corpus_sha256": sha256(json.dumps(corpus_hashes, sort_keys=True).encode()),
        "evidence": gate_evidence,
        "gates": results,
        "head": head,
        "head_tree": head_tree,
        "manifest_sha256": manifest_sha,
        "offline": True,
        "phase_product_coverage": {
            phase.get("id"): sum(
                1 for gate_id in phase.get("required_gates", [])
                if gate_evidence.get(gate_id) == EVIDENCE_PRODUCT
            )
            for phase in phases
            if isinstance(phase, dict)
        },
        "plan_complete": complete if selected_phase is None else False,
        "product_gates": product_gates,
        "schema": "required-gates-report-v2",
        "selected_phase": selected_phase,
        "status": "passed" if complete else "failed",
        "tracked_tree_sha256": tree_fingerprint,
        "validator_sha256": validator_sha,
        "why_incomplete": why_incomplete,
    }
    if selected_phase is not None:
        selected = phase_by_id.get(selected_phase, {})
        selected_required = selected.get("required_gates", []) if isinstance(selected, dict) else []
        selected_engineering = [
            gate_id for gate_id in selected_required
            if gate_evidence.get(gate_id) != EVIDENCE_PRODUCT
        ]
        report["phase_verdict"] = {
            # Fail-closed: a phase wiring no engineering gates has no
            # engineering coverage to claim, so it must never read as
            # vacuously complete.
            "engineering_complete": bool(selected_engineering)
            and all(passed.get(gate_id, False) for gate_id in selected_engineering),
            "phase": selected_phase,
            "phase_status": selected.get("status") if isinstance(selected, dict) else None,
            "product_complete": (
                isinstance(selected, dict)
                and selected.get("status") == "green"
                and all(passed.get(gate_id, False) for gate_id in selected_required)
                and any(
                    gate_evidence.get(gate_id) == EVIDENCE_PRODUCT
                    for gate_id in selected_required
                )
            ),
        }
    return report, complete


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--completion-output", type=Path)
    parser.add_argument("--gate-cache", type=Path, default=None)
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
        report, complete = run_validation(
            root, manifest, arguments.phase, gate_cache=arguments.gate_cache
        )
    except (OSError, UnicodeError, ValueError, subprocess.SubprocessError, ValidationError, tomllib.TOMLDecodeError) as error:
        report = {
            "bounded": True,
            "error": str(error),
            "head": initial_head,
            "offline": True,
            "plan_complete": False,
            "schema": "required-gates-report-v2",
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
            "product_complete": True,
            "product_gates": report["product_gates"],
            "required_gates_sha256": report["manifest_sha256"],
            "schema": "plan-complete-v1",
            "validator_sha256": report["validator_sha256"],
        })
    if arguments.phase is not None and arguments.phase_output and "phase_verdict" in report:
        # required-gates-v2 phase verdicts never claim product completion for
        # non-product phases: the artifact carries the schema phase-verdict-v2
        # with an explicit product_complete flag, so a legacy *-COMPLETE path
        # (or filename) can no longer smuggle a completion claim — the old
        # phase-complete-v1 markers are non-authoritative residue.
        atomic_json(arguments.phase_output, {
            "head": report["head"],
            "phase": arguments.phase,
            "producer": "required-gates-validator",
            "required_gates_sha256": report["manifest_sha256"],
            "schema": "phase-verdict-v2",
            **report["phase_verdict"],
        })
    return 0 if complete else 1


if __name__ == "__main__":
    raise SystemExit(main())
