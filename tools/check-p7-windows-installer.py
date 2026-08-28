#!/usr/bin/env python3
"""Validate the committed Windows installer definition (the p7-windows-installer gate).

Fail-closed checker for the p7-windows-installer required gate
(docs/required-gates.toml, unit p7-windows-installer -- the SIXTH P7
engineering deliverable per PLAN §6 P7 "Windows installer" +
docs/P7-PORTS.md §2 row windows-installer, D227). The deliverable is
the committed definition, so the gate grades exactly that definition
hermetically: it parses packaging/bedlam-shell.nsi,
packaging/windows-installer-README.txt, and the `windows-installer`
job of .github/workflows/ci.yml offline (stdlib only -- a CLOSED
NSIS COMMAND GRAMMAR for the script and the repo's stdlib-only
YAML-subset reader for the workflow, the check-p7-flatpak-manifest.py
family posture) and enforces

  1. FILE DISCIPLINE: the script and the README exist, are UTF-8,
     and the script parses under the closed grammar -- every line
     is one command from the closed set (Name, OutFile, Unicode,
     InstallDir, RequestExecutionLevel, CRCCheck, Page, UninstPage,
     Section, SectionEnd, SetOutPath, File, WriteUninstaller,
     WriteRegStr, DeleteRegKey, CreateDirectory, CreateShortcut,
     Delete, RMDir) with the exact argument shape that command
     pins; unknown commands (plug-ins, compiler directives, labels),
     unquoted string arguments, unbalanced quotes, C-style
     comments, line continuations, wildcards and path separators
     in File sources, and switches on Delete/RMDir are all parse
     errors -- the file that ships is the file that is graded;
  2. INSTALLER SCHEMA: Name "Bedlam engine"; OutFile exactly
     bedlam-shell-setup.exe (the file the CI job uploads);
     Unicode true; InstallDir $PROGRAMFILES64\\Bedlam
     (RequestExecutionLevel admin -- both pinned, the manual's
     64-bit guidance); CRCCheck force (the installer CRCs itself
     and the user cannot skip it); the minimal page flow
     directory + instfiles (uninstaller: uninstConfirm +
     instfiles); exactly two sections -- the install section and
     one un.-prefixed uninstaller section;
  3. THE ENGINE-ONLY FILE SET: the install section body is pinned
     instruction-for-instruction -- SetOutPath $INSTDIR, exactly
     TWO File lines (bedlam-shell.exe + windows-installer-
     README.txt, both staged bare names next to the script, no
     wildcard, no path, so nothing else can ride along),
     WriteUninstaller, the Add/Remove-Programs registration
     (HKLM Uninstall\\BedlamEngine DisplayName + UninstallString),
     CreateDirectory $SMPROGRAMS\\Bedlam, and ONE CreateShortcut
     whose target is the installed engine binary and whose
     working directory is $INSTDIR (NSIS stores $OUTDIR as the
     shortcut's working directory, and SetOutPath $INSTDIR runs
     first -- the engine's documented default lookup root sits
     directly inside the install folder); the uninstall section
     removes exactly what the installer wrote (every Delete names
     an installed artifact; RMDir only ever removes EMPTY
     directories -- the recursive switch cannot even parse);
  4. THE README CONTRACT: the file the installer drops next to
     the binary is honest user documentation -- non-empty, UTF-8,
     carrying the engine-only boundary and the supply-your-own
     sentence plus the documented default layout game-data\\BEDLAM
     (the ONLY shape in which the corpus token may appear in it),
     and no signing vocabulary;
  5. THE CI BUILD JOIN: the ci.yml `windows-installer` job exists
     on windows-latest, per-push trigger, checks out, installs the
     stable Rust toolchain, builds the engine reproducibly
     (cargo build --release --locked -p bedlam-shell, deliberately
     NOT --offline -- no vendored set is committed), installs NSIS
     via chocolatey, stages target\\release\\bedlam-shell.exe as
     packaging\\bedlam-shell.exe, runs makensis with
     working-directory: packaging on THIS script name, and uploads
     packaging/bedlam-shell-setup.exe with actions/upload-
     artifact@v4, if-no-files-found: error and a bounded retention;
  6. NO SIGNING MATERIAL and NO CORPUS MENTION: the denylist of
     credential and code-signing tokens (the check-p7-ci-artifacts.py
     family, comments included) matches NOWHERE across the script,
     the README and the windows-installer job -- the unsigned
     installer is the honest output (the D221 signing-keys
     exclusion) -- and the script and the job never mention the
     corpus directory at all (the README may name it only inside
     the documented default layout game-data\\BEDLAM).

It reads ONLY committed definitions -- no network, no game-data
read, no writes, stdlib only, PATH-free under the validator's
bwrap. The registry flip itself (the windows-installer row landed
with this gate named) is graded by tools/check-p7-ports-map.py, run
as the gate's second command.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

SCRIPT_RELATIVE = "packaging/bedlam-shell.nsi"
README_RELATIVE = "packaging/windows-installer-README.txt"
WORKFLOW_RELATIVE = ".github/workflows/ci.yml"
CI_JOB = "windows-installer"
APP_NAME = "Bedlam engine"
BINARY = "bedlam-shell.exe"
README_STAGED = "windows-installer-README.txt"
SETUP_EXE = "bedlam-shell-setup.exe"
ARTIFACT_ACTION = "actions/upload-artifact@v4"
ARTIFACT_NAME = "bedlam-shell-windows-installer-x86_64"
INSTALL_DIR = "$PROGRAMFILES64\\Bedlam"
UNINSTALL_KEY = (
    "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BedlamEngine"
)
STARTMENU_DIR = "$SMPROGRAMS\\Bedlam"
SHORTCUT = "$SMPROGRAMS\\Bedlam\\Bedlam engine.lnk"
INSTALLED_BINARY = "$INSTDIR\\" + BINARY
INSTALLED_README = "$INSTDIR\\" + README_STAGED
UNINSTALLER = "$INSTDIR\\uninstall.exe"
# The exact closed page flow (minimal wizard: pick a directory,
# install; the uninstaller confirms, then uninstalls).
PAGE_FLOW = [
    ("Page", "directory"),
    ("Page", "instfiles"),
    ("UninstPage", "uninstConfirm"),
    ("UninstPage", "instfiles"),
]
# The install section body, instruction-for-instruction.
INSTALL_BODY: list[tuple[str, list[str]]] = [
    ("SetOutPath", ["$INSTDIR"]),
    ("File", [BINARY]),
    ("File", [README_STAGED]),
    ("WriteUninstaller", [UNINSTALLER]),
    (
        "WriteRegStr",
        ["HKLM", UNINSTALL_KEY, "DisplayName", APP_NAME],
    ),
    (
        "WriteRegStr",
        ["HKLM", UNINSTALL_KEY, "UninstallString", UNINSTALLER],
    ),
    ("CreateDirectory", [STARTMENU_DIR]),
    ("CreateShortcut", [SHORTCUT, INSTALLED_BINARY]),
]
# The uninstall section body, instruction-for-instruction.
UNINSTALL_BODY: list[tuple[str, list[str]]] = [
    ("Delete", [SHORTCUT]),
    ("RMDir", [STARTMENU_DIR]),
    ("Delete", [INSTALLED_BINARY]),
    ("Delete", [INSTALLED_README]),
    ("Delete", [UNINSTALLER]),
    ("DeleteRegKey", ["HKLM", UNINSTALL_KEY]),
    ("RMDir", ["$INSTDIR"]),
]
REG_ROOTS = frozenset(
    {"HKCR", "HKLM", "HKCU", "HKU", "HKCC", "SHCTX"}
)
PAGE_TYPES = frozenset(
    {"license", "components", "directory", "instfiles", "uninstConfirm"}
)
# A staged File source: a bare file name sitting next to the script
# -- no separators, no wildcards, no drive, no parent hop.
STAGED_FILE_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
CORPUS_TOKEN = "game-data"
CORPUS_DEFAULT_LAYOUT = "game-data\\BEDLAM"
# README boundary sentences (whitespace-normalized matching).
README_SENTENCES = (
    "This install carries the ENGINE ONLY",
    "You supply your own original Bedlam install",
    "game-data\\BEDLAM",
)
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


class InstallerError(Exception):
    pass


# ---- the closed NSIS grammar ---------------------------------------------


def strip_nsis_comment(line: str) -> str:
    """Drop a full-line or trailing ;/# comment (quote-aware).

    A ; or # starts a comment only at line start or after
    whitespace, and only outside double quotes -- NSIS keeps
    quoted ; and # as string content (the manual's Script File
    Format section); quoted values keep their comment chars.
    """
    in_double = False
    for index, char in enumerate(line):
        if char == '"' and (index == 0 or line[index - 1] != "$"):
            in_double = not in_double
        elif char in (";", "#") and not in_double:
            if index == 0 or line[index - 1] in " \t":
                return line[:index]
    return line


def tokenize(content: str, number: int) -> list[tuple[str, bool]]:
    """Split one command line into (token, was-quoted) pairs.

    A double-quoted string is one token (the quotes are dropped,
    the quoting is remembered -- the grammar pins which arguments
    are quoted strings and which are bare words); anything else is
    whitespace-separated bare words. A bare word carrying a stray
    quote is an unbalanced-quote parse error.
    """
    tokens: list[tuple[str, bool]] = []
    index = 0
    while index < len(content):
        if content[index].isspace():
            index += 1
            continue
        if content[index] == '"':
            end = content.find('"', index + 1)
            if end < 0:
                raise InstallerError(
                    f"script line {number}: unbalanced quote in:"
                    f" {content!r}"
                )
            tokens.append((content[index + 1 : end], True))
            index = end + 1
        else:
            start = index
            while index < len(content) and not content[index].isspace():
                index += 1
            word = content[start:index]
            if '"' in word:
                raise InstallerError(
                    f"script line {number}: quote inside the bare"
                    f" argument {word!r} (string arguments are quoted"
                    " whole)"
                )
            tokens.append((word, False))
    return tokens


# command -> (arg kinds); q = quoted string, b = bare word
COMMAND_GRAMMAR: dict[str, tuple[str, ...]] = {
    "Name": ("q",),
    "OutFile": ("q",),
    "InstallDir": ("q",),
    "SetOutPath": ("q",),
    "WriteUninstaller": ("q",),
    "CreateDirectory": ("q",),
    "File": ("q",),
    "Delete": ("q",),
    "RMDir": ("q",),
    "Unicode": ("b",),
    "Section": ("q",),
    "CreateShortcut": ("q", "q"),
    "WriteRegStr": ("b", "q", "q", "q"),
    "DeleteRegKey": ("b", "q"),
    # checked with their own closed value domains below
    "RequestExecutionLevel": ("b",),
    "CRCCheck": ("b",),
    "Page": ("b",),
    "UninstPage": ("b",),
    "SectionEnd": (),
}
SECTION_COMMANDS = frozenset(
    {
        "SetOutPath",
        "File",
        "WriteUninstaller",
        "WriteRegStr",
        "DeleteRegKey",
        "CreateDirectory",
        "CreateShortcut",
        "Delete",
        "RMDir",
    }
)


def parse_script(text: str) -> list[tuple[str, str, list[str]]]:
    """Parse the script into (command, bare-arg, args) records.

    Section/SectionEnd pairs are validated inline (nesting depth
    exactly one); the records keep their in-section order so the
    semantic pass can pin the section bodies instruction-for-
    instruction. Every rule is fail-closed.
    """
    records: list[tuple[str, str, list[str]]] = []
    depth = 0
    for number, raw in enumerate(text.splitlines(), start=1):
        if "/*" in raw or "*/" in raw:
            raise InstallerError(
                f"script line {number}: C-style comments are outside"
                " the closed grammar"
            )
        stripped = strip_nsis_comment(raw)
        content = stripped.strip()
        if not content:
            continue
        if content.endswith("\\"):
            raise InstallerError(
                f"script line {number}: line continuations are"
                " outside the closed grammar (one command per line)"
            )
        parts = content.split(None, 1)
        command = parts[0]
        rest = parts[1] if len(parts) > 1 else ""
        if command not in COMMAND_GRAMMAR:
            raise InstallerError(
                f"script line {number}: unknown command {command!r}"
                " (plug-ins, compiler directives, labels and every"
                " other shape are outside the closed grammar)"
            )
        if command == "SectionEnd":
            if depth == 0:
                raise InstallerError(
                    f"script line {number}: SectionEnd outside any"
                    " section"
                )
            depth -= 1
            records.append((command, "", []))
            continue
        if command == "Section":
            if depth != 0:
                raise InstallerError(
                    f"script line {number}: nested Section"
                )
            depth += 1
        kinds = COMMAND_GRAMMAR[command]
        tokens = tokenize(rest, number)
        if len(tokens) != len(kinds):
            raise InstallerError(
                f"script line {number}: {command} takes"
                f" {len(kinds)} argument(s), found {len(tokens)}"
                f" in: {content!r}"
            )
        for (token, quoted), kind in zip(tokens, kinds):
            if kind == "q" and not quoted:
                raise InstallerError(
                    f"script line {number}: string argument must be"
                    f" quoted whole in: {content!r}"
                )
            if kind == "b" and quoted:
                raise InstallerError(
                    f"script line {number}: bare argument arrived"
                    f" quoted in: {content!r}"
                )
        if command in ("RequestExecutionLevel", "CRCCheck", "Page", "UninstPage"):
            if command == "RequestExecutionLevel" and tokens[0][0] not in {
                "none",
                "user",
                "highest",
                "admin",
            }:
                raise InstallerError(
                    f"script line {number}: unknown"
                    f" RequestExecutionLevel {tokens[0][0]!r}"
                )
            if command == "CRCCheck" and tokens[0][0] not in {
                "on",
                "off",
                "force",
            }:
                raise InstallerError(
                    f"script line {number}: unknown CRCCheck value"
                    f" {tokens[0][0]!r}"
                )
            if command in ("Page", "UninstPage") and tokens[0][0] not in (
                PAGE_TYPES
            ):
                raise InstallerError(
                    f"script line {number}: {command} type"
                    f" {tokens[0][0]!r} is not a built-in page (custom"
                    " pages are outside the closed grammar)"
                )
        if command == "WriteRegStr" and tokens[0][0] not in REG_ROOTS:
            raise InstallerError(
                f"script line {number}: unknown registry root"
                f" {tokens[0][0]!r}"
            )
        if command == "DeleteRegKey" and tokens[0][0] not in REG_ROOTS:
            raise InstallerError(
                f"script line {number}: unknown registry root"
                f" {tokens[0][0]!r}"
            )
        if command == "File":
            source = tokens[0][0]
            if not STAGED_FILE_PATTERN.match(source):
                raise InstallerError(
                    f"script line {number}: File source {source!r}"
                    " is not a staged bare file name next to the"
                    " script (no paths, no wildcards -- the closed"
                    " two-file set rides along and nothing else)"
                )
        # A string argument that is not quoted cannot even reach
        # here (unquoted words parse as bare), but the empty-string
        # quote "" would: reject empties outright.
        for (token, _quoted), kind in zip(tokens, kinds):
            if kind == "q" and token == "":
                raise InstallerError(
                    f"script line {number}: empty string argument"
                    f" in: {content!r}"
                )
        in_section = depth > 0 or command == "Section"
        if command in SECTION_COMMANDS and not in_section:
            raise InstallerError(
                f"script line {number}: {command} outside any"
                " section (instructions live inside sections)"
            )
        if command not in SECTION_COMMANDS and command != "Section" and (
            depth > 0
        ):
            raise InstallerError(
                f"script line {number}: attribute {command} inside a"
                " section (installer attributes live outside)"
            )
        bare = tokens[0][0] if kinds and kinds[0] == "b" else ""
        records.append((command, bare, [text for text, _ in tokens]))
    if depth != 0:
        raise InstallerError("script ends inside an open section")
    if not records:
        raise InstallerError("script has no commands")
    return records


# ---- shared loading ------------------------------------------------------


def read_text(root: Path, relative: str) -> str:
    path = root / relative
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise InstallerError(f"file is missing: {path}") from error
    try:
        return raw.decode("utf-8")
    except UnicodeError as error:
        raise InstallerError(f"file is not UTF-8: {relative}") from error


def normalize_ws(text: str) -> str:
    return " ".join(text.split())


def check_no_signing_material(relative: str, text: str) -> None:
    match = SIGNING_PATTERN.search(text)
    if match:
        raise InstallerError(
            f"{relative} carries signing material (denylisted token"
            f" {match.group(0)!r} at offset {match.start()}); the"
            " unsigned installer is the honest output -- the D221"
            " signing-keys exclusion"
        )


def check_no_corpus_mention(relative: str, text: str) -> None:
    if CORPUS_TOKEN in text:
        raise InstallerError(
            f"{relative} mentions {CORPUS_TOKEN!r}; the installer"
            " never carries or reads the corpus -- the user supplies"
            " their own original install"
        )


# ---- the script rules ----------------------------------------------------


def check_script(root: Path) -> list[tuple[str, str, list[str]]]:
    text = read_text(root, SCRIPT_RELATIVE)
    check_no_signing_material(SCRIPT_RELATIVE, text)
    check_no_corpus_mention(SCRIPT_RELATIVE, text)
    records = parse_script(text)

    def singles(command: str) -> list[str]:
        return [args[0] for name, _, args in records if name == command]

    if singles("Name") != [APP_NAME]:
        raise InstallerError(
            f"Name must be exactly {APP_NAME!r}, found"
            f" {singles('Name')!r}"
        )
    if singles("OutFile") != [SETUP_EXE]:
        raise InstallerError(
            f"OutFile must be exactly {SETUP_EXE!r} (the file the CI"
            f" job uploads), found {singles('OutFile')!r}"
        )
    if singles("Unicode") != ["true"]:
        raise InstallerError(
            f"Unicode must be true, found {singles('Unicode')!r}"
        )
    if singles("InstallDir") != [INSTALL_DIR]:
        raise InstallerError(
            f"InstallDir must be exactly {INSTALL_DIR!r} (the 64-bit"
            " program files directory, the manual's guidance for"
            f" 64-bit applications), found {singles('InstallDir')!r}"
        )
    if singles("RequestExecutionLevel") != ["admin"]:
        raise InstallerError(
            "RequestExecutionLevel must be admin (installing under"
            " $PROGRAMFILES64 needs the elevation), found"
            f" {singles('RequestExecutionLevel')!r}"
        )
    if singles("CRCCheck") != ["force"]:
        raise InstallerError(
            "CRCCheck must be force (the installer CRCs itself and"
            " the user cannot skip it), found"
            f" {singles('CRCCheck')!r}"
        )
    flow = [
        (name, bare)
        for name, bare, _ in records
        if name in ("Page", "UninstPage")
    ]
    if flow != PAGE_FLOW:
        raise InstallerError(
            f"the page flow must be exactly {PAGE_FLOW} (the minimal"
            f" wizard), found {flow}"
        )

    sections = [
        args[0] for name, _, args in records if name == "Section"
    ]
    if sections != [APP_NAME, "un.Uninstall"]:
        raise InstallerError(
            "the script must carry exactly two sections -- the"
            f" install section {APP_NAME!r} and the uninstaller"
            f" section 'un.Uninstall', found {sections!r}"
        )

    # The distinct invariants FIRST (so each rule has teeth on its
    # own), then the exact-body pins.
    # (a) the closed File set -- grammar already forbids paths and
    #     wildcards; this pins the exact members.
    sources = singles("File")
    if sorted(sources) != sorted([BINARY, README_STAGED]):
        raise InstallerError(
            "the File set must be exactly the closed two-file set"
            f" [{BINARY!r}, {README_STAGED!r}] (the engine binary +"
            f" its README), found {sources!r}"
        )
    # (b) every Delete names an installed artifact; nothing else is
    #     ever touched.
    deletable = {
        SHORTCUT,
        INSTALLED_BINARY,
        INSTALLED_README,
        UNINSTALLER,
    }
    for name, _, args in records:
        if name == "Delete" and args[0] not in deletable:
            raise InstallerError(
                f"the uninstaller deletes {args[0]!r}, which the"
                " installer never wrote (an exact-inverse uninstall"
                " only)"
            )

    # (c) the pinned section bodies (Section/SectionEnd records
    # stripped; nesting is exactly one, so the split is sound).
    body: list[tuple[str, list[str]]] = []
    current = ""
    bodies: dict[str, list[tuple[str, list[str]]]] = {}
    for name, _, args in records:
        if name == "Section":
            current = args[0]
            bodies[current] = []
            continue
        if name == "SectionEnd":
            current = ""
            continue
        if current:
            bodies[current].append((name, args))
    body = bodies[APP_NAME]
    if body != INSTALL_BODY:
        raise InstallerError(
            "the install section body is not the pinned"
            " instruction-for-instruction definition (SetOutPath"
            " $INSTDIR; exactly the two staged files; the"
            " uninstaller; the Add/Remove-Programs registration;"
            " the start-menu directory + its one shortcut):"
            f" found {body!r}"
        )
    if bodies["un.Uninstall"] != UNINSTALL_BODY:
        raise InstallerError(
            "the uninstall section body is not the pinned"
            " exact-inverse definition (every installed artifact"
            " deleted by name, the registration key removed, only"
            " empty directories removed): found"
            f" {bodies['un.Uninstall']!r}"
        )
    return records


# ---- the README contract -------------------------------------------------


def check_readme(root: Path) -> None:
    text = read_text(root, README_RELATIVE)
    check_no_signing_material(README_RELATIVE, text)
    if not text.strip():
        raise InstallerError(f"{README_RELATIVE} is empty")
    flat = normalize_ws(text)
    for sentence in README_SENTENCES:
        if sentence not in flat:
            raise InstallerError(
                f"{README_RELATIVE} is missing the required boundary"
                f" sentence: {sentence!r}"
            )
    # The corpus token may appear ONLY inside the documented
    # default layout game-data\BEDLAM (the engine's own default,
    # spelled exactly as the binary documents it).
    remainder = text.replace(CORPUS_DEFAULT_LAYOUT, "")
    if CORPUS_TOKEN in remainder:
        raise InstallerError(
            f"{README_RELATIVE} references the corpus outside the"
            f" documented default layout {CORPUS_DEFAULT_LAYOUT!r}"
            " (the README documents the user's OWN install layout,"
            " never the repository corpus)"
        )


# ---- the YAML subset reader (the family posture) --------------------------


def strip_comment(line: str) -> str:
    """Drop a full-line or trailing # comment (quote-aware)."""
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
            raise InstallerError(
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
            raise InstallerError(f"unterminated flow sequence: {value!r}")
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
                raise InstallerError(
                    f"bare sequence item with no nested value: {content!r}"
                )
            value, index = parse_node(lines, index + 1, lines[index + 1][0])
            items.append(value)
            continue
        key, separator, _ = rest.partition(":")
        if separator and " " not in key.strip() and not rest.startswith("'"):
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
        raise InstallerError("empty sequence")
    return items, index


def parse_mapping(lines: list[tuple[int, str]], index: int, indent: int):
    """A block mapping whose keys sit at exactly `indent`."""
    result: dict[str, object] = {}
    while index < len(lines):
        line_indent, content = lines[index]
        if line_indent != indent:
            if line_indent > indent:
                raise InstallerError(
                    f"unexpected deeper line where a key was due: {content!r}"
                )
            break
        if content == "-" or content.startswith("- "):
            break
        key, separator, value = content.partition(":")
        key = key.strip().strip("'\"")
        if not separator or not key:
            raise InstallerError(f"unparsable mapping line: {content!r}")
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
        raise InstallerError("empty mapping")
    return result, index


def parse_node(lines: list[tuple[int, str]], index: int, indent: int):
    content = lines[index][1]
    if content == "-" or content.startswith("- "):
        return parse_sequence(lines, index, indent)
    return parse_mapping(lines, index, indent)


def parse_document(text: str) -> dict:
    lines = scan_lines(text)
    if not lines:
        raise InstallerError("file has no content lines")
    document, consumed = parse_node(lines, 0, lines[0][0])
    if consumed != len(lines):
        leftover = lines[consumed][1]
        raise InstallerError(f"unparsed trailing content: {leftover!r}")
    if not isinstance(document, dict):
        raise InstallerError("top level is not a mapping")
    return document


def load_yaml(root: Path, relative: str) -> dict:
    return parse_document(read_text(root, relative))


# ---- the CI build join ----------------------------------------------------


def job_text(workflow_text: str, job: str) -> str:
    """The raw workflow lines belonging to one job (denylist scope)."""
    pattern = re.compile(rf"^  {re.escape(job)}:\s*$")
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


def check_ci_job(root: Path) -> str:
    workflow_text = read_text(root, WORKFLOW_RELATIVE)
    workflow = load_yaml(root, WORKFLOW_RELATIVE)
    trigger = workflow.get("on")
    if not isinstance(trigger, dict) or "push" not in trigger:
        raise InstallerError(
            "the workflow must carry a top-level push trigger (the"
            " installer is built per push)"
        )
    jobs = workflow.get("jobs")
    if not isinstance(jobs, dict) or not jobs:
        raise InstallerError("workflow defines no jobs")
    job = jobs.get(CI_JOB)
    if not isinstance(job, dict):
        raise InstallerError(
            f"the workflow has no `{CI_JOB}` job (the Windows"
            " installer build definition)"
        )
    if job.get("runs-on") != "windows-latest":
        raise InstallerError(
            f"the {CI_JOB} job must run on windows-latest (the"
            " Windows leg that builds the engine binary), found"
            f" {job.get('runs-on')!r}"
        )
    steps = job.get("steps")
    if not isinstance(steps, list) or not steps:
        raise InstallerError(f"the {CI_JOB} job carries no steps")

    uses = [
        step.get("uses")
        for step in steps
        if isinstance(step, dict) and isinstance(step.get("uses"), str)
    ]
    if "actions/checkout@v4" not in uses:
        raise InstallerError(
            f"the {CI_JOB} job never checks out the repository"
            " (actions/checkout@v4)"
        )
    if "dtolnay/rust-toolchain@stable" not in uses:
        raise InstallerError(
            f"the {CI_JOB} job never installs the Rust toolchain"
            " (dtolnay/rust-toolchain@stable, the build matrix's own"
            " toolchain)"
        )
    runs = [
        step["run"]
        for step in steps
        if isinstance(step, dict) and isinstance(step.get("run"), str)
    ]
    build = [run for run in runs if "cargo build --release" in run]
    if not build:
        raise InstallerError(
            f"the {CI_JOB} job never builds the release engine"
            " (cargo build --release)"
        )
    for run in build:
        if "--locked" not in run:
            raise InstallerError(
                "the cargo build is not --locked (the installer must"
                " package the reproducible crate set from the"
                " committed Cargo.lock)"
            )
        if "-p bedlam-shell" not in run:
            raise InstallerError(
                "the cargo build must target the engine binary"
                " -p bedlam-shell"
            )
        if "--offline" in run:
            raise InstallerError(
                "the cargo build is --offline but no vendored crate"
                " set is committed -- such a job could never build"
                " (crates.io is fetched on the runner)"
            )
    if not any("choco install nsis" in run for run in runs):
        raise InstallerError(
            f"the {CI_JOB} job never installs NSIS via chocolatey"
            " (choco install nsis -- the makensis compiler)"
        )
    staged_from = f"target\\release\\{BINARY}"
    staged_to = f"packaging\\{BINARY}"
    if not any(
        "Copy-Item" in run and staged_from in run and staged_to in run
        for run in runs
    ):
        raise InstallerError(
            f"the {CI_JOB} job never stages {staged_from} as"
            f" {staged_to} (the script's File sources are staged bare"
            " names next to the script)"
        )
    script_name = Path(SCRIPT_RELATIVE).name
    makensis = [
        step
        for step in steps
        if isinstance(step, dict)
        and isinstance(step.get("run"), str)
        and "makensis.exe" in step["run"]
    ]
    if not makensis:
        raise InstallerError(
            f"the {CI_JOB} job never runs makensis (the compiler"
            " that turns THIS script into the installer)"
        )
    for step in makensis:
        if step.get("working-directory") != "packaging":
            raise InstallerError(
                "the makensis step must run with working-directory:"
                " packaging so every relative path in the script"
                " resolves to packaging\\ under either candidate"
                " rule (script directory or process working"
                " directory)"
            )
        if script_name not in step["run"].split():
            raise InstallerError(
                f"the makensis step does not compile THIS script"
                f" ({script_name} as a whole argument word)"
            )
    uploads = [
        step
        for step in steps
        if isinstance(step, dict)
        and step.get("uses") == ARTIFACT_ACTION
        and isinstance(step.get("with"), dict)
    ]
    if not uploads:
        raise InstallerError(
            f"the {CI_JOB} job never uploads the installer with"
            f" {ARTIFACT_ACTION}"
        )
    for step in uploads:
        with_block = step["with"]
        if with_block.get("name") != ARTIFACT_NAME:
            raise InstallerError(
                f"the installer artifact name must be exactly"
                f" {ARTIFACT_NAME!r}, found {with_block.get('name')!r}"
            )
        path = with_block.get("path")
        if path != f"packaging/{SETUP_EXE}":
            raise InstallerError(
                f"the installer upload path must be exactly"
                f" packaging/{SETUP_EXE} (the script's OutFile, built"
                f" inside packaging/), found {path!r}"
            )
        if with_block.get("if-no-files-found") != "error":
            raise InstallerError(
                "the installer upload must set if-no-files-found:"
                " error so a missing installer fails the build"
            )
        if "retention-days" not in with_block:
            raise InstallerError(
                "the installer upload must carry a bounded"
                " retention-days (per-push artifacts do not"
                " accumulate)"
            )
    # Scoped denylist: the job's own text carries no signing
    # vocabulary and never mentions the corpus directory.
    scoped = job_text(workflow_text, CI_JOB)
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
        check_script(root)
        check_readme(root)
        artifact = check_ci_job(root)
    except InstallerError as error:
        print(f"p7-windows-installer: FAIL: {error}", file=sys.stderr)
        return 1
    print("p7-windows-installer: OK")
    print(f"  script: {SCRIPT_RELATIVE} parses under the closed grammar")
    print(
        f"  installer: {APP_NAME!r} -> {SETUP_EXE} (Unicode,"
        f" {INSTALL_DIR}, admin, CRCCheck force; pages"
        " directory+instfiles)"
    )
    print(
        f"  file set: exactly {BINARY} + {README_STAGED} (staged bare"
        " names; no paths, no wildcards)"
    )
    print(
        "  start menu: one shortcut onto the installed engine with"
        " $INSTDIR as its working directory ($OUTDIR rule; the"
        " engine's documented default lookup root sits inside the"
        " install folder)"
    )
    print(
        "  uninstall: the exact inverse (installed artifacts by name,"
        " the ARP key removed, only empty directories removed)"
    )
    print(
        f"  ci join: job {CI_JOB!r} on windows-latest builds the"
        f" engine --locked, stages the binary, compiles THIS script"
        f" with makensis in packaging/, uploads {artifact}"
        f" ({ARTIFACT_ACTION}, if-no-files-found: error)"
    )
    print(
        f"  signing material: none ({len(SIGNING_TOKENS)} denylisted"
        " tokens absent across script + README + the"
        f" {CI_JOB} job, comments included)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
