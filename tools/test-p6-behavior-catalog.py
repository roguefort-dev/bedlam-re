#!/usr/bin/python3
"""Hermetic fail-closed contracts for check-p6-behavior-catalog.py.

Every consistency rule of the checker is proven to FAIL LOUDLY on the
specific tampering it guards against, and to pass on the honest scaffold
state (the EMPTY catalog over the all-green 37-mission P5 ledger). One
test also runs the checker against the REAL repository catalog + ledger +
manifest (the same thing the gate runs), pinning the honest state.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("check-p6-behavior-catalog.py")
REPO = CHECKER.parent.parent
PYTHON = sys.executable or "/usr/bin/python3"

ZONE_SHAPE = {"A": 1, "B": 7, "C": 7, "D": 7, "E": 7, "F": 7, "G": 1}
LEDGER_IDS = [
    f"ZONE{letter}-MISSION{number}"
    for letter, count in ZONE_SHAPE.items()
    for number in range(1, count + 1)
]  # 37, the pinned P5 census


def ledger_text(refs_by_id: dict[str, list[str]] | None = None) -> str:
    refs_by_id = refs_by_id or {}
    lines = ['schema = "p5-mission-ledger-v1"']
    for identifier in LEDGER_IDS:
        refs = refs_by_id.get(identifier, [])
        lines += [
            "",
            "[[mission]]",
            f'id = "{identifier}"',
            f'zone = "{identifier[4]}"',
            f'mission = {identifier.split("MISSION")[1]}',
            'disposition = "green"',
            "catalog_refs = [" + ", ".join(f'"{r}"' for r in refs) + "]",
        ]
    return "\n".join(lines) + "\n"


def entry(**overrides) -> dict:
    base = {
        "id": "obs-001",
        "title": "sample original behavior",
        "class": "gameplay-coupled",
        "observed": "original",
        "repro": "oracle: pinned DOSBox-X run, scenario S# at frame N",
        "missions": ["ZONEA-MISSION1"],
        "disposition": "open",
        "provenance": "D2xx VERIFIED",
    }
    base.update(overrides)
    return base


def catalog_text(entries: list[dict], schema: str = "p6-behavior-catalog-v1") -> str:
    lines = [f'schema = "{schema}"']
    for row in entries:
        lines.append("")
        lines.append("[[entry]]")
        for key, value in row.items():
            if value is None:
                continue
            if isinstance(value, list):
                lines.append(key + " = [" + ", ".join(f'"{v}"' for v in value) + "]")
            elif isinstance(value, bool):
                lines.append(f"{key} = {str(value).lower()}")
            else:
                lines.append(f'{key} = "{value}"')
    return "\n".join(lines) + "\n"


def manifest_text(
    p6_gates: list[str] | None = None,
    p6_status: str = "pending",
    with_scaffold_gate: bool = True,
) -> str:
    if p6_gates is None:
        p6_gates = ["p6-modernization-scaffold"]
    lines = ['schema = "required-gates-v1"']
    for number in range(8):
        status = "green" if number < 6 else (p6_status if number == 6 else "pending")
        gates = p6_gates if number == 6 else []
        rendered = ", ".join(f'"{g}"' for g in gates)
        lines += [
            "",
            "[[phase]]",
            f'id = "P{number}"',
            f'status = "{status}"',
            f"required_gates = [{rendered}]",
        ]
    if with_scaffold_gate and p6_gates:
        lines += [
            "",
            "[[gate]]",
            'id = "p6-modernization-scaffold"',
            "timeout_seconds = 120",
            'commands = [["/usr/bin/python3", "fixture"]]',
            'tracked_paths = ["fixture"]',
        ]
    return "\n".join(lines) + "\n"


class CatalogCheckerTests(unittest.TestCase):
    def fixture(
        self,
        catalog: str | None = None,
        ledger: str | None = None,
        manifest: str | None = None,
        with_catalog: bool = True,
        with_ledger: bool = True,
    ) -> Path:
        # Fixtures live under HOME: inside the gate that is the validator's
        # repo-anchored scratch home (writable through the sandbox), the
        # test-validate-required-gates.py convention.
        base = Path(os.environ.get("HOME") or tempfile.gettempdir())
        root = Path(tempfile.mkdtemp(prefix="p6-catalog-", dir=base))
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        (root / "docs").mkdir()
        if with_catalog:
            (root / "docs" / "P6-BEHAVIOR-CATALOG.toml").write_text(
                catalog if catalog is not None else catalog_text([])
            )
        if with_ledger:
            (root / "docs" / "P5-MISSION-LEDGER.toml").write_text(
                ledger if ledger is not None else ledger_text()
            )
        (root / "docs" / "required-gates.toml").write_text(
            manifest if manifest is not None else manifest_text()
        )
        return root

    def run_checker(self, root: Path) -> tuple[int, str]:
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(root)],
            capture_output=True,
            text=True,
            timeout=60,
        )
        return result.returncode, result.stdout + result.stderr

    def test_honest_empty_catalog_passes(self):
        root = self.fixture()
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)
        self.assertIn("entries: 0", output)

    def test_real_repo_catalog_ledger_manifest_pass(self):
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(REPO)],
            capture_output=True,
            text=True,
            timeout=120,
        )
        output = result.stdout + result.stderr
        self.assertEqual(result.returncode, 0, output)
        # The honest post-P5 scaffold state (D200 seeding policy): the
        # catalog is EMPTY, the ledger is the 37-mission all-green census
        # with zero catalog_refs, and the manifest wires the scaffold as
        # the FIRST P6 gate. Move these pins ONLY with a deliberate
        # disposition/catalog change, same commit (the fingerprint
        # discipline).
        self.assertIn("entries: 0 (open 0;", output)
        self.assertIn("37 ledger ids", output)
        self.assertIn("0 ledger catalog_refs resolve", output)

    def test_missing_catalog_fails_closed(self):
        root = self.fixture(with_catalog=False)
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("catalog is missing", output)

    def test_missing_ledger_fails_closed(self):
        root = self.fixture(with_ledger=False)
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("ledger is missing", output)

    def test_bad_schema_fails(self):
        root = self.fixture(catalog=catalog_text([], schema="p6-behavior-catalog-v0"))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("schema must be p6-behavior-catalog-v1", output)

    def test_unknown_entry_key_fails(self):
        catalog = catalog_text([entry(note="x")])
        root = self.fixture(catalog=catalog)
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("unknown keys", output)

    def test_duplicate_id_fails(self):
        root = self.fixture(catalog=catalog_text([entry(), entry()]))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("duplicate entry id", output)

    def test_id_hygiene_fails(self):
        # "" trips the non-empty-string rule; "has space" the
        # whitespace-free rule — both fail loud.
        for bad, message in (("", "non-empty string"), ("has space", "whitespace-free")):
            with self.subTest(bad=bad):
                root = self.fixture(catalog=catalog_text([entry(id=bad)]))
                code, output = self.run_checker(root)
                self.assertNotEqual(code, 0)
                self.assertIn(message, output)

    def test_bad_class_fails(self):
        root = self.fixture(catalog=catalog_text([entry(**{"class": "severity-3"})]))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("class must be one of", output)

    def test_bad_observed_fails(self):
        root = self.fixture(catalog=catalog_text([entry(observed="rumor")]))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("observed must be one of", output)

    def test_bad_disposition_fails(self):
        root = self.fixture(catalog=catalog_text([entry(disposition="done")]))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("disposition must be one of", output)

    def test_closed_disposition_must_match_class_rubric(self):
        # R1: the PLAN §6 rubric as code — each class closes to exactly
        # one terminal disposition.
        cases = [
            ("crash-data-loss", "closed-fix-modern"),
            ("crash-data-loss", "closed-preserve-classic"),
            ("cosmetic", "closed-fix-everywhere"),
            ("cosmetic", "closed-preserve-classic"),
            ("gameplay-coupled", "closed-fix-everywhere"),
            ("gameplay-coupled", "closed-fix-modern"),
        ]
        for entry_class, disposition in cases:
            with self.subTest(cls=entry_class, disposition=disposition):
                root = self.fixture(
                    catalog=catalog_text(
                        [
                            entry(
                                **{
                                    "class": entry_class,
                                    "disposition": disposition,
                                    "evidence": "tests/x.rs::y",
                                    "purist_toggle": (
                                        "preserve-x" if disposition == "closed-preserve-classic" else None
                                    ),
                                }
                            )
                        ]
                    )
                )
                code, output = self.run_checker(root)
                self.assertNotEqual(code, 0)
                self.assertIn("may only close to", output)

    def test_valid_closures_pass_rubric(self):
        entries = [
            entry(
                id="obs-crash",
                **{
                    "class": "crash-data-loss",
                    "disposition": "closed-fix-everywhere",
                    "evidence": "tests/a.rs::crash_fixed",
                },
            ),
            entry(
                id="obs-cosmetic",
                **{
                    "class": "cosmetic",
                    "disposition": "closed-fix-modern",
                    "evidence": "tests/a.rs::cosmetic_fixed",
                },
            ),
            entry(
                id="obs-feel",
                **{
                    "class": "gameplay-coupled",
                    "disposition": "closed-preserve-classic",
                    "evidence": "tests/a.rs::both_arms",
                    "purist_toggle": "preserve-obs-feel",
                },
            ),
        ]
        root = self.fixture(catalog=catalog_text(entries))
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)
        self.assertIn("open 0", output)
        self.assertIn("closed-fix-everywhere 1", output)
        self.assertIn("closed-fix-modern 1", output)
        self.assertIn("closed-preserve-classic 1", output)

    def test_closed_without_evidence_fails(self):
        root = self.fixture(
            catalog=catalog_text(
                [
                    entry(
                        **{
                            "class": "cosmetic",
                            "disposition": "closed-fix-modern",
                        }
                    )
                ]
            )
        )
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("no regression evidence", output)

    def test_open_with_evidence_fails(self):
        root = self.fixture(
            catalog=catalog_text([entry(evidence="tests/a.rs::premature")])
        )
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("is open but carries evidence", output)

    def test_preserve_without_toggle_fails(self):
        root = self.fixture(
            catalog=catalog_text(
                [
                    entry(
                        disposition="closed-preserve-classic",
                        evidence="tests/a.rs::both_arms",
                    )
                ]
            )
        )
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("no purist_toggle", output)

    def test_toggle_on_non_preserving_disposition_fails(self):
        for row in (
            entry(purist_toggle="preserve-x"),  # open
            entry(
                **{
                    "class": "cosmetic",
                    "disposition": "closed-fix-modern",
                    "evidence": "tests/a.rs::c",
                    "purist_toggle": "preserve-x",
                }
            ),
        ):
            with self.subTest(disposition=row["disposition"]):
                root = self.fixture(catalog=catalog_text([row]))
                code, output = self.run_checker(root)
                self.assertNotEqual(code, 0)
                self.assertIn("only closed-preserve-classic entries carry a toggle", output)

    def test_whitespace_toggle_fails(self):
        root = self.fixture(
            catalog=catalog_text(
                [
                    entry(
                        disposition="closed-preserve-classic",
                        evidence="tests/a.rs::both_arms",
                        purist_toggle="has space",
                    )
                ]
            )
        )
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("purist_toggle must be whitespace-free", output)

    def test_duplicate_toggle_fails(self):
        shared = dict(
            disposition="closed-preserve-classic",
            evidence="tests/a.rs::both_arms",
            purist_toggle="preserve-x",
        )
        root = self.fixture(
            catalog=catalog_text([entry(id="obs-001", **shared), entry(id="obs-002", **shared)])
        )
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("share purist_toggle", output)

    def test_unknown_mission_fails(self):
        root = self.fixture(catalog=catalog_text([entry(missions=["ZONEA-MISSION2"])]))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("not P5 ledger mission ids", output)

    def test_bad_mission_lists_fail(self):
        for missions in ([], ["ZONEA-MISSION1", "ZONEA-MISSION1"]):
            with self.subTest(missions=missions):
                root = self.fixture(catalog=catalog_text([entry(missions=missions)]))
                code, output = self.run_checker(root)
                self.assertNotEqual(code, 0)
                self.assertIn("missions", output)

    def test_empty_required_strings_fail(self):
        for key in ("title", "repro", "provenance"):
            with self.subTest(key=key):
                root = self.fixture(catalog=catalog_text([entry(**{key: ""})]))
                code, output = self.run_checker(root)
                self.assertNotEqual(code, 0)
                self.assertIn(f"{key} must be a non-empty string", output)

    def test_dangling_ledger_catalog_ref_fails(self):
        # R5: the P5 feed join — a ledger ref that resolves to nothing
        # fails loud.
        root = self.fixture(ledger=ledger_text({"ZONEA-MISSION1": ["obs-ghost"]}))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("do not resolve to catalog entries", output)
        self.assertIn("obs-ghost", output)

    def test_resolving_ledger_catalog_ref_passes(self):
        # The join works in the wired direction: ledger ref -> entry, and
        # the entry's missions ground back into the ledger.
        root = self.fixture(
            catalog=catalog_text([entry()]),
            ledger=ledger_text({"ZONEA-MISSION1": ["obs-001"]}),
        )
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)
        self.assertIn("entries: 1 (open 1", output)
        self.assertIn("1 ledger catalog_refs resolve", output)

    def test_manifest_scaffold_not_first_fails(self):
        for gates in (
            ["p6-some-behavior-gate"],
            ["p6-some-behavior-gate", "p6-modernization-scaffold"],
        ):
            with self.subTest(gates=gates):
                root = self.fixture(manifest=manifest_text(p6_gates=gates))
                code, output = self.run_checker(root)
                self.assertNotEqual(code, 0)
                self.assertIn("must start with p6-modernization-scaffold", output)

    def test_manifest_scaffold_gate_undefined_fails(self):
        root = self.fixture(
            manifest=manifest_text(p6_gates=["p6-modernization-scaffold"], with_scaffold_gate=False)
        )
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("no [[gate]] with that id is defined", output)

    def test_manifest_empty_p6_gates_pass(self):
        # R6 binds only once P6 gates exist (the vacuous pre-wiring state).
        root = self.fixture(manifest=manifest_text(p6_gates=[]))
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)

    def test_p6_status_green_with_open_entry_fails(self):
        root = self.fixture(
            catalog=catalog_text([entry()]),
            manifest=manifest_text(p6_status="green"),
        )
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("P6 status is green", output)
        self.assertIn("still open", output)

    def test_p6_status_green_with_closed_entries_passes(self):
        # Necessary-not-sufficient: an empty-or-fully-closed catalog alone
        # must not read as full P6 completion anywhere else; this rule
        # only blocks a premature flip with untriaged entries.
        root = self.fixture(
            catalog=catalog_text(
                [
                    entry(
                        **{
                            "class": "cosmetic",
                            "disposition": "closed-fix-modern",
                            "evidence": "tests/a.rs::c",
                        }
                    )
                ]
            ),
            manifest=manifest_text(p6_status="green"),
        )
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)

    def test_open_entry_each_class_passes(self):
        # Triage lifecycle: an entry of ANY class may sit open (observed
        # + classed, fix not yet implemented/evidenced).
        entries = [
            entry(id="obs-a", **{"class": "crash-data-loss"}),
            entry(id="obs-b", **{"class": "gameplay-coupled"}),
            entry(id="obs-c", **{"class": "cosmetic"}),
        ]
        root = self.fixture(catalog=catalog_text(entries))
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)
        self.assertIn("open 3", output)


if __name__ == "__main__":
    unittest.main(verbosity=2)
