#!/usr/bin/env python3
"""Validate the committed macOS universal2 CI job (the p7-macos-universal2-ci gate).

Fail-closed checker for the p7-macos-universal2-ci required gate
(docs/required-gates.toml, unit p7-macos-universal2-ci -- the SEVENTH
P7 engineering deliverable per PLAN §6 P7 "macOS universal2 through
automated CI" + docs/P7-PORTS.md §2 row macos-universal2-ci, D229).
The deliverable is the committed job definition, so the gate grades
exactly that definition hermetically: it parses
.github/workflows/macos-universal2.yml offline and enforces

  1. the file exists, is UTF-8, and parses as the YAML subset the
     repo's workflows use (the D222 family reader: indentation
     mappings, block and flow sequences, quoted and plain scalars,
     block scalars `|`; tabs in indentation, unterminated flow
     sequences and unparsable lines are parse errors -- the file
     that ships is the file that is graded);
  2. THE SCHEDULED CADENCE: PLAN §3 pins the posture -- "automated
     scheduled macOS CI when a runner is available" -- so the
     workflow carries a `schedule` trigger (at least one cron of
     exactly five space-separated fields) plus `workflow_dispatch`,
     and carries NO `push`/`pull_request` trigger: no push is ever
     gated on a macOS runner existing (the macos-runner-availability
     exclusion made mechanical; the per-push artifact surface stays
     the Linux + Windows ci.yml matrix);
  3. THE UNIVERSAL2 JOB: a `macos-universal2` job runs on a macOS
     runner label, installs BOTH slices via
     dtolnay/rust-toolchain@stable (with.targets carrying
     aarch64-apple-darwin AND x86_64-apple-darwin), builds each
     slice with the reproducible release build (cargo build
     --release --locked -p bedlam-shell --target <slice>, one step
     per slice), joins exactly the two built binaries into ONE
     universal Mach-O with `lipo -create` (pinned output
     staging/bedlam-shell), and uploads that file via
     actions/upload-artifact@v4 as `bedlam-shell-macos-universal2`
     with `if-no-files-found: error` and bounded 14-day retention;
  4. GOLDENS NEVER RUN ON macOS CI (PLAN §3's own boundary): the job
     carries NO test command -- `cargo test`, `--lib`, `diffharness`
     and `goldens` are refused in every run step; this job builds,
     joins and uploads;
  5. NO SIGNING MATERIAL: the credential/code-signing denylist of
     the D222 family (GitHub `secrets` references, signtool,
     codesign, notarytool, notariz*, osslsigncode, authenticode,
     gpg) may appear NOWHERE in the workflow, comments included --
     the unsigned universal binary is the honest engineering output
     (the signing-keys exclusion); and the corpus token `game-data`
     appears nowhere at all (the artifact is the engine binary
     only);
  6. LEAST PRIVILEGE: top-level `permissions: contents: read`, the
     ci.yml/frame-pacing.yml pattern.

The runner itself is EXTERNAL (the macos-runner-availability
exclusion): this checker never requires a macOS machine, a network,
or a game-data read -- it grades the committed definition only
(stdlib only, PATH-free under the validator's bwrap). The registry
flip itself (the row landed with this gate named) is graded by
tools/check-p7-ports-map.py, run as the gate's second command.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

WORKFLOW_RELATIVE = ".github/workflows/macos-universal2.yml"
JOB_ID = "macos-universal2"
ARTIFACT_ACTION = "actions/upload-artifact@v4"
TOOLCHAIN_ACTION = "dtolnay/rust-toolchain@stable"
AARCH64_SLICE = "aarch64-apple-darwin"
X86_64_SLICE = "x86_64-apple-darwin"
BUILD_AARCH64 = (
    "cargo build --release --locked -p bedlam-shell"
    f" --target {AARCH64_SLICE}"
)
BUILD_X86_64 = (
    "cargo build --release --locked -p bedlam-shell"
    f" --target {X86_64_SLICE}"
)
AARCH64_BINARY = f"target/{AARCH64_SLICE}/release/bedlam-shell"
X86_64_BINARY = f"target/{X86_64_SLICE}/release/bedlam-shell"
LIPO_OUTPUT = "staging/bedlam-shell"
ARTIFACT_NAME = "bedlam-shell-macos-universal2"
RETENTION_DAYS = "14"
# Prefix matches (no trailing \b) so inflections (notarization,
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
CORPUS_PATTERN = re.compile(r"(?i)game-data")
# PLAN §3: "goldens never run on macOS CI" -- the job is
# build + join + upload, nothing else.
TEST_TOKENS = ("cargo test", "--lib", "diffharness", "goldens")


class WorkflowError(Exception):
    pass


# ---- the YAML subset reader (the D222 family reader, verbatim) ------------


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


def load_workflow(root: Path) -> tuple[dict, str]:
    path = root / WORKFLOW_RELATIVE
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise WorkflowError(f"workflow file is missing: {path}") from error
    try:
        text = raw.decode("utf-8")
    except UnicodeError as error:
        raise WorkflowError(f"workflow file is not UTF-8: {error}") from error
    return parse_workflow(text), text


def check_no_signing_material(text: str) -> None:
    match = SIGNING_PATTERN.search(text)
    if match:
        raise WorkflowError(
            f"workflow carries signing material (denylisted token"
            f" {match.group(0)!r} at offset {match.start()}); the unsigned"
            " universal binary is the honest engineering output -- the D221"
            " signing-keys exclusion"
        )


def check_no_corpus_token(text: str) -> None:
    match = CORPUS_PATTERN.search(text)
    if match:
        raise WorkflowError(
            f"workflow mentions the original-install directory (token"
            f" {match.group(0)!r} at offset {match.start()}); the macOS"
            " artifact is the ENGINE BINARY ONLY and the user supplies"
            " their own install at run time"
        )


def _steps(job: dict, job_id: str) -> list[dict]:
    steps = job.get("steps")
    if not isinstance(steps, list) or not steps:
        raise WorkflowError(
            f"job {job_id!r} carries no steps (the universal2 build lives"
            " there)"
        )
    for step in steps:
        if not isinstance(step, dict):
            raise WorkflowError(f"job {job_id!r} has a step that is not a"
                                " mapping")
    return steps


def _run_strings(steps: list[dict]) -> list[str]:
    return [
        step["run"]
        for step in steps
        if isinstance(step.get("run"), str) and step["run"]
    ]


def check_trigger(workflow: dict) -> str:
    trigger = workflow.get("on")
    if not isinstance(trigger, dict):
        raise WorkflowError(
            "workflow must carry a top-level `on:` trigger mapping"
        )
    for forbidden in ("push", "pull_request"):
        if forbidden in trigger:
            raise WorkflowError(
                f"workflow must NOT carry a {forbidden!r} trigger: the macOS"
                " cadence is SCHEDULED (PLAN §3 'automated scheduled macOS"
                " CI when a runner is available'), so no push is ever gated"
                " on a macOS runner existing (the macos-runner-availability"
                " exclusion); the per-push artifact surface stays the Linux"
                " + Windows ci.yml matrix"
            )
    schedule = trigger.get("schedule")
    if not isinstance(schedule, list) or not schedule:
        raise WorkflowError(
            "workflow must carry a scheduled trigger (PLAN §3: automated"
            " scheduled macOS CI -- at least one `schedule:` cron entry)"
        )
    crons: list[str] = []
    for entry in schedule:
        if not isinstance(entry, dict) or not isinstance(
            entry.get("cron"), str
        ):
            raise WorkflowError(
                "every schedule entry must carry a `cron` string"
            )
        fields = entry["cron"].split()
        if len(fields) != 5 or not all(fields):
            raise WorkflowError(
                f"cron {entry['cron']!r} must have exactly 5 non-empty"
                " space-separated fields"
            )
        crons.append(entry["cron"])
    if "workflow_dispatch" not in trigger:
        raise WorkflowError(
            "workflow must carry a workflow_dispatch trigger (a manual"
            " verification run when a runner is present)"
        )
    return crons[0]


def check_permissions(workflow: dict) -> None:
    permissions = workflow.get("permissions")
    if not isinstance(permissions, dict) or permissions.get("contents") != "read":
        raise WorkflowError(
            "workflow must pin least-privilege top-level permissions"
            " (contents: read, the ci.yml/frame-pacing.yml pattern)"
        )


def find_job(workflow: dict) -> dict:
    jobs = workflow.get("jobs")
    if not isinstance(jobs, dict) or not jobs:
        raise WorkflowError("workflow defines no jobs")
    job = jobs.get(JOB_ID)
    if not isinstance(job, dict):
        raise WorkflowError(
            f"workflow defines no {JOB_ID!r} job (the universal2 deliverable"
            " is that job definition)"
        )
    return job


def check_runner_label(job: dict) -> str:
    runs_on = job.get("runs-on")
    label = runs_on if isinstance(runs_on, str) else ""
    if not label.startswith("macos-"):
        raise WorkflowError(
            f"the {JOB_ID} job must run on a macOS runner label"
            f" (macos-...), found {runs_on!r}"
        )
    return label


def check_toolchain_targets(steps: list[dict]) -> None:
    for step in steps:
        if step.get("uses") != TOOLCHAIN_ACTION:
            continue
        with_block = step.get("with")
        targets = with_block.get("targets") if isinstance(with_block, dict) else None
        if isinstance(targets, str) and AARCH64_SLICE in targets and X86_64_SLICE in targets:
            return
    raise WorkflowError(
        f"the {JOB_ID} job must install BOTH universal2 slices via"
        f" {TOOLCHAIN_ACTION} (with.targets carrying"
        f" {AARCH64_SLICE} AND {X86_64_SLICE})"
    )


def check_build_step(steps: list[dict], slice_name: str, command: str) -> None:
    runs = _run_strings(steps)
    if not any(command in run for run in runs):
        raise WorkflowError(
            f"the {slice_name} build step must run exactly"
            f" {command!r} (the reproducible release build of that slice;"
            " --locked pins the committed set, and it is deliberately not"
            " --offline because no vendored set is committed)"
        )


def check_lipo_join(steps: list[dict]) -> None:
    joins = [
        run for run in _run_strings(steps) if "lipo -create" in run
    ]
    if not joins:
        raise WorkflowError(
            f"the {JOB_ID} job must join the two slices with `lipo -create`"
            f" into {LIPO_OUTPUT!r} (the universal2 deliverable is ONE"
            " binary carrying BOTH architectures)"
        )
    join = joins[0]
    if f"-output {LIPO_OUTPUT}" not in join:
        raise WorkflowError(
            f"the lipo -create step must write exactly -output"
            f" {LIPO_OUTPUT!r} (the staged universal binary the upload"
            " step pins)"
        )
    for binary in (AARCH64_BINARY, X86_64_BINARY):
        if binary not in join:
            raise WorkflowError(
                f"the lipo -create step must consume exactly the built"
                f" binary {binary!r} (the join is over the two cargo"
                " --target builds and nothing else)"
            )


def check_upload(steps: list[dict]) -> dict:
    candidates = [
        step for step in steps if step.get("uses") == ARTIFACT_ACTION
    ]
    named = [
        step
        for step in candidates
        if isinstance(step.get("with"), dict)
        and step["with"].get("name") == ARTIFACT_NAME
    ]
    if not named:
        raise WorkflowError(
            f"no {ARTIFACT_ACTION} step uploads the artifact"
            f" {ARTIFACT_NAME!r} (the macOS universal2 per-run artifact)"
        )
    step = named[0]
    with_block = step["with"]
    path = with_block.get("path")
    if path != LIPO_OUTPUT:
        raise WorkflowError(
            f"the {ARTIFACT_NAME} upload path must be exactly"
            f" {LIPO_OUTPUT!r} (the lipo output), found {path!r}"
        )
    strict = with_block.get("if-no-files-found")
    if strict != "error":
        raise WorkflowError(
            f"the {ARTIFACT_NAME} upload must set if-no-files-found:"
            f" error (found {strict!r}) so a missing universal binary"
            " fails the run instead of shipping an empty artifact"
        )
    retention = with_block.get("retention-days")
    if retention != RETENTION_DAYS:
        raise WorkflowError(
            f"the {ARTIFACT_NAME} upload must bound retention at"
            f" retention-days: {RETENTION_DAYS} (found {retention!r}), the"
            " D222 per-run-artifact posture"
        )
    return step


def check_no_test_commands(steps: list[dict]) -> None:
    for run in _run_strings(steps):
        for token in TEST_TOKENS:
            if token in run:
                raise WorkflowError(
                    f"the {JOB_ID} job carries a test command"
                    f" ({token!r} in a run step) -- goldens never run on"
                    " macOS CI (PLAN §3); this job builds, joins and"
                    " uploads"
                )


def main() -> int:
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=default_root)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    try:
        workflow, text = load_workflow(root)
        check_no_signing_material(text)
        check_no_corpus_token(text)
        cron = check_trigger(workflow)
        check_permissions(workflow)
        job = find_job(workflow)
        label = check_runner_label(job)
        steps = _steps(job, JOB_ID)
        check_no_test_commands(steps)
        check_toolchain_targets(steps)
        check_build_step(steps, AARCH64_SLICE, BUILD_AARCH64)
        check_build_step(steps, X86_64_SLICE, BUILD_X86_64)
        check_lipo_join(steps)
        check_upload(steps)
    except WorkflowError as error:
        print(f"p7-macos-universal2-ci: FAIL: {error}", file=sys.stderr)
        return 1
    jobs = workflow.get("jobs")
    print("p7-macos-universal2-ci: OK")
    print(f"  workflow: {WORKFLOW_RELATIVE} parses ({len(jobs)} jobs)")
    print(
        f"  trigger: scheduled (cron {cron!r}) + workflow_dispatch, no"
        " push/pull_request trigger (PLAN sec 3 posture: no push waits on"
        " a macOS runner; the runner is the macos-runner-availability"
        " exclusion)"
    )
    print(f"  job: {JOB_ID} on {label}")
    print(
        f"  universal2: {AARCH64_SLICE} + {X86_64_SLICE} -> lipo -create"
        f" -> {LIPO_OUTPUT}"
    )
    print(
        f"  artifact: {ARTIFACT_NAME} <- {LIPO_OUTPUT}"
        f" ({ARTIFACT_ACTION}, if-no-files-found: error,"
        f" {RETENTION_DAYS}-day retention)"
    )
    print(
        "  tests: none in the job (goldens never run on macOS CI --"
        " build, join, upload)"
    )
    print(
        f"  signing material: none ({len(SIGNING_TOKENS)} denylisted"
        " tokens absent, comments included); corpus token absent"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
