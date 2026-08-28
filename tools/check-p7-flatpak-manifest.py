#!/usr/bin/env python3
"""Validate the committed Flatpak build manifest (the p7-flatpak-manifest gate).

Fail-closed checker for the p7-flatpak-manifest required gate
(docs/required-gates.toml, unit p7-flatpak-manifest -- the FIFTH P7
engineering deliverable per PLAN §6 P7 "Linux native + Flatpak" +
docs/P7-PORTS.md §2 row flatpak-manifest, D225). The deliverable is
the committed definition, so the gate grades exactly that definition
hermetically: it parses packaging/dev.roguefort.bedlam.yml,
packaging/dev.roguefort.bedlam.desktop, and the `flatpak` job of
.github/workflows/ci.yml offline (a stdlib-only YAML-subset reader,
the check-p7-ci-artifacts.py family posture) and enforces

  1. FILE DISCIPLINE: the manifest and the desktop entry exist, are
     UTF-8, and parse with the closed YAML subset (indentation
     mappings, block and flow sequences, quoted and plain scalars;
     tabs in indentation, unterminated flow sequences and unparsable
     lines are parse errors -- the file that ships is the file that
     is graded); the desktop entry parses as a one-section INI;
  2. MANIFEST SCHEMA: the closed top-level key set (app-id, runtime,
     runtime-version, sdk, command, finish-args, modules -- unknown
     keys fail); app-id reverse-DNS shaped (>= 3 dot-separated
     alphanumeric segments) and equal to the manifest/desktop file
     stems; the matched runtime pair org.freedesktop.Platform +
     org.freedesktop.Sdk at a PINNED runtime-version (YY.MM); the
     command is the engine binary bedlam-shell;
  3. THE CLOSED SANDBOX SURFACE: finish-args is exactly the five
     contracted tokens (--socket=wayland, --socket=fallback-x11,
     --socket=pulseaudio, --device=dri, --share=ipc) -- no host
     filesystem grant, no network, no bus, no wider device (the
     user-side override stays the user's own choice, never baked in);
  4. THE ENGINE-ONLY MODULE: exactly one module, buildsystem simple,
     build-options append the rust-stable extension and set CARGO_HOME
     inside the build tree; the build-commands carry the reproducible
     engine build (cargo build --release --locked -p bedlam-shell,
     deliberately NOT --offline -- no vendored set is committed, so a
     manifest that could not build fails), the binary install into
     /app/bin, and the desktop install into /app/share/applications;
  5. THE NEVER-BUNDLE GUARD: the single source is a dir source at the
     repo root (path "..") whose skip list carries at least the
     original-asset and scratch trees (.git, game-data, game-data-2,
     derived, derived-2, goldens, ghidra-project, target) -- nothing
     from the corpus ever enters the copy; no other source exists (no
     url/archive/git origin), and outside the skip list no parsed
     value of the manifest references the corpus at all;
  6. THE CI BUILD JOIN: the ci.yml `flatpak` job exists on
     ubuntu-latest, installs flatpak-builder, installs
     org.freedesktop.Sdk//<runtime-version> AND
     org.freedesktop.Sdk.Extension.rust-stable//<runtime-version> at
     the SAME version the manifest pins, builds THIS manifest path
     with flatpak-builder, exports the bundle naming the SAME app-id
     via flatpak build-bundle, and uploads it with
     actions/upload-artifact@v4, if-no-files-found: error and a
     bounded retention; the workflow still triggers per push;
  7. NO SIGNING MATERIAL and NO CORPUS MENTION: the denylist of
     credential and signing tokens (the check-p7-ci-artifacts.py
     family, comments included) matches NOWHERE across the manifest,
     the desktop entry and the flatpak job -- the unsigned bundle is
     the honest output (the D221 signing-keys exclusion) -- and the
     desktop entry and the flatpak job never mention the corpus
     directory at all.

It reads ONLY committed definitions -- no network, no game-data
read, no writes, stdlib only, PATH-free under the validator's
bwrap. The registry flip itself (the flatpak-manifest row landed
with this gate named) is graded by tools/check-p7-ports-map.py, run
as the gate's second command.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

MANIFEST_RELATIVE = "packaging/dev.roguefort.bedlam.yml"
DESKTOP_RELATIVE = "packaging/dev.roguefort.bedlam.desktop"
WORKFLOW_RELATIVE = ".github/workflows/ci.yml"
APP_ID = "dev.roguefort.bedlam"
BINARY = "bedlam-shell"
RUNTIME = "org.freedesktop.Platform"
SDK = "org.freedesktop.Sdk"
RUST_EXTENSION_APPEND_PATH = "/usr/lib/sdk/rust-stable/bin"
CI_JOB = "flatpak"
BUNDLE = "bedlam-shell.flatpak"
ARTIFACT_ACTION = "actions/upload-artifact@v4"
# The closed five-token sandbox surface (order-insensitive, exactly).
FINISH_ARGS_REQUIRED = frozenset(
    {
        "--socket=wayland",
        "--socket=fallback-x11",
        "--socket=pulseaudio",
        "--device=dri",
        "--share=ipc",
    }
)
# Wider grants the surface deliberately excludes (explicit messages).
FINISH_ARGS_FORBIDDEN_PREFIXES = (
    "--filesystem",
    "--share=network",
    "--device=all",
    "--socket=session-bus",
    "--socket=system-bus",
    "--talk-name",
    "--system-talk-name",
)
# The never-bundle skip floor: the dir source MUST skip at least
# these original-asset and scratch trees (extra entries allowed).
SKIP_REQUIRED = frozenset(
    {
        ".git",
        "derived",
        "derived-2",
        "game-data",
        "game-data-2",
        "ghidra-project",
        "goldens",
        "target",
    }
)
CORPUS_TOKEN = "game-data"
MANIFEST_TOP_KEYS = frozenset(
    {
        "app-id",
        "runtime",
        "runtime-version",
        "sdk",
        "command",
        "finish-args",
        "modules",
    }
)
MODULE_KEYS = frozenset(
    {"name", "buildsystem", "build-options", "build-commands", "sources"}
)
SOURCE_KEYS = frozenset({"type", "path", "skip"})
APP_ID_PATTERN = re.compile(r"^[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+){2,}$")
RUNTIME_VERSION_PATTERN = re.compile(r"^[0-9]{2}\.[0-9]{2}$")
# Prefix matches (no trailing \b) so inflections trip too; comments
# included by design (the check-p7-ci-artifacts.py family posture).
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
DESKTOP_KEYS = frozenset(
    {"Type", "Name", "Comment", "Exec", "Terminal", "Categories"}
)


class ManifestError(Exception):
    pass


# ---- the YAML subset reader (the check-p7-ci-artifacts.py family) ---


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
            raise ManifestError(
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
            raise ManifestError(f"unterminated flow sequence: {value!r}")
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
                raise ManifestError(
                    f"bare sequence item with no nested value: {content!r}"
                )
            value, index = parse_node(lines, index + 1, lines[index + 1][0])
            items.append(value)
            continue
        key, separator, _ = rest.partition(":")
        if separator and " " not in key.strip() and not rest.startswith("'"):
            # Inline first key of a mapping item; continuation lines
            # sit at indent + 2 (or deeper, for block scalars).
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
        raise ManifestError("empty sequence")
    return items, index


def parse_mapping(lines: list[tuple[int, str]], index: int, indent: int):
    """A block mapping whose keys sit at exactly `indent`."""
    result: dict[str, object] = {}
    while index < len(lines):
        line_indent, content = lines[index]
        if line_indent != indent:
            if line_indent > indent:
                raise ManifestError(
                    f"unexpected deeper line where a key was due: {content!r}"
                )
            break
        if content == "-" or content.startswith("- "):
            break
        key, separator, value = content.partition(":")
        key = key.strip().strip("'\"")
        if not separator or not key:
            raise ManifestError(f"unparsable mapping line: {content!r}")
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
        raise ManifestError("empty mapping")
    return result, index


def parse_node(lines: list[tuple[int, str]], index: int, indent: int):
    content = lines[index][1]
    if content == "-" or content.startswith("- "):
        return parse_sequence(lines, index, indent)
    return parse_mapping(lines, index, indent)


def parse_document(text: str) -> dict:
    lines = scan_lines(text)
    if not lines:
        raise ManifestError("file has no content lines")
    document, consumed = parse_node(lines, 0, lines[0][0])
    if consumed != len(lines):
        leftover = lines[consumed][1]
        raise ManifestError(f"unparsed trailing content: {leftover!r}")
    if not isinstance(document, dict):
        raise ManifestError("top level is not a mapping")
    return document


# ---- shared loading ----------------------------------------------------


def read_text(root: Path, relative: str) -> str:
    path = root / relative
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ManifestError(f"file is missing: {path}") from error
    try:
        return raw.decode("utf-8")
    except UnicodeError as error:
        raise ManifestError(f"file is not UTF-8: {relative}") from error


def load_yaml(root: Path, relative: str) -> dict:
    return parse_document(read_text(root, relative))


# ---- the manifest rules ------------------------------------------------


def check_no_signing_material(relative: str, text: str) -> None:
    match = SIGNING_PATTERN.search(text)
    if match:
        raise ManifestError(
            f"{relative} carries signing material (denylisted token"
            f" {match.group(0)!r} at offset {match.start()}); the unsigned"
            " bundle is the honest output -- the D221 signing-keys"
            " exclusion"
        )


def check_no_corpus_mention(relative: str, text: str) -> None:
    if CORPUS_TOKEN in text:
        raise ManifestError(
            f"{relative} mentions {CORPUS_TOKEN!r}; the bundle never"
            " carries or reads the corpus -- the user supplies their own"
            " original install"
        )


def _require_str(value: object, what: str) -> str:
    if not isinstance(value, str) or not value:
        raise ManifestError(f"{what} must be a non-empty string")
    return value


def _string_leaves(node: object) -> list[str]:
    leaves: list[str] = []
    if isinstance(node, str):
        leaves.append(node)
    elif isinstance(node, dict):
        for key, value in node.items():
            leaves.append(str(key))
            leaves.extend(_string_leaves(value))
    elif isinstance(node, list):
        for item in node:
            leaves.extend(_string_leaves(item))
    return leaves


def check_manifest(root: Path) -> dict:
    text = read_text(root, MANIFEST_RELATIVE)
    check_no_signing_material(MANIFEST_RELATIVE, text)
    manifest = load_yaml(root, MANIFEST_RELATIVE)

    unknown = set(manifest) - MANIFEST_TOP_KEYS
    if unknown:
        raise ManifestError(
            f"manifest carries unknown top-level keys: {sorted(unknown)}"
        )

    app_id = _require_str(manifest.get("app-id"), "app-id")
    if not APP_ID_PATTERN.match(app_id):
        raise ManifestError(
            f"app-id {app_id!r} is not reverse-DNS shaped (at least"
            " three alphanumeric segments)"
        )
    stem = Path(MANIFEST_RELATIVE).stem
    if app_id != stem:
        raise ManifestError(
            f"app-id {app_id!r} must equal the manifest file stem"
            f" {stem!r} (the desktop entry and the CI bundle command"
            " join on it)"
        )

    if manifest.get("runtime") != RUNTIME:
        raise ManifestError(
            f"runtime must be {RUNTIME!r}, found {manifest.get('runtime')!r}"
        )
    if manifest.get("sdk") != SDK:
        raise ManifestError(
            f"sdk must be {SDK!r}, found {manifest.get('sdk')!r}"
        )
    version = _require_str(
        manifest.get("runtime-version"), "runtime-version"
    )
    if not RUNTIME_VERSION_PATTERN.match(version):
        raise ManifestError(
            f"runtime-version {version!r} is not a pinned YY.MM release"
            " (a floating runtime is not reproducible)"
        )

    command = _require_str(manifest.get("command"), "command")
    if command != BINARY:
        raise ManifestError(
            f"command must be the engine binary {BINARY!r},"
            f" found {command!r}"
        )
    return manifest


def check_finish_args(manifest: dict) -> None:
    args = manifest.get("finish-args")
    if not isinstance(args, list) or not args:
        raise ManifestError("finish-args must be a non-empty sequence")
    for arg in args:
        if not isinstance(arg, str) or not arg:
            raise ManifestError(
                f"finish-args entries must be non-empty strings, found"
                f" {arg!r}"
            )
        for prefix in FINISH_ARGS_FORBIDDEN_PREFIXES:
            if arg == prefix or arg.startswith(prefix + ","):
                raise ManifestError(
                    f"finish-args carries the wider grant {arg!r}; the"
                    " sandbox surface is closed (no host filesystem,"
                    " no network, no bus, no wider device)"
                )
    if len(set(args)) != len(args):
        raise ManifestError("finish-args carries duplicate tokens")
    present = set(args)
    missing = FINISH_ARGS_REQUIRED - present
    extra = present - FINISH_ARGS_REQUIRED
    if missing or extra:
        raise ManifestError(
            "finish-args must be exactly the closed five-token surface"
            f" {sorted(FINISH_ARGS_REQUIRED)} (missing {sorted(missing)},"
            f" extra {sorted(extra)})"
        )


def check_module(manifest: dict) -> tuple[str, dict]:
    modules = manifest.get("modules")
    if not isinstance(modules, list) or len(modules) != 1:
        raise ManifestError(
            "modules must be exactly the one engine module (the bundle"
            f" ships the engine only), found {len(modules) if isinstance(modules, list) else 'non-list'}"
        )
    module = modules[0]
    if not isinstance(module, dict):
        raise ManifestError("the engine module is not a mapping")
    unknown = set(module) - MODULE_KEYS
    if unknown:
        raise ManifestError(
            f"the engine module carries unknown keys: {sorted(unknown)}"
        )
    name = _require_str(module.get("name"), "module name")
    if module.get("buildsystem") != "simple":
        raise ManifestError(
            "the engine module must build with buildsystem simple"
            f" (found {module.get('buildsystem')!r})"
        )

    options = module.get("build-options")
    if not isinstance(options, dict) or set(options) != {
        "append-path",
        "env",
    }:
        raise ManifestError(
            "build-options must carry exactly append-path + env (the"
            " rust-stable extension wiring)"
        )
    if options.get("append-path") != RUST_EXTENSION_APPEND_PATH:
        raise ManifestError(
            "build-options append-path must be the rust-stable extension"
            f" at {RUST_EXTENSION_APPEND_PATH!r} (found"
            f" {options.get('append-path')!r})"
        )
    env = options.get("env")
    if not isinstance(env, dict) or not env:
        raise ManifestError("build-options env must be a mapping")
    cargo_home = env.get("CARGO_HOME")
    if not isinstance(cargo_home, str) or not cargo_home.startswith(
        "/run/build/"
    ):
        raise ManifestError(
            "build-options env CARGO_HOME must live inside the build"
            f" tree (/run/build/...), found {cargo_home!r}"
        )

    commands = module.get("build-commands")
    if not isinstance(commands, list) or not commands:
        raise ManifestError("the engine module carries no build-commands")
    command_text = "\n".join(
        item for item in commands if isinstance(item, str)
    )
    if "cargo build --release" not in command_text:
        raise ManifestError(
            "no build-command runs `cargo build --release` (the engine"
            " build)"
        )
    if "--locked" not in command_text:
        raise ManifestError(
            "the cargo build is not --locked (the bundle must build the"
            " reproducible crate set from the committed Cargo.lock)"
        )
    if f"-p {BINARY}" not in command_text:
        raise ManifestError(
            f"the cargo build must target the engine binary -p {BINARY}"
        )
    if "--offline" in command_text:
        raise ManifestError(
            "the cargo build is --offline but no vendored crate set is"
            " committed -- such a manifest could never build (crates.io"
            " is fetched inside the build sandbox)"
        )
    if f"target/release/{BINARY}" not in command_text or "/app/bin" not in (
        command_text
    ):
        raise ManifestError(
            "no build-command installs target/release/bedlam-shell into"
            " /app/bin (the bundle's single binary)"
        )
    desktop_name = Path(DESKTOP_RELATIVE).name
    if desktop_name not in command_text or "/app/share/applications" not in (
        command_text
    ):
        raise ManifestError(
            f"no build-command installs {desktop_name} into"
            " /app/share/applications"
        )
    return name, module


def check_sources(module: dict) -> list[str]:
    sources = module.get("sources")
    if not isinstance(sources, list) or len(sources) != 1:
        raise ManifestError(
            "the engine module must carry exactly one source (the repo"
            " itself -- no url/archive/git origin, nothing downloaded"
            " from a foreign origin beyond crates.io inside the build)"
        )
    source = sources[0]
    if not isinstance(source, dict):
        raise ManifestError("the source entry is not a mapping")
    unknown = set(source) - SOURCE_KEYS
    if unknown:
        raise ManifestError(
            f"the dir source carries unknown keys: {sorted(unknown)}"
        )
    if source.get("type") != "dir":
        raise ManifestError(
            f"the source must be a dir source, found {source.get('type')!r}"
        )
    if source.get("path") != "..":
        raise ManifestError(
            "the dir source must point at the repo root (path \"..\""
            f" relative to packaging/), found {source.get('path')!r}"
        )
    skip = source.get("skip")
    if not isinstance(skip, list) or not skip:
        raise ManifestError("the dir source carries no skip list")
    for entry in skip:
        if not isinstance(entry, str) or not entry:
            raise ManifestError(
                f"skip entries must be non-empty strings, found {entry!r}"
            )
    if len(set(skip)) != len(skip):
        raise ManifestError("the skip list carries duplicate entries")
    missing = SKIP_REQUIRED - set(skip)
    if missing:
        raise ManifestError(
            "the skip list is missing the never-bundle floor"
            f" {sorted(SKIP_REQUIRED)} (missing {sorted(missing)}) --"
            " nothing from the corpus or its derivatives may ever enter"
            " the copy"
        )
    # Outside the skip list, no parsed value of the manifest may
    # reference the corpus at all (comments are already gone here).
    module_no_sources = {
        key: value for key, value in module.items() if key != "sources"
    }
    for leaf in _string_leaves(module_no_sources):
        if CORPUS_TOKEN in leaf:
            raise ManifestError(
                f"the manifest references {CORPUS_TOKEN!r} outside the"
                " skip list; the app never reads the corpus"
            )
    return list(skip)


def parse_desktop(root: Path) -> dict:
    text = read_text(root, DESKTOP_RELATIVE)
    check_no_signing_material(DESKTOP_RELATIVE, text)
    check_no_corpus_mention(DESKTOP_RELATIVE, text)
    entry: dict[str, str] = {}
    section = ""
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            if section:
                raise ManifestError(
                    "the desktop entry must carry exactly one section"
                    f" ([Desktop Entry]), found {line!r} after it"
                )
            section = line[1:-1]
            if section != "Desktop Entry":
                raise ManifestError(
                    f"the desktop entry section must be [Desktop Entry],"
                    f" found [{section}]"
                )
            continue
        if not section:
            raise ManifestError(
                f"desktop line {number} sits before any [section]:"
                f" {line!r}"
            )
        key, separator, value = line.partition("=")
        if not separator or not key.strip():
            raise ManifestError(f"unparsable desktop line {number}: {line!r}")
        entry[key.strip()] = value.strip()
    if not entry:
        raise ManifestError("the desktop entry has no keys")
    return entry


def check_desktop(root: Path, command: str) -> None:
    entry = parse_desktop(root)
    if "Icon" in entry:
        raise ManifestError(
            "the desktop entry must ship no Icon key -- no asset ever"
            " enters the bundle (D21; a generic icon is the honest"
            " posture)"
        )
    unknown = set(entry) - DESKTOP_KEYS
    if unknown:
        raise ManifestError(
            f"the desktop entry carries unknown keys: {sorted(unknown)}"
        )
    if entry.get("Type") != "Application":
        raise ManifestError(
            f"desktop Type must be Application, found {entry.get('Type')!r}"
        )
    if not entry.get("Name"):
        raise ManifestError("the desktop entry carries no Name")
    if entry.get("Exec") != command:
        raise ManifestError(
            f"desktop Exec must be the manifest command {command!r},"
            f" found {entry.get('Exec')!r}"
        )
    if entry.get("Terminal") != "false":
        raise ManifestError(
            f"desktop Terminal must be false, found"
            f" {entry.get('Terminal')!r}"
        )
    categories = {
        item for item in (entry.get("Categories") or "").split(";") if item
    }
    if categories != {"Game"}:
        raise ManifestError(
            f"desktop Categories must be exactly Game, found"
            f" {entry.get('Categories')!r}"
        )


# ---- the CI build join -------------------------------------------------


def job_text(workflow_text: str, job: dict) -> str:
    """The raw workflow lines belonging to one job (denylist scope)."""
    name = job.get("__name__", "")
    pattern = re.compile(rf"^  {re.escape(name)}:\s*$")
    lines = workflow_text.splitlines()
    start = None
    for index, line in enumerate(lines):
        if pattern.match(line):
            start = index
            break
    if start is None:
        return ""
    chunk = [lines[start]]
    for line in lines[start + 1 :]:
        if line.strip() and not line.startswith("    "):
            if line.startswith("  ") and not line.startswith("   "):
                break
            if not line.startswith("  "):
                break
        chunk.append(line)
    return "\n".join(chunk)


def _run_steps(job: dict) -> list[str]:
    steps = job.get("steps")
    runs: list[str] = []
    for step in steps if isinstance(steps, list) else []:
        if isinstance(step, dict) and isinstance(step.get("run"), str):
            runs.append(step["run"])
    return runs


def check_ci_job(root: Path, manifest: dict) -> str:
    app_id = manifest["app-id"]
    version = manifest["runtime-version"]
    command = manifest["command"]
    workflow_text = read_text(root, WORKFLOW_RELATIVE)
    workflow = load_yaml(root, WORKFLOW_RELATIVE)
    trigger = workflow.get("on")
    if not isinstance(trigger, dict) or "push" not in trigger:
        raise ManifestError(
            "the workflow must carry a top-level push trigger (the"
            " flatpak build is per push)"
        )
    jobs = workflow.get("jobs")
    if not isinstance(jobs, dict) or not jobs:
        raise ManifestError("workflow defines no jobs")
    job = jobs.get(CI_JOB)
    if not isinstance(job, dict):
        raise ManifestError(
            f"the workflow has no `{CI_JOB}` job (the flatpak build"
            " definition)"
        )
    if job.get("runs-on") != "ubuntu-latest":
        raise ManifestError(
            f"the {CI_JOB} job must run on ubuntu-latest (the Linux"
            f" native build platform), found {job.get('runs-on')!r}"
        )
    steps = job.get("steps")
    if not isinstance(steps, list) or not steps:
        raise ManifestError(f"the {CI_JOB} job carries no steps")
    runs = _run_steps(job)
    all_runs = "\n".join(runs)
    if "apt-get install" not in all_runs or "flatpak-builder" not in (
        all_runs
    ):
        raise ManifestError(
            f"the {CI_JOB} job never installs flatpak-builder (the"
            " build tool)"
        )
    if f"{SDK}//{version}" not in all_runs:
        raise ManifestError(
            f"the {CI_JOB} job does not install {SDK}//{version} at the"
            " version the manifest pins (the runtime-version join)"
        )
    extension = f"{SDK}.Extension.rust-stable//{version}"
    if extension not in all_runs:
        raise ManifestError(
            f"the {CI_JOB} job does not install {extension} (the"
            " rust-stable extension the manifest's append-path wires)"
        )
    build_steps = [
        run for run in runs if "flatpak-builder" in run
    ]
    if not any(MANIFEST_RELATIVE in run for run in build_steps):
        raise ManifestError(
            f"the {CI_JOB} job never runs flatpak-builder on"
            f" {MANIFEST_RELATIVE} (THIS manifest is what CI builds)"
        )
    if not any(
        "build-bundle" in run and app_id in run.split()
        for run in runs
    ):
        raise ManifestError(
            f"the {CI_JOB} job never exports the bundle with"
            f" `flatpak build-bundle ... {app_id}` (the app-id join,"
            " matched as a whole command word -- a substring inside the"
            " manifest path is not the join)"
        )
    uploads = [
        step
        for step in steps
        if isinstance(step, dict)
        and step.get("uses") == ARTIFACT_ACTION
        and isinstance(step.get("with"), dict)
    ]
    if not uploads:
        raise ManifestError(
            f"the {CI_JOB} job never uploads the bundle with"
            f" {ARTIFACT_ACTION}"
        )
    for step in uploads:
        with_block = step["with"]
        if not isinstance(with_block.get("name"), str) or not with_block[
            "name"
        ]:
            raise ManifestError(
                "the bundle upload step has no artifact name"
            )
        path = with_block.get("path")
        if path != f"packaging/{BUNDLE}":
            raise ManifestError(
                f"the bundle upload path must be exactly"
                f" packaging/{BUNDLE}, found {path!r}"
            )
        if with_block.get("if-no-files-found") != "error":
            raise ManifestError(
                "the bundle upload must set if-no-files-found: error so a"
                " missing bundle fails the build"
            )
        if "retention-days" not in with_block:
            raise ManifestError(
                "the bundle upload must carry a bounded retention-days"
                " (per-push artifacts do not accumulate)"
            )
    # Scoped denylist: the flatpak job's own text carries no signing
    # vocabulary and never mentions the corpus directory.
    scoped = job_text(workflow_text, {"__name__": CI_JOB})
    if scoped:
        check_no_signing_material(f"ci.yml job {CI_JOB!r}", scoped)
        check_no_corpus_mention(f"ci.yml job {CI_JOB!r}", scoped)
    return uploads[0]["with"]["name"]


def main() -> int:
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=default_root)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    try:
        manifest = check_manifest(root)
        check_finish_args(manifest)
        module_name, module = check_module(manifest)
        skip = check_sources(module)
        check_desktop(root, manifest["command"])
        artifact = check_ci_job(root, manifest)
    except ManifestError as error:
        print(f"p7-flatpak-manifest: FAIL: {error}", file=sys.stderr)
        return 1
    print("p7-flatpak-manifest: OK")
    print(f"  manifest: {MANIFEST_RELATIVE} parses")
    print(
        f"  app: {manifest['app-id']} on {manifest['runtime']}"
        f"//{manifest['runtime-version']} + {manifest['sdk']},"
        f" command {manifest['command']}"
    )
    print(
        "  finish-args: exactly the closed five-token surface"
        f" ({', '.join(sorted(FINISH_ARGS_REQUIRED))})"
    )
    print(
        f"  module: {module_name} -- cargo build --release --locked"
        f" -p {BINARY}, install into /app/bin + /app/share/applications"
    )
    print(
        f"  never-bundle: dir source at the repo root, skip floor"
        f" {sorted(SKIP_REQUIRED)} present ({len(skip)} entries)"
    )
    print(
        f"  ci join: job {CI_JOB!r} on ubuntu-latest builds THIS manifest"
        f" at {manifest['runtime']}//{manifest['runtime-version']},"
        f" bundle {BUNDLE} -> artifact {artifact}"
        f" ({ARTIFACT_ACTION}, if-no-files-found: error)"
    )
    print(
        f"  signing material: none ({len(SIGNING_TOKENS)} denylisted"
        " tokens absent across manifest + desktop + the"
        f" {CI_JOB} job, comments included)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
