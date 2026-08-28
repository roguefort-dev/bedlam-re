#!/usr/bin/python3
"""Validate the P6 original-behavior catalog and its cross-artifact joins.

Fail-closed checker for the p6-modernization-scaffold required gate
(docs/required-gates.toml, D200). It validates:

  1. the committed catalog (docs/P6-BEHAVIOR-CATALOG.toml) is schema-clean
     and internally consistent, with the PLAN §6 bug-triage rubric enforced
     as code: a closed entry's disposition must be the mechanical terminal
     disposition of its class (crash/data-loss -> fix everywhere;
     gameplay-coupled -> classic preserves / modern fixes; cosmetic ->
     fix in modern), closure requires regression evidence, and only
     closed-preserve-classic entries carry a (unique) purist toggle;
  2. mission grounding + the P5 feed join: every entry mission id exists in
     docs/P5-MISSION-LEDGER.toml, and every ledger catalog_ref resolves to
     a catalog entry id (bidirectional);
  3. cross-artifact safety with docs/required-gates.toml: a non-empty P6
     required_gates list must start with p6-modernization-scaffold (and
     that gate must be defined), and P6 status green requires zero open
     entries.

Layering (one source of truth per fact): the ledger's own schema/corpus
binding is tools/check-p5-zone-ledger.py's job; this checker treats the
ledger as the mission identity source. It reads ONLY committed docs —
no game-data read, no writes, PATH-free under the validator's bwrap.
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path

CATALOG_SCHEMA = "p6-behavior-catalog-v1"
CLASSES = ("crash-data-loss", "gameplay-coupled", "cosmetic")
OBSERVED = ("original", "divergence")
DISPOSITIONS = ("open", "closed-fix-everywhere", "closed-fix-modern", "closed-preserve-classic")
# PLAN §6 rubric as code: the ONLY terminal disposition each class may close to.
CLASS_TERMINAL = {
    "crash-data-loss": "closed-fix-everywhere",
    "cosmetic": "closed-fix-modern",
    "gameplay-coupled": "closed-preserve-classic",
}
ENTRY_KEYS = {
    "id",
    "title",
    "class",
    "observed",
    "repro",
    "missions",
    "disposition",
    "evidence",
    "purist_toggle",
    "provenance",
}
SCAFFOLD_GATE = "p6-modernization-scaffold"


class CatalogError(Exception):
    pass


def _require_str(row: dict, key: str, identifier: str, *, allow_empty: bool = False) -> str:
    value = row.get(key, "")
    if not isinstance(value, str) or (not allow_empty and not value):
        raise CatalogError(
            f"catalog entry {identifier} {key} must be a non-empty string, "
            f"found {value!r}"
        )
    return value


def load_ledger(root: Path) -> dict[str, list[str]]:
    """Mission identity + catalog_refs from the P5 ledger (light parse).

    Deep ledger validation (schema, corpus binding) belongs to
    tools/check-p5-zone-ledger.py; here rows only need usable identity.
    """
    ledger_path = root / "docs" / "P5-MISSION-LEDGER.toml"
    try:
        raw = ledger_path.read_bytes()
    except OSError as error:
        raise CatalogError(f"P5 mission ledger is missing: {ledger_path}") from error
    try:
        value = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise CatalogError(f"P5 mission ledger does not parse: {error}") from error
    rows = value.get("mission")
    if not isinstance(rows, list) or not rows:
        raise CatalogError("P5 mission ledger has no [[mission]] rows")
    refs_by_id: dict[str, list[str]] = {}
    for index, row in enumerate(rows):
        if not isinstance(row, dict):
            raise CatalogError(f"P5 mission ledger row {index} is not a table")
        identifier = row.get("id")
        if not isinstance(identifier, str) or not identifier:
            raise CatalogError(
                f"P5 mission ledger row {index} id must be a non-empty string"
            )
        refs = row.get("catalog_refs")
        if not isinstance(refs, list) or not all(isinstance(r, str) for r in refs):
            raise CatalogError(
                f"P5 mission ledger row {identifier} catalog_refs must be an "
                "array of strings"
            )
        refs_by_id[identifier] = refs
    return refs_by_id


def load_catalog(root: Path) -> dict[str, dict]:
    catalog_path = root / "docs" / "P6-BEHAVIOR-CATALOG.toml"
    try:
        raw = catalog_path.read_bytes()
    except OSError as error:
        raise CatalogError(f"catalog is missing: {catalog_path}") from error
    try:
        value = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise CatalogError(f"catalog does not parse: {error}") from error
    if value.get("schema") != CATALOG_SCHEMA:
        raise CatalogError(
            f"catalog schema must be {CATALOG_SCHEMA}, found {value.get('schema')!r}"
        )
    entries = value.get("entry", [])
    if not isinstance(entries, list):
        raise CatalogError("catalog entry must be an array of tables")
    by_id: dict[str, dict] = {}
    toggles: dict[str, str] = {}
    for index, row in enumerate(entries):
        if not isinstance(row, dict):
            raise CatalogError(f"catalog entry {index} is not a table")
        unknown = set(row) - ENTRY_KEYS
        if unknown:
            raise CatalogError(
                f"catalog entry {index} has unknown keys: {sorted(unknown)}"
            )
        identifier = _require_str(row, "id", str(index))
        if identifier.split() != [identifier]:
            raise CatalogError(
                f"catalog entry id must be whitespace-free, found {identifier!r}"
            )
        if identifier in by_id:
            raise CatalogError(f"catalog duplicate entry id: {identifier}")
        _require_str(row, "title", identifier)
        _require_str(row, "repro", identifier)
        _require_str(row, "provenance", identifier)
        entry_class = row.get("class")
        if entry_class not in CLASSES:
            raise CatalogError(
                f"catalog entry {identifier} class must be one of "
                f"{list(CLASSES)}, found {entry_class!r}"
            )
        observed = row.get("observed")
        if observed not in OBSERVED:
            raise CatalogError(
                f"catalog entry {identifier} observed must be one of "
                f"{list(OBSERVED)}, found {observed!r}"
            )
        missions = row.get("missions")
        if (
            not isinstance(missions, list)
            or not missions
            or not all(isinstance(m, str) and m for m in missions)
        ):
            raise CatalogError(
                f"catalog entry {identifier} missions must be a non-empty "
                "array of mission id strings"
            )
        if len(set(missions)) != len(missions):
            raise CatalogError(
                f"catalog entry {identifier} missions contains duplicates"
            )
        disposition = row.get("disposition")
        if disposition not in DISPOSITIONS:
            raise CatalogError(
                f"catalog entry {identifier} disposition must be one of "
                f"{list(DISPOSITIONS)}, found {disposition!r}"
            )
        # R2 evidence discipline (also tolerates the key being absent).
        evidence = row.get("evidence", "")
        if not isinstance(evidence, str):
            raise CatalogError(
                f"catalog entry {identifier} evidence must be a string, "
                f"found {evidence!r}"
            )
        closed = disposition != "open"
        if closed and not evidence:
            raise CatalogError(
                f"catalog entry {identifier} is closed ({disposition}) but "
                "carries no regression evidence"
            )
        if not closed and evidence:
            raise CatalogError(
                f"catalog entry {identifier} is open but carries evidence "
                f"{evidence!r} (close the entry or move the finding to repro)"
            )
        # R1 rubric-as-code: the terminal disposition matches the class.
        if closed and disposition != CLASS_TERMINAL[entry_class]:
            raise CatalogError(
                f"catalog entry {identifier} class {entry_class} may only "
                f"close to {CLASS_TERMINAL[entry_class]} (the PLAN §6 "
                f"rubric), found {disposition}"
            )
        # R3 toggle discipline.
        toggle = row.get("purist_toggle")
        if disposition == "closed-preserve-classic":
            if not isinstance(toggle, str) or not toggle:
                raise CatalogError(
                    f"catalog entry {identifier} is {disposition} but carries "
                    "no purist_toggle (classic preservation needs the "
                    "ModeConfig toggle id)"
                )
            if toggle.split() != [toggle]:
                raise CatalogError(
                    f"catalog entry {identifier} purist_toggle must be "
                    f"whitespace-free, found {toggle!r}"
                )
            if toggle in toggles:
                raise CatalogError(
                    f"catalog entries {toggles[toggle]} and {identifier} "
                    f"share purist_toggle {toggle!r}"
                )
            toggles[toggle] = identifier
        elif isinstance(toggle, str) and toggle:
            raise CatalogError(
                f"catalog entry {identifier} carries purist_toggle {toggle!r} "
                f"but its disposition is {disposition} (only "
                "closed-preserve-classic entries carry a toggle)"
            )
        by_id[identifier] = row
    return by_id


def check_grounding_and_feed(
    by_id: dict[str, dict], ledger_refs: dict[str, list[str]]
) -> None:
    # R4: every entry mission is a ledger mission id.
    for identifier, row in by_id.items():
        unknown = sorted(set(row["missions"]) - set(ledger_refs))
        if unknown:
            raise CatalogError(
                f"catalog entry {identifier} missions are not P5 ledger "
                f"mission ids: {unknown}"
            )
    # R5: every ledger catalog_ref resolves to a catalog entry id.
    for mission_id, refs in sorted(ledger_refs.items()):
        dangling = sorted(set(refs) - set(by_id))
        if dangling:
            raise CatalogError(
                f"P5 ledger mission {mission_id} catalog_refs do not resolve "
                f"to catalog entries: {dangling}"
            )


def check_manifest_consistency(root: Path, by_id: dict[str, dict]) -> None:
    manifest_path = root / "docs" / "required-gates.toml"
    try:
        raw = manifest_path.read_bytes()
    except OSError as error:
        raise CatalogError(
            f"required-gates manifest is missing: {manifest_path}"
        ) from error
    try:
        manifest = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise CatalogError(f"required-gates manifest does not parse: {error}") from error
    phases = {
        phase.get("id"): phase
        for phase in manifest.get("phase", [])
        if isinstance(phase, dict)
    }
    p6 = phases.get("P6")
    if p6 is None:
        raise CatalogError("required-gates manifest has no P6 phase")
    required = p6.get("required_gates", [])
    if not isinstance(required, list):
        raise CatalogError("P6 required_gates must be an array")
    # R6: the scaffold precedes every other P6 gate.
    if required:
        for gate_id in required:
            if not isinstance(gate_id, str):
                raise CatalogError("P6 required_gates entries must be strings")
        if required[0] != SCAFFOLD_GATE:
            raise CatalogError(
                f"P6 required_gates must start with {SCAFFOLD_GATE} (the "
                f"contract lands before the behavior it grades), found "
                f"{required[0]!r}"
            )
        gate_ids = {
            gate.get("id")
            for gate in manifest.get("gate", [])
            if isinstance(gate, dict)
        }
        if SCAFFOLD_GATE not in gate_ids:
            raise CatalogError(
                f"P6 required_gates names {SCAFFOLD_GATE} but no [[gate]] "
                "with that id is defined"
            )
    # R7: no premature phase flip with untriaged entries.
    status = p6.get("status")
    if status == "green":
        open_entries = sorted(
            identifier for identifier, row in by_id.items() if row["disposition"] == "open"
        )
        if open_entries:
            raise CatalogError(
                f"manifest P6 status is green but {len(open_entries)} catalog "
                f"entries are still open (e.g. {open_entries[0]})"
            )


def main() -> int:
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=default_root)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    try:
        ledger_refs = load_ledger(root)
        by_id = load_catalog(root)
        check_grounding_and_feed(by_id, ledger_refs)
        check_manifest_consistency(root, by_id)
    except CatalogError as error:
        print(f"p6-behavior-catalog: FAIL: {error}", file=sys.stderr)
        return 1
    closed = {
        "closed-fix-everywhere": 0,
        "closed-fix-modern": 0,
        "closed-preserve-classic": 0,
    }
    open_count = 0
    for row in by_id.values():
        if row["disposition"] == "open":
            open_count += 1
        else:
            closed[row["disposition"]] += 1
    ref_count = sum(len(refs) for refs in ledger_refs.values())
    print("p6-behavior-catalog: OK")
    print(
        f"  entries: {len(by_id)} (open {open_count}; closed "
        + ", ".join(f"{k} {v}" for k, v in closed.items())
        + ")"
    )
    print(f"  missions: {len(ledger_refs)} ledger ids; {ref_count} ledger catalog_refs resolve")
    print(
        "  rubric: crash-data-loss->fix-everywhere, "
        "gameplay-coupled->preserve-classic, cosmetic->fix-modern"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
