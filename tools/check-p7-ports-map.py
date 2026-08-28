#!/usr/bin/python3
"""Validate the committed P7 ports deliverable-map contract (the P7 opener).

Fail-closed checker for the p7-ports-scaffold required gate
(docs/required-gates.toml, D221 -- the D175 scaffold pattern: the
machine-checkable contract lands BEFORE any packaging work it grades).
PLAN §6 (P7) is the whole phase text; this checker enforces its
operationalization in docs/P7-PORTS.md:

  1. the doc carries the plan's boundary sentences verbatim
     (whitespace-normalized matching) -- the three-OS surface, the
     external-conditions non-blocking rule, per-push artifacts, the
     CDDA user-supply/never-redistribute boundary, the SteamDeck
     stretch default, and this unit's own scaffold bounds;
  2. the deliverable registry (schema p7-ports-map-v1, a fenced TOML
     block inside the doc) is discipline-clean: unique whitespace-free
     ids, closed kind set, a plan anchor on every row, the ENGINEERING
     set exactly the seven plan-derived deliverables and the
     EXTERNAL-CONDITIONAL set exactly the three recorded exclusions;
  3. evidence discipline (the P6 catalog R2 analogue): an engineering
     deliverable is landed exactly when its proving gate is named; a
     pending row carries none; an external-conditional row never
     carries status or gate (an exclusion cannot be landed) and must
     record its exclusion note;
  4. the gate join: every named gate resolves to a [[gate]] id in the
     manifest AND sits in the P7 phase required_gates list;
  5. cross-artifact safety with docs/required-gates.toml: a non-empty
     P7 required_gates list starts with p7-ports-scaffold, that gate
     block is defined, runs this checker, and tracks the doc + both
     tools + the manifest; P7 status green requires every engineering
     deliverable landed (no premature phase flip).

It reads ONLY committed docs -- no network, no game-data read, no
writes, PATH-free under the validator's bwrap.
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path

DOC_RELATIVE = "docs/P7-PORTS.md"
MANIFEST_RELATIVE = "docs/required-gates.toml"
GATE_ID = "p7-ports-scaffold"
CHECKER_RELATIVE = "tools/check-p7-ports-map.py"
SUITE_RELATIVE = "tools/test-p7-ports-map.py"
REGISTRY_SCHEMA = "p7-ports-map-v1"

KINDS = ("engineering", "external-conditional")
STATUSES = ("pending", "landed")
# R3: the split is the plan's own scope; additions/re-scope are a
# DECISIONS entry + a checker update, never silent.
ENGINEERING_REQUIRED = frozenset(
    {
        "ci-artifacts-per-push",
        "cdda-user-supply",
        "flatpak-manifest",
        "linux-native",
        "macos-universal2-ci",
        "steamdeck-default",
        "windows-installer",
    }
)
EXTERNAL_REQUIRED = frozenset(
    {"macos-runner-availability", "publication-stores", "signing-keys"}
)
ROW_KEYS = frozenset({"id", "kind", "plan_anchor", "status", "gate", "note"})
REQUIRED_SECTIONS = (
    "## 1. The P7 scope map (VERBATIM from PLAN §6, P7)",
    "## 2. The deliverable map: ENGINEERING vs EXTERNAL-CONDITIONAL",
    "## 3. The deliverable registry (`p7-ports-map-v1`)",
    "## 4. The CDDA user-supply + local-cache contract",
    "## 5. The SteamDeck stretch default",
    "## 6. Gate wiring (the first P7 required gate) + the gate shape the phase closes on",
    "## 7. P7 acceptance surface (pointer, not re-statement)",
)
REQUIRED_SENTENCES = (
    # the plan's own surface sentence (PLAN §6 P7 verbatim)
    "Linux native + Flatpak; Windows installer; macOS universal2 through automated CI.",
    # the exclusion rule that grades only the engineering
    "Runner, signing, and publication availability are external conditions and do not"
    " block engineering completion.",
    "CI artifacts per push",
    # the CDDA boundary
    "user-supplied original tracks",
    "optional local lossy cache generated on first run",
    "never redistributed",
    # the SteamDeck default
    "SteamDeck defaults stretch",
    # this unit's own bounds
    "no engine change and no packaging build lands in this unit",
    # the split's teeth + the phase-close posture + the exclusion analogy
    "A deliverable is landed exactly when its proving gate is named",
    "P7 status stays pending until every engineering deliverable is landed",
    "recorded as exclusions exactly like the P4 live-capture diagnostics",
    # the schema id this checker reads
    "p7-ports-map-v1",
)
TOML_BLOCK = re.compile(r"```toml\r?\n(.*?)\r?\n```", re.DOTALL)


class ContractError(Exception):
    pass


def load_doc(root: Path) -> str:
    path = root / DOC_RELATIVE
    try:
        return path.read_bytes().decode("utf-8")
    except OSError as error:
        raise ContractError(f"contract doc is missing: {path}") from error
    except UnicodeError as error:
        raise ContractError(f"contract doc is not UTF-8: {error}") from error


def normalize_ws(text: str) -> str:
    """Collapse markdown line-wrapping so wrapped sentences still match."""
    return " ".join(text.split())


def check_sections_and_sentences(text: str) -> None:
    flat = normalize_ws(text)
    for header in REQUIRED_SECTIONS:
        if normalize_ws(header) not in flat:
            raise ContractError(f"contract doc is missing section: {header!r}")
    for sentence in REQUIRED_SENTENCES:
        if normalize_ws(sentence) not in flat:
            raise ContractError(
                f"contract doc is missing the required rule sentence: {sentence!r}"
            )


def extract_registry(text: str) -> str:
    blocks = TOML_BLOCK.findall(text)
    registries = [
        block
        for block in blocks
        if block.lstrip().startswith(f'schema = "{REGISTRY_SCHEMA}"')
    ]
    if not registries:
        raise ContractError(
            f"contract doc has no fenced toml registry with"
            f' schema = "{REGISTRY_SCHEMA}"'
        )
    if len(registries) > 1:
        raise ContractError(
            f"contract doc carries {len(registries)} {REGISTRY_SCHEMA} blocks (want 1)"
        )
    return registries[0]


def _require_str(row: dict, key: str, identifier: str) -> str:
    value = row.get(key, "")
    if not isinstance(value, str) or not value:
        raise ContractError(
            f"deliverable {identifier} {key} must be a non-empty string,"
            f" found {value!r}"
        )
    return value


def load_rows(text: str) -> list[dict]:
    registry = extract_registry(text)
    try:
        value = tomllib.loads(registry)
    except tomllib.TOMLDecodeError as error:
        raise ContractError(f"deliverable registry does not parse: {error}") from error
    if value.get("schema") != REGISTRY_SCHEMA:
        raise ContractError(
            f"deliverable registry schema must be {REGISTRY_SCHEMA},"
            f" found {value.get('schema')!r}"
        )
    rows = value.get("deliverable", [])
    if not isinstance(rows, list) or not rows:
        raise ContractError("deliverable registry has no [[deliverable]] rows")
    seen: set[str] = set()
    for index, row in enumerate(rows):
        if not isinstance(row, dict):
            raise ContractError(f"deliverable registry row {index} is not a table")
        unknown = set(row) - ROW_KEYS
        if unknown:
            raise ContractError(
                f"deliverable registry row {index} has unknown keys: {sorted(unknown)}"
            )
        identifier = _require_str(row, "id", str(index))
        if identifier.split() != [identifier]:
            raise ContractError(
                f"deliverable id must be whitespace-free, found {identifier!r}"
            )
        if identifier in seen:
            raise ContractError(f"duplicate deliverable id: {identifier}")
        seen.add(identifier)
        _require_str(row, "plan_anchor", identifier)
        kind = row.get("kind")
        if kind not in KINDS:
            raise ContractError(
                f"deliverable {identifier} kind must be one of {list(KINDS)},"
                f" found {kind!r}"
            )
        note = row.get("note", "")
        if not isinstance(note, str):
            raise ContractError(
                f"deliverable {identifier} note must be a string, found {note!r}"
            )
        if kind == "external-conditional":
            # R8: exclusions never carry status/gate and always record why.
            for forbidden in ("status", "gate"):
                if forbidden in row:
                    raise ContractError(
                        f"deliverable {identifier} is {kind} and must not carry"
                        f" {forbidden} (an exclusion cannot be landed by"
                        " engineering work)"
                    )
            if not note:
                raise ContractError(
                    f"deliverable {identifier} is {kind} and must carry the"
                    " recorded exclusion note"
                )
        else:
            # R2: engineering rows are pending-or-landed, gate iff landed.
            status = row.get("status")
            if status not in STATUSES:
                raise ContractError(
                    f"deliverable {identifier} status must be one of"
                    f" {list(STATUSES)}, found {status!r}"
                )
            gate = row.get("gate", "")
            if not isinstance(gate, str):
                raise ContractError(
                    f"deliverable {identifier} gate must be a string,"
                    f" found {gate!r}"
                )
            if status == "landed" and not gate:
                raise ContractError(
                    f"deliverable {identifier} is landed but names no proving"
                    " gate (an engineering deliverable is landed exactly when"
                    " its proving gate is named)"
                )
            if status == "pending" and gate:
                raise ContractError(
                    f"deliverable {identifier} is pending but carries gate"
                    f" {gate!r} (a pending row carries no gate)"
                )
    return rows


def check_coverage(rows: list[dict]) -> None:
    engineering = {row["id"] for row in rows if row["kind"] == "engineering"}
    external = {
        row["id"] for row in rows if row["kind"] == "external-conditional"
    }
    missing = sorted(ENGINEERING_REQUIRED - engineering)
    if missing:
        raise ContractError(
            f"registry is missing the engineering deliverables: {missing}"
        )
    extra = sorted(engineering - ENGINEERING_REQUIRED)
    if extra:
        raise ContractError(
            f"registry carries engineering deliverables outside the decided"
            f" P7 scope (re-scope = DECISIONS entry + checker update): {extra}"
        )
    missing = sorted(EXTERNAL_REQUIRED - external)
    if missing:
        raise ContractError(
            f"registry is missing the recorded exclusions: {missing}"
        )
    extra = sorted(external - EXTERNAL_REQUIRED)
    if extra:
        raise ContractError(
            f"registry carries external-conditional rows outside the decided"
            f" exclusion set: {extra}"
        )


def load_manifest(root: Path) -> dict:
    path = root / MANIFEST_RELATIVE
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ContractError(f"required-gates manifest is missing: {path}") from error
    try:
        return tomllib.loads(raw.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise ContractError(f"required-gates manifest does not parse: {error}") from error


def check_manifest(root: Path, rows: list[dict]) -> None:
    manifest = load_manifest(root)
    phases = {
        phase.get("id"): phase
        for phase in manifest.get("phase", [])
        if isinstance(phase, dict)
    }
    p7 = phases.get("P7")
    if p7 is None:
        raise ContractError("required-gates manifest has no P7 phase")
    required = p7.get("required_gates", [])
    if not isinstance(required, list) or not all(
        isinstance(gate, str) for gate in required
    ):
        raise ContractError("P7 required_gates must be an array of strings")
    gate_blocks = {
        gate.get("id"): gate
        for gate in manifest.get("gate", [])
        if isinstance(gate, dict)
    }
    # R5: the scaffold precedes every other P7 gate and is wired to this
    # checker + the doc.
    if required:
        if required[0] != GATE_ID:
            raise ContractError(
                f"P7 required_gates must start with {GATE_ID} (the contract"
                f" lands before the packaging work it grades), found"
                f" {required[0]!r}"
            )
        scaffold = gate_blocks.get(GATE_ID)
        if scaffold is None:
            raise ContractError(
                f"P7 required_gates names {GATE_ID} but no [[gate]] with"
                " that id is defined"
            )
        commands = scaffold.get("commands", [])
        if not isinstance(commands, list):
            raise ContractError(f"gate {GATE_ID} commands must be an array")
        if not any(
            isinstance(command, list) and CHECKER_RELATIVE in command
            for command in commands
        ):
            raise ContractError(
                f"gate {GATE_ID} commands do not run {CHECKER_RELATIVE}"
            )
        tracked = scaffold.get("tracked_paths", [])
        for needed in (DOC_RELATIVE, CHECKER_RELATIVE, SUITE_RELATIVE, MANIFEST_RELATIVE):
            if not isinstance(tracked, list) or needed not in tracked:
                raise ContractError(
                    f"gate {GATE_ID} tracked_paths do not include {needed}"
                )
    # R4: every named proving gate exists AND sits in the phase list.
    for row in rows:
        if row["kind"] != "engineering" or row.get("status") != "landed":
            continue
        gate = row["gate"]
        if gate not in gate_blocks:
            raise ContractError(
                f"deliverable {row['id']} names proving gate {gate!r} but no"
                " [[gate]] with that id is defined"
            )
        if gate not in required:
            raise ContractError(
                f"deliverable {row['id']} names proving gate {gate!r} which"
                " is not in the P7 required_gates list (a landed deliverable"
                " is proved by a gate the phase actually runs)"
            )
    # R6: no premature phase flip with unfinished engineering.
    status = p7.get("status")
    if status == "green":
        unfinished = sorted(
            row["id"]
            for row in rows
            if row["kind"] == "engineering" and row.get("status") != "landed"
        )
        if unfinished:
            raise ContractError(
                f"manifest P7 status is green but {len(unfinished)} engineering"
                f" deliverables are not landed (e.g. {unfinished[0]})"
            )


def main() -> int:
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=default_root)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    try:
        text = load_doc(root)
        check_sections_and_sentences(text)
        rows = load_rows(text)
        check_coverage(rows)
        check_manifest(root, rows)
    except ContractError as error:
        print(f"p7-ports-map: FAIL: {error}", file=sys.stderr)
        return 1
    engineering = [row for row in rows if row["kind"] == "engineering"]
    external = [row for row in rows if row["kind"] == "external-conditional"]
    landed = [row for row in engineering if row.get("status") == "landed"]
    print("p7-ports-map: OK")
    print(
        f"  deliverables: {len(engineering)} engineering"
        f" ({len(landed)} landed, {len(engineering) - len(landed)} pending)"
        f" + {len(external)} recorded exclusions"
    )
    if landed:
        summary = ", ".join(
            f"{row['id']} (gate {row['gate']})"
            for row in sorted(landed, key=lambda row: row["id"])
        )
        print(f"  landed: {summary}")
    for row in sorted(external, key=lambda row: row["id"]):
        print(f"  exclusion: {row['id']}")
    print(
        "  rules: boundary sentences + registry discipline + gate joins +"
        " manifest wiring verified"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
