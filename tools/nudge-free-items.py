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
import os
import re
import sys
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


def validate_deadline(value: str, ordinal: str) -> None:
    if not DEADLINE_RE.fullmatch(value):
        fail(f"item {ordinal}: malformed deadline; expected YYYY-MM-DDTHH:MM:SSZ")
    try:
        dt.datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError:
        fail(f"item {ordinal}: malformed deadline {value!r}")


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


def validate_item(ordinal: str, text: str) -> tuple[str, str, str]:
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
        if "timeout" in metadata and not DURATION_RE.fullmatch(metadata["timeout"]):
            fail(f"item {ordinal}: timeout must be a positive bounded duration")
        if "deadline" in metadata:
            validate_deadline(metadata["deadline"], ordinal)

    return status, metadata["id"], metadata["gate"]


def validate_queue(text: str) -> list[tuple[str, str]]:
    parsed = parse_items(active_now_lines(text))
    seen_ordinals: set[str] = set()
    seen_ids: set[str] = set()
    seen_gates: set[str] = set()
    validated: list[tuple[str, str]] = []

    for ordinal, _line_number, item_text in parsed:
        if ordinal in seen_ordinals:
            fail(f"duplicate ordinal {ordinal} in active ## Now")
        seen_ordinals.add(ordinal)

        status, item_id, gate = validate_item(ordinal, item_text)
        if item_id in seen_ids:
            fail(f"duplicate id {item_id!r} in active ## Now")
        if gate in seen_gates:
            fail(f"duplicate gate {gate!r} in active ## Now")
        seen_ids.add(item_id)
        seen_gates.add(gate)
        validated.append((ordinal, status))

    return validated


def claimed_ordinals(claims_path: Path) -> set[str]:
    try:
        if not claims_path.is_dir():
            fail(f"claims path is not a directory: {claims_path}")
        with os.scandir(claims_path) as entries:
            return {
                entry.name.split("-", 1)[0]
                for entry in entries
                if entry.name.endswith(".claim")
            }
    except OSError as error:
        fail(f"cannot enumerate claims directory {claims_path}: {error}")


def main(argv: list[str]) -> int:
    state_v1 = len(argv) == 4 and argv[3] == "--state-v1"
    if len(argv) not in {3, 4} or (len(argv) == 4 and not state_v1):
        print(
            "usage: nudge-free-items.py NEXT.md CLAIMS_DIR [--state-v1]",
            file=sys.stderr,
        )
        return 2

    queue_path = Path(argv[1])
    claims_path = Path(argv[2])
    try:
        text = queue_path.read_text(encoding="utf-8")
        items = validate_queue(text)
        claimed = claimed_ordinals(claims_path)
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

    ready = [ordinal for ordinal, status in items if status == "READY"]
    unclaimed_ready = [ordinal for ordinal in ready if ordinal not in claimed]

    if not state_v1:
        print(" ".join(unclaimed_ready))
    elif unclaimed_ready:
        print(f"RUNNABLE {' '.join(unclaimed_ready)}")
    elif ready:
        print("CLAIMED-RUNNING")
    elif any(status == "WAITING-AUTOMATIC" for _ordinal, status in items):
        print("AUTOMATIC-WAIT")
    else:
        print("REQUIRED-QUEUE-EMPTY")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
