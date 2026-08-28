#!/usr/bin/env python3
"""Validate the active queue and print unclaimed READY item ordinals.

Default output is the legacy space-separated list of unclaimed READY ordinals.
``--state-v1`` prints exactly one of ``RUNNABLE <ordinals>``,
``CLAIMED-RUNNING``, ``AUTOMATIC-WAIT``, or ``REQUIRED-QUEUE-EMPTY`` and exits
0. Invalid input exits 2: state mode prints ``INVALID-DEADLOCKED`` while
default mode prints no stdout; both modes write a diagnostic to stderr.
"""

from __future__ import annotations

import datetime as dt
import fcntl
import hashlib
import os
import re
import stat
import sys
import subprocess
import tomllib
import unicodedata
from pathlib import Path


STATUS_TAGS = {"READY", "WAITING-AUTOMATIC"}
FORBIDDEN_STATUS_TAGS = {
    "BLOCKED",
    "INTERACTIVE",
    "MANUAL",
    "DESKTOP",
    "OPTIONAL",
    "EXTERNAL",
    "LEGAL",
}
METADATA_KEYS = {"id", "gate", "probe", "retry", "timeout", "deadline"}
WAIT_METADATA_KEYS = {"probe", "retry", "timeout", "deadline"}

HEADING_RE = re.compile(r"^(#{1,6})[ \t]+(\S(?:.*\S)?)[ \t]*$")
INDENTED_HEADING_RE = re.compile(r"^[ \t]{1,3}#{1,6}")
ITEM_RE = re.compile(r"^([1-9][0-9]*)\.[ \t]+(.+\S|\S)[ \t]*$")
ITEM_LIKE_RE = re.compile(r"^[0-9]+\.")
TAG_RE = re.compile(r"\[([^\[\]]*)\]")
METADATA_RE = re.compile(r"^([a-z]+)=([^\s=\[\]]+)$")
SAFE_ID_RE = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
PROBE_PART_RE = re.compile(r"^[a-zA-Z0-9][a-zA-Z0-9._-]*$")
DURATION_RE = re.compile(r"^[1-9][0-9]*(?:ms|s|m|h|d)$")
DEADLINE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
OWNER_CLAIM_RE = re.compile(r"^([1-9][0-9]*)-owner\.claim$")
RESERVATION_CLAIM_RE = re.compile(
    r"^([1-9][0-9]*)-([A-Za-z0-9][A-Za-z0-9._-]*)\.claim$"
)
CLAIM_TIME_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:Z|[+-]\d{2}:\d{2})$"
)
MAX_RETRY_SECONDS = 60 * 60
MAX_TIMEOUT_SECONDS = 24 * 60 * 60
MAX_DEADLINE_HORIZON_SECONDS = 7 * 24 * 60 * 60
MAX_QUEUE_BYTES = 1024 * 1024
MAX_ACTIVE_ITEMS = 256
MAX_CLAIM_BYTES = 64 * 1024
MAX_CLAIMS = 256
MAX_PROBE_BYTES = 1024 * 1024
MAX_PROBES = 128

FORBIDDEN_TEXT = (
    ("human", re.compile(r"\bhuman\b")),
    ("operator", re.compile(r"\boperator\b")),
    ("manual", re.compile(r"\bmanual\b")),
    ("interactive", re.compile(r"\binteractive\b")),
    ("desktop", re.compile(r"\bdesktop\b")),
    ("listen/listening", re.compile(r"\blisten(?:ing)?\b")),
    ("visual sign-off", re.compile(r"\bvisual (?:sign off|signoff)\b")),
    (
        "owner approval/signature",
        re.compile(r"\b(?:owner (?:approval|signature)|ownerapproval|ownersignature)\b"),
    ),
    ("sudo", re.compile(r"\bsudo\b")),
    ("credentials/secrets", re.compile(r"\b(?:credentials?|secrets?)\b")),
    (
        "legal/license acceptance",
        re.compile(
            r"\b(?:(?:legal|license) acceptance|legalacceptance|licenseacceptance)\b"
        ),
    ),
)


class QueueError(Exception):
    """The active queue does not satisfy the automation-only grammar."""


def fail(message: str) -> None:
    raise QueueError(message)


def head_blob(root: Path, relative: str, maximum: int, label: str) -> bytes:
    try:
        size_text = subprocess.run(
            ["/usr/bin/git", "-C", str(root), "cat-file", "-s", f"HEAD:{relative}"],
            check=True, capture_output=True, text=True, timeout=10,
        ).stdout.strip()
        if not re.fullmatch(r"[0-9]{1,20}", size_text) or int(size_text) > maximum:
            fail(f"{label} exceeds size limit")
        return subprocess.run(
            ["/usr/bin/git", "-C", str(root), "show", f"HEAD:{relative}"],
            check=True, capture_output=True, timeout=10,
        ).stdout
    except subprocess.CalledProcessError:
        fail(f"{label} is missing at HEAD")


def active_now_lines(text: str) -> list[tuple[int, str]]:
    lines = text.splitlines()
    headings: list[tuple[int, int, str]] = []
    for index, line in enumerate(lines):
        match = HEADING_RE.fullmatch(line)
        if not match:
            continue
        level = len(match.group(1))
        title = match.group(2)
        headings.append((index, level, title))
        if level != 2:
            continue
        normalized_title = unicodedata.normalize("NFKC", title).casefold()
        if re.match(r"^optional(?:$|[^a-z0-9])", normalized_title):
            fail(f"line {index + 1}: Optional section/category is forbidden")

    now_headings = [
        index for index, level, title in headings if level == 2 and title == "Now"
    ]
    if len(now_headings) != 1:
        fail(f"expected exactly one active ## Now section; found {len(now_headings)}")

    now_index = now_headings[0]
    for index in range(now_index + 1):
        line = lines[index]
        if INDENTED_HEADING_RE.match(line):
            fail(f"line {index + 1}: indented headings are not canonical queue structure")
        match = HEADING_RE.fullmatch(line)
        if line.startswith("#") and not match:
            fail(f"line {index + 1}: malformed queue heading")
        if match and len(match.group(1)) == 2 and match.group(2) != "Now":
            fail(f"line {index + 1}: unknown queue section before ## Now")

    start = now_index + 1
    for end in range(start, len(lines)):
        line = lines[end]
        if INDENTED_HEADING_RE.match(line):
            fail(f"line {end + 1}: indented headings are not canonical queue structure")
        match = HEADING_RE.fullmatch(line)
        if line.startswith("#") and not match:
            fail(f"line {end + 1}: malformed queue heading")
        if not match:
            continue
        if len(match.group(1)) == 2 and match.group(2) in {"Backlog", "Done"}:
            return [(index + 1, lines[index]) for index in range(start, end)]
        fail(
            f"line {end + 1}: active ## Now has invalid boundary; "
            "expected ## Backlog or ## Done"
        )

    fail("active ## Now section is truncated; expected ## Backlog or ## Done")


def parse_items(lines: list[tuple[int, str]]) -> list[tuple[str, int, str]]:
    items: list[tuple[str, int, list[str]]] = []
    for line_number, line in lines:
        if not line.strip():
            continue
        match = ITEM_RE.match(line)
        if match:
            ordinal, first_line = match.groups()
            items.append((ordinal, line_number, [first_line]))
            continue
        if ITEM_LIKE_RE.match(line):
            fail(f"line {line_number}: ordinal must be a canonical positive integer")
        if items and line[:1].isspace():
            items[-1][2].append(line.strip())
            continue
        fail(f"line {line_number}: malformed or unnumbered content in active ## Now")

    return [
        (ordinal, line_number, "\n".join(parts))
        for ordinal, line_number, parts in items
    ]


def deadline_epoch(value: str) -> float:
    return dt.datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(
        tzinfo=dt.timezone.utc
    ).timestamp()


def validate_deadline(value: str, ordinal: str) -> None:
    if not DEADLINE_RE.fullmatch(value):
        fail(f"item {ordinal}: malformed deadline; expected YYYY-MM-DDTHH:MM:SSZ")
    try:
        value_epoch = deadline_epoch(value)
    except ValueError:
        fail(f"item {ordinal}: malformed deadline {value!r}")
    now = dt.datetime.now(dt.timezone.utc).timestamp()
    # Expired deadlines remain valid queue syntax. The executor owns expiry so
    # it can publish a structured failure instead of deadlocking the parser.
    if value_epoch - now > MAX_DEADLINE_HORIZON_SECONDS:
        fail(f"item {ordinal}: deadline exceeds the practical bounded horizon")


def duration_seconds(value: str) -> float:
    match = re.fullmatch(r"([1-9][0-9]*)(ms|s|m|h|d)", value)
    if not match:
        raise ValueError(value)
    amount = int(match.group(1))
    multiplier = {"ms": 0.001, "s": 1, "m": 60, "h": 3600, "d": 86400}
    return amount * multiplier[match.group(2)]


def normalized_active_texts(text: str) -> tuple[str, str]:
    normalized = unicodedata.normalize("NFKC", text).casefold()
    normalized = re.sub(r"(?<=\w)-[ \t]*\n[ \t]*(?=\w)", "", normalized)
    words = "".join(
        character if character.isalnum() else " " for character in normalized
    )
    without_punctuation = "".join(
        character
        if character.isalnum() or character.isspace()
        else ""
        for character in normalized
    )
    return " ".join(words.split()), " ".join(without_punctuation.split())


def validate_item(
    ordinal: str, text: str, queue_path: Path | None = None
) -> tuple[str, str, str, dict[str, str]]:
    if re.search(r"\\[\[\]]", text):
        fail(f"item {ordinal}: escaped brackets are not canonical active metadata")
    if any(
        character not in "[]" and unicodedata.normalize("NFKC", character) in {"[", "]"}
        for character in text
    ):
        fail(f"item {ordinal}: noncanonical Unicode bracket in active metadata")
    tags = TAG_RE.findall(text)
    without_tags = TAG_RE.sub("", text)
    if "[" in without_tags or "]" in without_tags:
        fail(f"item {ordinal}: malformed bracket tag or metadata")

    statuses: list[str] = []
    metadata: dict[str, str] = {}
    for tag in tags:
        if tag in STATUS_TAGS:
            statuses.append(tag)
            continue

        normalized_tag = unicodedata.normalize("NFKC", tag).casefold()
        forbidden = next(
            (
                status
                for status in FORBIDDEN_STATUS_TAGS
                if re.match(
                    rf"^{re.escape(status.casefold())}(?:$|[^a-z0-9])",
                    normalized_tag,
                )
            ),
            None,
        )
        if forbidden:
            fail(f"item {ordinal}: forbidden status tag {tag!r} ({forbidden})")

        match = METADATA_RE.fullmatch(tag)
        if not match:
            if "=" in tag:
                fail(f"item {ordinal}: malformed metadata tag [{tag}]")
            fail(f"item {ordinal}: unknown status/tag [{tag}]")
        key, value = match.groups()
        if key not in METADATA_KEYS:
            fail(f"item {ordinal}: unknown metadata tag [{tag}]")
        if key in metadata:
            fail(f"item {ordinal}: duplicate {key} metadata")
        metadata[key] = value

    if len(statuses) != 1:
        fail(
            f"item {ordinal}: expected exactly one READY or "
            f"WAITING-AUTOMATIC status; found {len(statuses)}"
        )
    status = statuses[0]

    ordered = [status, f"id={metadata.get('id', '')}", f"gate={metadata.get('gate', '')}"]
    if status == "WAITING-AUTOMATIC":
        ordered.extend([
            f"probe={metadata.get('probe', '')}",
            f"retry={metadata.get('retry', '')}",
        ])
        if "timeout" in metadata:
            ordered.append(f"timeout={metadata['timeout']}")
        if "deadline" in metadata:
            ordered.append(f"deadline={metadata['deadline']}")
    canonical_prefix = " ".join(f"[{tag}]" for tag in ordered)
    order_check_ready = "id" in metadata and "gate" in metadata
    if status == "WAITING-AUTOMATIC":
        order_check_ready = order_check_ready and "probe" in metadata and "retry" in metadata and ("timeout" in metadata or "deadline" in metadata)
    if order_check_ready and not text.startswith(canonical_prefix + " "):
        fail(f"item {ordinal}: metadata must use canonical status/id/gate/wait order")

    lint_texts = normalized_active_texts(text)
    for forbidden_name, pattern in FORBIDDEN_TEXT:
        if any(pattern.search(lint_text) for lint_text in lint_texts):
            fail(
                f"item {ordinal}: forbidden human-only instruction token "
                f"{forbidden_name!r}"
            )

    for required in ("id", "gate"):
        if required not in metadata:
            fail(f"item {ordinal}: missing required {required} metadata")
        if not SAFE_ID_RE.fullmatch(metadata[required]):
            fail(f"item {ordinal}: unsafe or malformed {required} {metadata[required]!r}")

    if status == "READY":
        inapplicable = sorted(WAIT_METADATA_KEYS.intersection(metadata))
        if inapplicable:
            fail(
                f"item {ordinal}: {', '.join(inapplicable)} metadata applies only "
                "to WAITING-AUTOMATIC"
            )
    else:
        for required in ("probe", "retry"):
            if required not in metadata:
                fail(
                    f"item {ordinal}: WAITING-AUTOMATIC requires {required} metadata"
                )
        if "timeout" not in metadata and "deadline" not in metadata:
            fail(
                f"item {ordinal}: WAITING-AUTOMATIC requires a bounded timeout "
                "or deadline"
            )
        probe = metadata["probe"]
        probe_parts = probe.split("/")
        if any(
            not PROBE_PART_RE.fullmatch(part) or part in {".", ".."}
            for part in probe_parts
        ):
            fail(f"item {ordinal}: unsafe or malformed machine probe reference {probe!r}")
        if not DURATION_RE.fullmatch(metadata["retry"]):
            fail(f"item {ordinal}: retry must be a positive bounded duration")
        if duration_seconds(metadata["retry"]) > MAX_RETRY_SECONDS:
            fail(f"item {ordinal}: retry exceeds the practical maximum")
        if "timeout" in metadata and not DURATION_RE.fullmatch(metadata["timeout"]):
            fail(f"item {ordinal}: timeout must be a positive bounded duration")
        if (
            "timeout" in metadata
            and duration_seconds(metadata["timeout"]) > MAX_TIMEOUT_SECONDS
        ):
            fail(f"item {ordinal}: timeout exceeds the practical maximum")
        if "deadline" in metadata:
            validate_deadline(metadata["deadline"], ordinal)
        if queue_path is not None:
            root = queue_path.parent.parent.resolve(strict=True)
            manifest_relative = "docs/automatic-probes.toml"
            head_manifest = head_blob(root, manifest_relative, MAX_QUEUE_BYTES, "automatic probe allowlist")
            manifest_path = root / manifest_relative
            try:
                manifest_info = manifest_path.lstat()
                if (not stat.S_ISREG(manifest_info.st_mode)
                        or manifest_info.st_size > MAX_QUEUE_BYTES):
                    fail(f"item {ordinal}: automatic probe allowlist is unsafe or oversized")
                with manifest_path.open("rb") as handle:
                    current_manifest = handle.read(MAX_QUEUE_BYTES + 1)
            except OSError as error:
                fail(f"item {ordinal}: committed automatic probe allowlist is missing: {error}")
            if current_manifest != head_manifest:
                fail(f"item {ordinal}: automatic probe allowlist differs from HEAD")
            try:
                manifest = tomllib.loads(head_manifest.decode("utf-8"))
            except (UnicodeError, tomllib.TOMLDecodeError) as error:
                fail(f"item {ordinal}: malformed automatic probe allowlist: {error}")
            entries = manifest.get("probe", [])
            if (manifest.get("schema") != "automatic-probes-v1" or not isinstance(entries, list)
                    or not entries or len(entries) > MAX_PROBES):
                fail(f"item {ordinal}: automatic probe allowlist is empty or invalid")
            allowed: dict[str, dict[str, object]] = {}
            for entry_value in entries:
                if not isinstance(entry_value, dict) or not isinstance(entry_value.get("id"), str):
                    fail(f"item {ordinal}: malformed committed probe allowlist entry")
                entry_id = entry_value["id"]
                if entry_id in allowed:
                    fail(f"item {ordinal}: duplicate committed probe id")
                allowed[entry_id] = entry_value
            entry = allowed.get(probe)
            if entry is None:
                fail(f"item {ordinal}: probe id is not in the committed allowlist: {probe}")
            path_value, expected_hash = entry.get("path"), entry.get("sha256")
            if (not isinstance(path_value, str) or not isinstance(expected_hash, str)
                    or not re.fullmatch(r"[0-9a-f]{64}", expected_hash)):
                fail(f"item {ordinal}: malformed committed probe allowlist entry")
            probe = path_value
            probe_parts = probe.split("/")
            metadata["probe_path"] = probe
            metadata["probe_sha256"] = expected_hash
            head_probe = head_blob(root, probe, MAX_PROBE_BYTES, "allowlisted probe")
            if hashlib.sha256(head_probe).hexdigest() != expected_hash:
                fail(f"item {ordinal}: allowlisted probe digest does not match HEAD")
            if not probe_parts or probe_parts[0] != "tools":
                fail(f"item {ordinal}: machine probe must resolve inside trusted tools")
            if any(not PROBE_PART_RE.fullmatch(part) or part in {".", ".."} for part in probe_parts):
                fail(f"item {ordinal}: unsafe allowlisted probe path")
            probe_path = root / probe
            try:
                probe_info = probe_path.lstat()
                resolved_probe = probe_path.resolve(strict=True)
            except OSError as error:
                fail(f"item {ordinal}: machine probe does not exist: {error}")
            if (
                probe_path.is_symlink()
                or resolved_probe != probe_path
                or not stat.S_ISREG(probe_info.st_mode)
                or not os.access(probe_path, os.X_OK)
            ):
                fail(
                    f"item {ordinal}: machine probe must be a regular non-symlink executable"
                )
            if probe_info.st_size > MAX_PROBE_BYTES:
                fail(f"item {ordinal}: probe exceeds size limit")
            digest = hashlib.sha256()
            with probe_path.open("rb") as handle:
                while chunk := handle.read(64 * 1024):
                    digest.update(chunk)
            if digest.hexdigest() != metadata["probe_sha256"]:
                fail(f"item {ordinal}: mutable probe differs from committed allowlist digest")

    return status, metadata["id"], metadata["gate"], metadata


def validate_queue(
    text: str, queue_path: Path | None = None
) -> list[tuple[str, str, str, str, dict[str, str]]]:
    if len(text.encode("utf-8")) > MAX_QUEUE_BYTES:
        fail("queue exceeds practical size limit")
    parsed = parse_items(active_now_lines(text))
    if len(parsed) > MAX_ACTIVE_ITEMS:
        fail("active queue item count exceeds practical limit")
    seen_ordinals: set[str] = set()
    seen_ids: set[str] = set()
    seen_gates: set[str] = set()
    validated: list[tuple[str, str, str, str, dict[str, str]]] = []

    for ordinal, _line_number, item_text in parsed:
        if ordinal in seen_ordinals:
            fail(f"duplicate ordinal {ordinal} in active ## Now")
        seen_ordinals.add(ordinal)

        status, item_id, gate, metadata = validate_item(
            ordinal, item_text, queue_path
        )
        if item_id in seen_ids:
            fail(f"duplicate id {item_id!r} in active ## Now")
        if gate in seen_gates:
            fail(f"duplicate gate {gate!r} in active ## Now")
        seen_ids.add(item_id)
        seen_gates.add(gate)
        validated.append((ordinal, status, item_id, gate, metadata))

    return validated


def read_queue(path: Path) -> tuple[str, os.stat_result, str]:
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_QUEUE_BYTES:
            fail("queue must be a bounded regular non-symlink file")
        chunks: list[bytes] = []
        while chunk := os.read(descriptor, 1024 * 1024):
            chunks.append(chunk)
        raw = b"".join(chunks)
    finally:
        os.close(descriptor)
    try:
        text = raw.decode("utf-8")
    except UnicodeError as error:
        fail(f"queue is not valid UTF-8: {error}")
    return text, info, hashlib.sha256(raw).hexdigest()


def validate_v2_claim(
    name: str,
    raw: bytes,
    ordinal: str,
    session: str | None,
    active: dict[str, tuple[str, str, str]],
    queue_identity: tuple[int, int, str],
) -> None:
    try:
        lines = raw.decode("utf-8").splitlines()
    except UnicodeError as error:
        fail(f"malformed lock-v2 claim {name}: {error}")
    if not lines or lines[0] != "lock-v2" or lines.count("lock-v2") != 1:
        fail(f"malformed lock-v2 claim {name}: invalid header")
    values: dict[str, str] = {}
    allowed = {
        "ordinal", "id", "gate", "owner", "session", "claimed_at", "unit", "pid",
        "body_sha256", "queue_device", "queue_inode", "queue_sha256",
    }
    for line in lines[1:]:
        if "=" not in line:
            fail(f"malformed lock-v2 claim {name}: invalid field")
        key, value = line.split("=", 1)
        if key not in allowed or key in values or not value:
            fail(f"malformed lock-v2 claim {name}: invalid or duplicate {key}")
        values[key] = value
    required = {
        "ordinal",
        "id",
        "gate",
        "owner",
        "session",
        "claimed_at",
        "unit",
        "pid", "body_sha256", "queue_device", "queue_inode", "queue_sha256",
    }
    if not required.issubset(values):
        fail(f"malformed lock-v2 claim {name}: missing required field")
    claim_session = values["session"]
    if (
        values["ordinal"] != ordinal
        or (session is not None and claim_session != session)
        or not SAFE_ID_RE.fullmatch(values["id"])
        or not SAFE_ID_RE.fullmatch(values["gate"])
        or not RESERVATION_CLAIM_RE.fullmatch(f"1-{claim_session}.claim")
        or values["owner"] != "worker"
        or not CLAIM_TIME_RE.fullmatch(values["claimed_at"])
        or values["unit"] != f"bedlam-nudge-item{ordinal}-{claim_session}"
        or not re.fullmatch(r"[1-9][0-9]*", values["pid"])
    ):
        fail(f"malformed lock-v2 claim {name}: identity mismatch")
    try:
        claimed_at = dt.datetime.fromisoformat(values["claimed_at"].replace("Z", "+00:00"))
    except ValueError:
        fail(f"malformed lock-v2 claim {name}: invalid timestamp")
    now = dt.datetime.now(dt.timezone.utc)
    if claimed_at.tzinfo is None or claimed_at.astimezone(dt.timezone.utc) > now + dt.timedelta(minutes=5):
        fail(f"malformed lock-v2 claim {name}: future timestamp")
    current = active.get(ordinal)
    if current is None or (values["id"], values["gate"]) != current[:2]:
        if session is not None:
            # A reservation authorizes a launch and must stay bound to the
            # exact active identity it was issued against.
            fail(f"lock-v2 claim {name}: active queue identity mismatch")
        # An owner claim whose (id, gate) left the active set is the AGENTS.md
        # step-7 completion-rewrite shape (the claimed item moved to ## Done, a
        # successor may now hold the ordinal) or its post-crash residue. It
        # authorizes nothing here: an unlocked owner claim never suppresses
        # work (see claimed_ordinals), and a locked one only holds its slot
        # while its live wrapper finishes inside the wrapper-enforced
        # boundary grace; the reaper deletes the residue after DEAD_CLAIM_TTL.
        # Failing the whole preflight on this state turned every legitimate
        # completion window into a false INVALID-DEADLOCKED (watchdog repairs
        # 2026-08-27 22:17/23:36, 2026-08-28 00:50/01:55/02:32 - the last one
        # killed the wrapper mid-grace and orphaned the very claim its
        # epilogue was about to release).
        return
    device, inode, queue_sha256 = queue_identity
    if (
        values["body_sha256"] != current[2]
        or values["queue_device"] != str(device)
        or values["queue_inode"] != str(inode)
        or values["queue_sha256"] != queue_sha256
    ):
        fail(f"lock-v2 claim {name}: queue body/hash/identity mismatch")


def validate_locked_v1_claim(name: str, raw: bytes, ordinal: str, fd: int) -> None:
    try:
        text = raw.decode("utf-8")
    except UnicodeError as error:
        fail(f"malformed lock-v1 migration claim {name}: {error}")
    matches = re.findall(
        r"^lock-v1 worker ([A-Za-z0-9][A-Za-z0-9._-]*) "
        r"owns queue item ([1-9][0-9]*)$",
        text,
        re.MULTILINE,
    )
    if len(matches) != 1 or matches[0][1] != ordinal:
        fail(f"malformed lock-v1 migration claim {name}")
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return
    fcntl.flock(fd, fcntl.LOCK_UN)
    fail(f"unlocked lock-v1 migration claim {name}")


def claimed_ordinals(
    claims_path: Path,
    items: list[tuple[str, str, str, str, dict[str, str]]],
    item_bodies: dict[str, str],
    queue_identity: tuple[int, int, str],
) -> set[str]:
    try:
        info = claims_path.lstat()
        if claims_path.is_symlink() or not stat.S_ISDIR(info.st_mode):
            fail(f"claims path is not a directory: {claims_path}")
        directory_fd = os.open(claims_path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            # Trigger race instrumentation only; authoritative enumeration is by
            # the descriptor already pinned above.
            os.scandir(claims_path).close()
            entries = list(os.scandir(directory_fd))
            claim_entries = [entry for entry in entries if entry.name.endswith(".claim")]
            if len(claim_entries) > MAX_CLAIMS:
                fail("claim count exceeds practical limit")
            claimed: set[str] = set()
            active = {
                ordinal: (item_id, gate, item_bodies[ordinal])
                for ordinal, _status, item_id, gate, _metadata in items
            }
            for entry in claim_entries:
                owner_match = OWNER_CLAIM_RE.fullmatch(entry.name)
                reservation_match = RESERVATION_CLAIM_RE.fullmatch(entry.name)
                if owner_match:
                    ordinal, session = owner_match.group(1), None
                elif reservation_match:
                    ordinal, session = reservation_match.groups()
                else:
                    fail(f"malformed canonical claim filename: {entry.name}")
                fd = os.open(entry.name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_fd)
                try:
                    entry_info = os.fstat(fd)
                    if not stat.S_ISREG(entry_info.st_mode) or entry_info.st_size > MAX_CLAIM_BYTES:
                        fail(f"claim is not a regular non-symlink file: {entry.name}")
                    if entry_info.st_uid != os.geteuid() or entry_info.st_mode & 0o022:
                        fail(f"unsafe claim owner or mode: {entry.name}")
                    acquired = False
                    try:
                        fcntl.flock(fd, fcntl.LOCK_SH | fcntl.LOCK_NB)
                        acquired = True
                    except BlockingIOError:
                        # A live worker may hold the same inode exclusively.
                        pass
                    raw = b""
                    while chunk := os.read(fd, 64 * 1024):
                        raw += chunk
                        if len(raw) > MAX_CLAIM_BYTES:
                            fail(f"claim exceeds size limit: {entry.name}")
                    first = raw.splitlines()[0] if raw.splitlines() else b""
                    if owner_match and acquired:
                        # Validate structured bytes so stale/malformed state is
                        # visible, but an unlocked valid owner has no ownership
                        # capability and therefore cannot suppress work.
                        if first == b"lock-v2":
                            validate_v2_claim(entry.name, raw, ordinal, session, active, queue_identity)
                        fcntl.flock(fd, fcntl.LOCK_UN)
                        continue
                    if acquired:
                        fcntl.flock(fd, fcntl.LOCK_UN)
                    if first == b"lock-v2":
                        validate_v2_claim(entry.name, raw, ordinal, session, active, queue_identity)
                    elif session is not None:
                        fail(f"new reservation is not lock-v2: {entry.name}")
                    else:
                        validate_locked_v1_claim(entry.name, raw, ordinal, fd)
                    claimed.add(ordinal)
                finally:
                    os.close(fd)
            return claimed
        finally:
            os.close(directory_fd)
    except OSError as error:
        fail(f"cannot enumerate claims directory {claims_path}: {error}")


def main(argv: list[str]) -> int:
    state_v1 = len(argv) == 4 and argv[3] == "--state-v1"
    item_v1 = len(argv) == 5 and argv[3] == "--item-v1"
    item_v2 = len(argv) == 5 and argv[3] == "--item-v2"
    if len(argv) not in {3, 4, 5} or (
        len(argv) == 4 and not state_v1
    ) or (len(argv) == 5 and not (item_v1 or item_v2)):
        print(
            "usage: nudge-free-items.py NEXT.md CLAIMS_DIR "
            "[--state-v1 | --item-v1 ORDINAL | --item-v2 ORDINAL]",
            file=sys.stderr,
        )
        return 2

    queue_path = Path(argv[1])
    claims_path = Path(argv[2])
    try:
        text, queue_info, queue_sha256 = read_queue(queue_path)
        items = validate_queue(text, queue_path)
        parsed_bodies = {
            ordinal: hashlib.sha256(body.encode("utf-8")).hexdigest()
            for ordinal, _line, body in parse_items(active_now_lines(text))
        }
        claimed = claimed_ordinals(
            claims_path,
            items,
            parsed_bodies,
            (queue_info.st_dev, queue_info.st_ino, queue_sha256),
        )
    except (OSError, UnicodeError) as error:
        if state_v1:
            print("INVALID-DEADLOCKED")
        print(f"invalid queue: cannot read {queue_path}: {error}", file=sys.stderr)
        return 2
    except QueueError as error:
        if state_v1:
            print("INVALID-DEADLOCKED")
        print(f"invalid queue: {error}", file=sys.stderr)
        return 2

    if item_v1 or item_v2:
        requested = argv[4]
        matching = [item for item in items if item[0] == requested]
        if not matching:
            print(f"invalid queue: active item {requested} does not exist", file=sys.stderr)
            return 2
        ordinal, status, item_id, gate, _metadata = matching[0]
        if item_v1:
            print(status, item_id, gate)
        else:
            print(
                status,
                item_id,
                gate,
                parsed_bodies[ordinal],
                queue_info.st_dev,
                queue_info.st_ino,
                queue_sha256,
            )
        return 0

    ready = [
        ordinal
        for ordinal, status, _item_id, _gate, _metadata in items
        if status == "READY"
    ]
    unclaimed_ready = [ordinal for ordinal in ready if ordinal not in claimed]

    if not state_v1:
        print(" ".join(unclaimed_ready))
    elif unclaimed_ready:
        print(f"RUNNABLE {' '.join(unclaimed_ready)}")
    elif ready:
        print("CLAIMED-RUNNING")
    elif any(
        status == "WAITING-AUTOMATIC"
        for _ordinal, status, _item_id, _gate, _metadata in items
    ):
        print("AUTOMATIC-WAIT")
    else:
        print("REQUIRED-QUEUE-EMPTY")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
