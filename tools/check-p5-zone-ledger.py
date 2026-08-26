#!/usr/bin/python3
"""Validate the P5 per-zone parity ledger against the read-only corpus.

Fail-closed checker for the p5-zone-gate-scaffold required gate
(docs/required-gates.toml). It validates:

  1. the committed ledger (docs/P5-MISSION-LEDGER.toml) is schema-clean,
     complete, and internally consistent;
  2. its mission set equals the corpus set enumerated READ-ONLY from
     game-data/BEDLAM/EDITOR/ZONE*/MISSION*.TOT (exactly the 37 shipped
     missions in the pinned zone shape A:1, B-F:7 each, G:1);
  3. cross-artifact safety with docs/required-gates.toml: a per-zone
     completion gate p5-zone-{a..g} may only exist once its zone is fully
     green in the ledger, and the P5 phase status may only be green once
     every mission is green.

game-data is never git-tracked: no corpus path may appear in the gate's
tracked_paths or corpus policy. This checker READS the corpus at runtime
(read-only) exactly like MANIFEST.sha256 verification does. It never writes
anything and runs PATH-free under the validator's bwrap containment.
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path

LEDGER_SCHEMA = "p5-mission-ledger-v1"
DISPOSITIONS = ("pending", "green")
# The VERIFIED shipped census (docs/P5-ZONE-GATES.md §2; FORMATS-MISSION §0):
# ZONEA MISSION1; ZONEB..ZONEF MISSION1..7 each; ZONEG MISSION1.
PINNED_ZONE_SHAPE = {"A": 1, "B": 7, "C": 7, "D": 7, "E": 7, "F": 7, "G": 1}
PINNED_TOTAL = sum(PINNED_ZONE_SHAPE.values())  # 37
ZONE_LETTERS = "".join(sorted(PINNED_ZONE_SHAPE))
MISSION_FILE = re.compile(r"^MISSION([0-9]+)\.TOT$")
ZONE_GATE_ID = re.compile(rf"^p5-zone-([{ZONE_LETTERS.lower()}])$")


class LedgerError(Exception):
    pass


def enumerate_corpus(root: Path) -> dict[str, list[int]]:
    """Enumerate shipped missions read-only from the game-data corpus."""
    editor = root / "game-data" / "BEDLAM" / "EDITOR"
    if not editor.is_dir():
        raise LedgerError(
            f"corpus is unavailable for read-only enumeration: {editor}"
        )
    found: dict[str, list[int]] = {}
    for zone_dir in sorted(editor.iterdir()):
        if not zone_dir.is_dir() or not re.fullmatch(r"ZONE[A-Z]", zone_dir.name):
            continue
        letter = zone_dir.name[4]
        missions: list[int] = []
        for entry in sorted(zone_dir.iterdir()):
            match = MISSION_FILE.fullmatch(entry.name)
            if match:
                if not entry.is_file():
                    raise LedgerError(
                        f"corpus mission path is not a regular file: {entry}"
                    )
                missions.append(int(match.group(1)))
        found[letter] = sorted(missions)
    if not found:
        raise LedgerError(
            f"corpus enumeration found no ZONE*/MISSION*.TOT under {editor}"
        )
    return found


def check_zone_shape(found: dict[str, list[int]]) -> None:
    expected_letters = set(PINNED_ZONE_SHAPE)
    actual_letters = set(found)
    if actual_letters != expected_letters:
        raise LedgerError(
            "corpus zone set drifted from the pinned census: "
            f"expected {sorted(expected_letters)}, found {sorted(actual_letters)}"
        )
    for letter in sorted(expected_letters):
        expected = list(range(1, PINNED_ZONE_SHAPE[letter] + 1))
        if found[letter] != expected:
            raise LedgerError(
                f"corpus ZONE{letter} missions drifted from the pinned census: "
                f"expected {expected}, found {found[letter]}"
            )
    total = sum(len(v) for v in found.values())
    if total != PINNED_TOTAL:
        raise LedgerError(
            f"corpus mission total drifted: expected {PINNED_TOTAL}, found {total}"
        )


def load_ledger(root: Path) -> tuple[dict, dict[str, dict]]:
    ledger_path = root / "docs" / "P5-MISSION-LEDGER.toml"
    try:
        raw = ledger_path.read_bytes()
    except OSError as error:
        raise LedgerError(f"ledger is missing: {ledger_path}") from error
    try:
        value = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise LedgerError(f"ledger does not parse: {error}") from error
    if value.get("schema") != LEDGER_SCHEMA:
        raise LedgerError(
            f"ledger schema must be {LEDGER_SCHEMA}, found {value.get('schema')!r}"
        )
    rows = value.get("mission")
    if not isinstance(rows, list) or not rows:
        raise LedgerError("ledger has no [[mission]] rows")
    by_id: dict[str, dict] = {}
    for index, row in enumerate(rows):
        if not isinstance(row, dict):
            raise LedgerError(f"ledger row {index} is not a table")
        allowed = {"id", "zone", "mission", "disposition", "catalog_refs"}
        unknown = set(row) - allowed
        if unknown:
            raise LedgerError(
                f"ledger row {index} has unknown keys: {sorted(unknown)}"
            )
        identifier = row.get("id")
        if not isinstance(identifier, str):
            raise LedgerError(f"ledger row {index} id must be a string")
        if identifier in by_id:
            raise LedgerError(f"ledger duplicate mission id: {identifier}")
        zone = row.get("zone")
        number = row.get("mission")
        disposition = row.get("disposition")
        refs = row.get("catalog_refs")
        if not isinstance(zone, str) or zone not in PINNED_ZONE_SHAPE:
            raise LedgerError(
                f"ledger row {identifier} zone must be one of "
                f"{sorted(PINNED_ZONE_SHAPE)}, found {zone!r}"
            )
        if isinstance(number, bool) or not isinstance(number, int) or number < 1:
            raise LedgerError(
                f"ledger row {identifier} mission must be a positive integer"
            )
        if disposition not in DISPOSITIONS:
            raise LedgerError(
                f"ledger row {identifier} disposition must be one of "
                f"{list(DISPOSITIONS)}, found {disposition!r}"
            )
        expected_id = f"ZONE{zone}-MISSION{number}"
        if identifier != expected_id:
            raise LedgerError(
                f"ledger row id {identifier!r} disagrees with "
                f"zone/mission ({expected_id!r})"
            )
        if not isinstance(refs, list) or not all(isinstance(r, str) for r in refs):
            raise LedgerError(
                f"ledger row {identifier} catalog_refs must be an array of strings"
            )
        for ref in refs:
            if not ref or ref.split() != [ref]:
                raise LedgerError(
                    f"ledger row {identifier} catalog_ref entries must be "
                    f"non-empty and whitespace-free, found {ref!r}"
                )
        if len(set(refs)) != len(refs):
            raise LedgerError(
                f"ledger row {identifier} catalog_refs contains duplicates"
            )
        by_id[identifier] = row
    return value, by_id


def check_ledger_vs_corpus(by_id: dict[str, dict], found: dict[str, list[int]]) -> None:
    corpus_ids = {
        f"ZONE{letter}-MISSION{number}"
        for letter, numbers in found.items()
        for number in numbers
    }
    ledger_ids = set(by_id)
    missing = sorted(corpus_ids - ledger_ids)
    extra = sorted(ledger_ids - corpus_ids)
    if missing:
        raise LedgerError(f"ledger is missing corpus missions: {missing}")
    if extra:
        raise LedgerError(f"ledger carries non-corpus missions: {extra}")
    if len(ledger_ids) != PINNED_TOTAL:
        raise LedgerError(
            f"ledger must carry exactly {PINNED_TOTAL} missions, found {len(ledger_ids)}"
        )


def zone_states(by_id: dict[str, dict]) -> dict[str, dict[str, int]]:
    summary: dict[str, dict[str, int]] = {}
    for letter in sorted(PINNED_ZONE_SHAPE):
        summary[letter] = {"green": 0, "pending": 0, "total": PINNED_ZONE_SHAPE[letter]}
    for row in by_id.values():
        summary[row["zone"]][row["disposition"]] += 1
    return summary


def check_manifest_consistency(root: Path, by_id: dict[str, dict]) -> None:
    manifest_path = root / "docs" / "required-gates.toml"
    try:
        raw = manifest_path.read_bytes()
    except OSError as error:
        raise LedgerError(f"required-gates manifest is missing: {manifest_path}") from error
    try:
        manifest = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise LedgerError(f"required-gates manifest does not parse: {error}") from error
    phases = {
        phase.get("id"): phase
        for phase in manifest.get("phase", [])
        if isinstance(phase, dict)
    }
    p5 = phases.get("P5")
    if p5 is None:
        raise LedgerError("required-gates manifest has no P5 phase")
    required = p5.get("required_gates", [])
    if not isinstance(required, list):
        raise LedgerError("P5 required_gates must be an array")
    # A per-zone completion gate may only be wired once its zone closed.
    for gate_id in required:
        if not isinstance(gate_id, str):
            raise LedgerError("P5 required_gates entries must be strings")
        match = ZONE_GATE_ID.fullmatch(gate_id)
        if not match:
            continue
        letter = match.group(1).upper()
        pending = [
            identifier
            for identifier, row in by_id.items()
            if row["zone"] == letter and row["disposition"] != "green"
        ]
        if pending:
            raise LedgerError(
                f"manifest wires zone completion gate {gate_id} but ledger "
                f"ZONE{letter} still has {len(pending)} non-green missions "
                f"(e.g. {sorted(pending)[0]})"
            )
    # The phase status may only be green once every mission is green.
    status = p5.get("status")
    if status == "green":
        not_green = [
            identifier
            for identifier, row in by_id.items()
            if row["disposition"] != "green"
        ]
        if not_green:
            raise LedgerError(
                f"manifest P5 status is green but {len(not_green)} missions "
                f"are still pending (e.g. {sorted(not_green)[0]})"
            )


def main() -> int:
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=default_root)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    try:
        found = enumerate_corpus(root)
        check_zone_shape(found)
        _, by_id = load_ledger(root)
        check_ledger_vs_corpus(by_id, found)
        check_manifest_consistency(root, by_id)
    except LedgerError as error:
        print(f"p5-zone-ledger: FAIL: {error}", file=sys.stderr)
        return 1
    summary = zone_states(by_id)
    total_green = sum(state["green"] for state in summary.values())
    print("p5-zone-ledger: OK")
    print(
        f"  missions: {len(by_id)} ("
        + ", ".join(
            f"ZONE{letter} {state['green']}/{state['total']} green"
            for letter, state in summary.items()
        )
        + ")"
    )
    print(f"  overall: {total_green}/{len(by_id)} missions green; P5 open")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
