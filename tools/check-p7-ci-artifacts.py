#!/usr/bin/env python3
"""Validate the committed per-push CI artifact jobs (the p7-ci-artifacts gate).

Fail-closed checker for the p7-ci-artifacts required gate
(docs/required-gates.toml, unit p7-ci-artifacts -- the FIRST P7
engineering deliverable per PLAN §6 P7 "CI artifacts per push" +
docs/P7-PORTS.md §2 rows ci-artifacts-per-push + linux-native, D222).
The deliverable is the committed workflow definition, so the gate
grades exactly that definition hermetically: it parses
.github/workflows/ci.yml offline and enforces

  1. the file exists, is UTF-8, and parses as the YAML subset the
     repo's workflows use (indentation mappings, block and flow
     sequences, quoted and plain scalars, block scalars `|`; tabs in
     indentation, unterminated flow sequences and unparsable lines are
     parse errors -- the file that ships is the file that is graded);
  2. PER-PUSH TRIGGER: the workflow's top-level `on:` mapping carries
     a `push` key (every push produces the artifacts);
  3. THE RELEASE MATRIX: some job runs the release build
     (`cargo build --release`) on a strategy matrix whose `os` list
     includes BOTH existing legs, ubuntu-latest and windows-latest
     (Linux native + the Windows build);
  4. THE ARTIFACT UPLOADS: that same job carries two
     actions/upload-artifact@v4 steps -- the Linux leg uploads exactly
     the release binary `target/release/bedlam-shell`, the Windows leg
     exactly `target/release/bedlam-shell.exe`, each gated on its
     runner.os, each with a non-empty artifact name and
     `if-no-files-found: error` (a missing binary fails the build,
     never silently ships an empty artifact);
  5. NO SIGNING MATERIAL: a denylist of credential and code-signing
     tokens (GitHub `secrets` references, signtool, codesign,
     notarytool, notariz*, osslsigncode, authenticode, gpg) may appear
     NOWHERE in the workflow, comments included -- the unsigned
     artifact is the honest engineering output (the D221
     signing-keys exclusion), so the file must carry no signing
     vocabulary at all.

It reads ONLY the committed workflow -- no network, no game-data
read, no writes, stdlib only (the D216 tomllib family posture; the
mini-parser below is the YAML analogue), PATH-free under the
validator's bwrap. The registry flip itself (the two rows landed with
this gate named) is graded by tools/check-p7-ports-map.py, run as the
gate's second command.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

WORKFLOW_RELATIVE = ".github/workflows/ci.yml"
ARTIFACT_ACTION = "actions/upload-artifact@v4"
LINUX_BINARY = "target/release/bedlam-shell"
WINDOWS_BINARY = "target/release/bedlam-shell.exe"
LINUX_LEG = "ubuntu-latest"
WINDOWS_LEG = "windows-latest"
RELEASE_BUILD_MARK = "cargo build --release"
# Prefix matches (no trailing \\b) so inflections (notarization,
# codesigned, ...) trip too; comments included by design.
SIGNING_TOKENS = (
    "secrets",
    "signtool",
    "codesign",
    "notarytool",
    "notariz",
    "osslsigncode",
    "authenticode",
    "gpg",
)
SIGNING_PATTERN = re.compile(
    r"(?i)\b(?:" + "|".join(SIGNING_TOKENS) + r")"
)


class WorkflowError(Exception):
    pass


# ---- the YAML subset reader ---------------------------------------------


def strip_comment(line: str) -> str:
    """Drop a full-line or trailing # comment (quote-aware).

    A # starts a comment only at line start or after whitespace, and
    only outside single/double quotes -- quoted values keep their #.
    """
    in_single = False
    in_double = False
    for index, char in enumerate(line):
        if char == "'" and not in_double:
            in_single = not in_single
        elif char == '"' and not in_single:
            in_double = not in_double
        elif char == "#" and not in_single and not in_double:
            if index == 0 or line[index - 1] in " \t":
                return line[:index]
    return line


def scan_lines(text: str) -> list[tuple[int, str]]:
    """(indent, content) for every non-blank, comment-stripped line."""
    scanned: list[tuple[int, str]] = []
    for number, raw in enumerate(text.splitlines(), start=1):
        stripped = strip_comment(raw)
        if not stripped.strip():
            continue
        body = stripped.lstrip(" \t")
        indent_ws = stripped[: len(stripped) - len(body)]
        if "\t" in indent_ws:
            raise WorkflowError(
                f"line {number}: tab in indentation is not YAML"
            )
        content = body.strip()
        if content == "":
            continue
        scanned.append((len(indent_ws), content))
    return scanned


def parse_scalar(value: str) -> object:
    value = value.strip()
    if value.startswith("["):
        if not value.endswith("]"):
            raise WorkflowError(f"unterminated flow sequence: {value!r}")
        inner = value[1:-1].strip()
        if not inner:
            return []
        return [parse_scalar(item) for item in inner.split(",")]
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
        return value[1:-1]
    return value


def parse_sequence(lines: list[tuple[int, str]], index: int, indent: int):
    """A block sequence whose items sit at exactly `indent`."""
    items: list[object] = []
    while index < len(lines):
        line_indent, content = lines[index]
        if line_indent != indent or not (
            content == "-" or content.startswith("- ")
        ):
            break
        rest = "" if content == "-" else content[2:].strip()
        if rest == "":
            if index + 1 >= len(lines) or lines[index + 1][0] <= indent:
                raise WorkflowError(
                    f"bare sequence item with no nested value: {content!r}"
                )
            value, index = parse_node(lines, index + 1, lines[index + 1][0])
            items.append(value)
            continue
        key, separator, _ = rest.partition(":")
        if separator and " " not in key.strip() and not rest.startswith("'"):
            # Inline first key of a mapping item; continuation lines sit
            # at indent + 2 (or deeper, for block scalars).
            virtual = [(indent + 2, rest)]
            scan = index + 1
            while scan < len(lines) and lines[scan][0] >= indent + 2:
                virtual.append(lines[scan])
                scan += 1
            value, _ = parse_mapping(virtual, 0, indent + 2)
            items.append(value)
            index = scan
        else:
            items.append(parse_scalar(rest))
            index += 1
    if not items:
        raise WorkflowError("empty sequence")
    return items, index


def parse_mapping(lines: list[tuple[int, str]], index: int, indent: int):
    """A block mapping whose keys sit at exactly `indent`."""
    result: dict[str, object] = {}
    while index < len(lines):
        line_indent, content = lines[index]
        if line_indent != indent:
            if line_indent > indent:
                raise WorkflowError(
                    f"unexpected deeper line where a key was due: {content!r}"
                )
            break
        if content == "-" or content.startswith("- "):
            break
        key, separator, value = content.partition(":")
        key = key.strip().strip("'\"")
        if not separator or not key:
            raise WorkflowError(f"unparsable mapping line: {content!r}")
        value = value.strip()
        if value in ("|", "|-", "|+", ">", ">-", ">+"):
            chunk: list[str] = []
            scan = index + 1
            while scan < len(lines) and lines[scan][0] > indent:
                chunk.append(lines[scan][1])
                scan += 1
            result[key] = "\n".join(chunk)
            index = scan
        elif value == "":
            if index + 1 < len(lines) and lines[index + 1][0] > indent:
                # A nested mapping OR a deeper-indented sequence
                # (steps:/ - uses: ... both live here).
                nested, index = parse_node(
                    lines, index + 1, lines[index + 1][0]
                )
                result[key] = nested
            elif (
                index + 1 < len(lines)
                and lines[index + 1][0] == indent
                and (
                    lines[index + 1][1] == "-"
                    or lines[index + 1][1].startswith("- ")
                )
            ):
                nested, index = parse_sequence(lines, index + 1, indent)
                result[key] = nested
            else:
                result[key] = None
                index += 1
        else:
            result[key] = parse_scalar(value)
            index += 1
    if not result:
        raise WorkflowError("empty mapping")
    return result, index


def parse_node(lines: list[tuple[int, str]], index: int, indent: int):
    content = lines[index][1]
    if content == "-" or content.startswith("- "):
        return parse_sequence(lines, index, indent)
    return parse_mapping(lines, index, indent)


def parse_workflow(text: str) -> dict:
    lines = scan_lines(text)
    if not lines:
        raise WorkflowError("workflow file has no content lines")
    document, consumed = parse_node(lines, 0, lines[0][0])
    if consumed != len(lines):
        leftover = lines[consumed][1]
        raise WorkflowError(f"unparsed trailing content: {leftover!r}")
    if not isinstance(document, dict):
        raise WorkflowError("workflow top level is not a mapping")
    return document


# ---- the deliverable rules ----------------------------------------------


def load_workflow(root: Path) -> dict:
    path = root / WORKFLOW_RELATIVE
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise WorkflowError(f"workflow file is missing: {path}") from error
    try:
        text = raw.decode("utf-8")
    except UnicodeError as error:
        raise WorkflowError(f"workflow file is not UTF-8: {error}") from error
    return parse_workflow(text)


def check_no_signing_material(text: str) -> None:
    match = SIGNING_PATTERN.search(text)
    if match:
        raise WorkflowError(
            f"workflow carries signing material (denylisted token"
            f" {match.group(0)!r} at offset {match.start()}); the unsigned"
            " artifact is the honest engineering output -- the D221"
            " signing-keys exclusion"
        )


def find_release_matrix_job(workflow: dict) -> tuple[str, dict]:
    jobs = workflow.get("jobs")
    if not isinstance(jobs, dict) or not jobs:
        raise WorkflowError("workflow defines no jobs")
    found: tuple[str, dict] = ("", {})
    for name, job in jobs.items():
        if not isinstance(job, dict):
            raise WorkflowError(f"job {name!r} is not a mapping")
        strategy = job.get("strategy")
        matrix = strategy.get("matrix") if isinstance(strategy, dict) else None
        os_list = matrix.get("os") if isinstance(matrix, dict) else None
        if not isinstance(os_list, list) or not all(
            isinstance(item, str) for item in os_list
        ):
            continue
        if LINUX_LEG not in os_list or WINDOWS_LEG not in os_list:
            continue
        steps = job.get("steps")
        if not isinstance(steps, list):
            continue
        builds = any(
            isinstance(step, dict)
            and isinstance(step.get("run"), str)
            and RELEASE_BUILD_MARK in step["run"]
            for step in steps
        )
        if not builds:
            continue
        if found[0]:
            raise WorkflowError(
                f"workflow carries two release-matrix jobs ({found[0]!r}"
                f" and {name!r}); the per-push artifact surface is ONE"
                " matrix job by design"
            )
        found = (name, job)
    if not found[0]:
        raise WorkflowError(
            "no job builds the release binary on the ubuntu-latest +"
            " windows-latest matrix (the per-push artifact surface needs"
            f" a job running {RELEASE_BUILD_MARK!r} on both legs)"
        )
    return found


def _upload_steps(job: dict, leg_condition: str) -> list[dict]:
    steps = job.get("steps")
    matches: list[dict] = []
    for step in steps if isinstance(steps, list) else []:
        if not isinstance(step, dict):
            continue
        if step.get("uses") != ARTIFACT_ACTION:
            continue
        condition = step.get("if")
        if isinstance(condition, str) and leg_condition in condition:
            matches.append(step)
    return matches


def check_artifact_uploads(
    job: dict, leg: str, leg_condition: str, binary: str
) -> dict:
    steps = _upload_steps(job, leg_condition)
    if not steps:
        raise WorkflowError(
            f"no {ARTIFACT_ACTION} step gated on {leg_condition} in the"
            f" release-matrix job (the {leg} leg must upload the"
            " per-push artifact)"
        )
    for step in steps:
        with_block = step.get("with")
        if not isinstance(with_block, dict):
            raise WorkflowError(
                f"the {leg} artifact-upload step carries no `with:` block"
            )
        name = with_block.get("name")
        if not isinstance(name, str) or not name:
            raise WorkflowError(
                f"the {leg} artifact-upload step has no artifact name"
            )
        path = with_block.get("path")
        if path != binary:
            raise WorkflowError(
                f"the {leg} artifact-upload path must be exactly"
                f" {binary!r} (the release binary), found {path!r}"
            )
        strict = with_block.get("if-no-files-found")
        if strict != "error":
            raise WorkflowError(
                f"the {leg} artifact-upload step must set"
                f" if-no-files-found: error (found {strict!r}) so a"
                " missing binary fails the build instead of shipping an"
                " empty artifact"
            )
    return steps[0]


def main() -> int:
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=default_root)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    try:
        workflow = load_workflow(root)
        path = root / WORKFLOW_RELATIVE
        text = path.read_bytes().decode("utf-8")
        check_no_signing_material(text)
        trigger = workflow.get("on")
        if not isinstance(trigger, dict) or "push" not in trigger:
            raise WorkflowError(
                "workflow must carry a top-level push trigger (the"
                " deliverable is CI artifacts per push)"
            )
        job_name, job = find_release_matrix_job(workflow)
        linux = check_artifact_uploads(
            job, "Linux", "runner.os == 'Linux'", LINUX_BINARY
        )
        windows = check_artifact_uploads(
            job, "Windows", "runner.os == 'Windows'", WINDOWS_BINARY
        )
    except WorkflowError as error:
        print(f"p7-ci-artifacts: FAIL: {error}", file=sys.stderr)
        return 1
    jobs = workflow.get("jobs")
    print("p7-ci-artifacts: OK")
    print(f"  workflow: {WORKFLOW_RELATIVE} parses ({len(jobs)} jobs)")
    print("  trigger: push (every push produces the artifacts)")
    print(
        f"  matrix: {LINUX_LEG} + {WINDOWS_LEG} (job {job_name!r},"
        f" {RELEASE_BUILD_MARK})"
    )
    print(
        f"  artifacts: {linux['with']['name']} <- {LINUX_BINARY},"
        f" {windows['with']['name']} <- {WINDOWS_BINARY}"
        f" ({ARTIFACT_ACTION}, if-no-files-found: error)"
    )
    print(
        f"  signing material: none ({len(SIGNING_TOKENS)} denylisted"
        " tokens absent, comments included)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
