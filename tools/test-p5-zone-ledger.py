#!/usr/bin/python3
"""Hermetic fail-closed contracts for check-p5-zone-ledger.py.

Every consistency rule of the checker is proven to FAIL LOUDLY on the
specific tampering it guards against, and to pass on the honest scaffold
state (37 pending missions). One test also runs the checker against the
REAL repository ledger + read-only corpus (the same thing the gate runs).
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("check-p5-zone-ledger.py")
REPO = CHECKER.parent.parent
PYTHON = sys.executable or "/usr/bin/python3"

ZONE_SHAPE = {"A": 1, "B": 7, "C": 7, "D": 7, "E": 7, "F": 7, "G": 1}
CORPUS_SHAPE = {
    letter: list(range(1, count + 1)) for letter, count in ZONE_SHAPE.items()
}


def ledger_text(rows: list[dict], schema: str = "p5-mission-ledger-v1") -> str:
    lines = [f'schema = "{schema}"']
    for row in rows:
        lines.append("")
        lines.append("[[mission]]")
        lines.append(f'id = "{row["id"]}"')
        lines.append(f'zone = "{row["zone"]}"')
        lines.append(f'mission = {row["mission"]}')
        lines.append(f'disposition = "{row.get("disposition", "pending")}"')
        refs = row.get("catalog_refs", [])
        lines.append("catalog_refs = [" + ", ".join(f'"{r}"' for r in refs) + "]")
    return "\n".join(lines) + "\n"


def honest_rows(tamper=None) -> list[dict]:
    rows = [
        {
            "id": f"ZONE{letter}-MISSION{number}",
            "zone": letter,
            "mission": number,
            "disposition": "pending",
            "catalog_refs": [],
        }
        for letter, count in ZONE_SHAPE.items()
        for number in range(1, count + 1)
    ]
    if tamper is not None:
        tamper(rows)
    return rows


def manifest_text(p5_gates: list[str] | None = None, p5_status: str = "pending") -> str:
    gates = p5_gates if p5_gates is not None else ["p5-zone-gate-scaffold"]
    return (
        'schema = "required-gates-v1"\n\n'
        "".join(
            f'[[phase]]\nid = "P{n}"\nstatus = "{("green" if n < 5 else p5_status if n == 5 else "pending")}"\n'
            f'required_gates = {("[]" if n != 5 else repr(gates).replace("'", '"'))}\n\n'
            for n in range(8)
        )
    )


class LedgerCheckerTests(unittest.TestCase):
    def fixture(
        self,
        ledger: str,
        manifest: str | None = None,
        corpus: dict[str, list[int]] | None = None,
        with_corpus: bool = True,
    ) -> Path:
        # Fixtures live under HOME: inside the gate that is the validator's
        # repo-anchored scratch home (writable through the sandbox), the
        # test-validate-required-gates.py convention.
        base = Path(os.environ.get("HOME") or tempfile.gettempdir())
        root = Path(tempfile.mkdtemp(prefix="p5-ledger-", dir=base))
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        (root / "docs").mkdir()
        (root / "docs" / "P5-MISSION-LEDGER.toml").write_text(ledger)
        if manifest is not None:
            (root / "docs" / "required-gates.toml").write_text(manifest)
        else:
            (root / "docs" / "required-gates.toml").write_text(manifest_text())
        if with_corpus:
            shape = corpus if corpus is not None else CORPUS_SHAPE
            for letter, numbers in shape.items():
                zone_dir = root / "game-data" / "BEDLAM" / "EDITOR" / f"ZONE{letter}"
                zone_dir.mkdir(parents=True)
                for number in numbers:
                    (zone_dir / f"MISSION{number}.TOT").write_bytes(b"fixture")
        return root

    def run_checker(self, root: Path) -> tuple[int, str]:
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(root)],
            capture_output=True,
            text=True,
            timeout=60,
        )
        return result.returncode, result.stdout + result.stderr

    def test_honest_scaffold_passes(self):
        root = self.fixture(ledger_text(honest_rows()))
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)
        self.assertIn("37", output)

    def test_real_repo_ledger_and_corpus_pass(self):
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(REPO)],
            capture_output=True,
            text=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        # Re-baselined with the D178 ZONEA-MISSION1 flip (the first
        # green disposition; was "0/37 missions green" while every
        # mission was pending), then with the D192 ZONEB closure (the
        # first 7-mission zone), then with the D193 ZONEC closure
        # (the first pure ZoneSpec instantiation). Move this pin ONLY
        # with a deliberate disposition flip, same commit (the
        # fingerprint discipline).
        self.assertIn("15/37 missions green", result.stdout)
        self.assertIn("ZONEA 1/1 green", result.stdout)
        self.assertIn("ZONEB 7/7 green", result.stdout)
        self.assertIn("ZONEC 7/7 green", result.stdout)

    def test_missing_corpus_fails_closed(self):
        root = self.fixture(ledger_text(honest_rows()), with_corpus=False)
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("corpus is unavailable", output)

    def test_missing_ledger_row_fails(self):
        def tamper(rows):
            rows[:] = [row for row in rows if row["id"] != "ZONED-MISSION4"]

        root = self.fixture(ledger_text(honest_rows(tamper)))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("missing corpus missions", output)
        self.assertIn("ZONED-MISSION4", output)

    def test_extra_ledger_row_fails(self):
        def tamper(rows):
            rows.append(
                {"id": "ZONEA-MISSION2", "zone": "A", "mission": 2}
            )

        root = self.fixture(ledger_text(honest_rows(tamper)))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("non-corpus missions", output)

    def test_duplicate_id_fails(self):
        def tamper(rows):
            rows.append(dict(rows[0]))

        root = self.fixture(ledger_text(honest_rows(tamper)))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("duplicate mission id", output)

    def test_id_zone_mission_mismatch_fails(self):
        def tamper(rows):
            rows[0]["id"] = "ZONEB-MISSION1"

        root = self.fixture(ledger_text(honest_rows(tamper)))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("disagrees with zone/mission", output)

    def test_bad_disposition_fails(self):
        def tamper(rows):
            rows[3]["disposition"] = "done"

        root = self.fixture(ledger_text(honest_rows(tamper)))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("disposition must be one of", output)

    def test_bad_schema_fails(self):
        root = self.fixture(ledger_text(honest_rows(), schema="p5-mission-ledger-v0"))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("schema must be p5-mission-ledger-v1", output)

    def test_unknown_row_key_fails(self):
        ledger = ledger_text(honest_rows()).replace(
            '[[mission]]\nid = "ZONEA-MISSION1"',
            '[[mission]]\nid = "ZONEA-MISSION1"\nnote = "x"',
        )
        root = self.fixture(ledger)
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("unknown keys", output)

    def test_catalog_ref_validation_fails_closed(self):
        for refs in ([""], ["a", "a"], ["has space"]):
            with self.subTest(refs=refs):
                def tamper(rows, refs=refs):
                    rows[0]["catalog_refs"] = refs

                root = self.fixture(ledger_text(honest_rows(tamper)))
                code, output = self.run_checker(root)
                self.assertNotEqual(code, 0)
                self.assertIn("catalog_ref", output)

    def test_corpus_drift_from_pinned_census_fails(self):
        drifted = dict(CORPUS_SHAPE)
        drifted["A"] = [1, 2]  # a stray ZONEA/MISSION2.TOT
        root = self.fixture(ledger_text(honest_rows()), corpus=drifted)
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("drifted from the pinned census", output)

    def test_missing_corpus_zone_fails(self):
        drifted = {k: v for k, v in CORPUS_SHAPE.items() if k != "G"}
        root = self.fixture(ledger_text(honest_rows()), corpus=drifted)
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("zone set drifted", output)

    def test_zone_gate_wired_before_zone_green_fails(self):
        root = self.fixture(ledger_text(honest_rows()), manifest=manifest_text(["p5-zone-gate-scaffold", "p5-zone-a"]))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("p5-zone-a", output)
        self.assertIn("non-green missions", output)

    def test_zone_gate_passes_once_zone_is_green(self):
        def green_zone_a(rows):
            for row in rows:
                if row["zone"] == "A":
                    row["disposition"] = "green"

        root = self.fixture(
            ledger_text(honest_rows(green_zone_a)),
            manifest=manifest_text(["p5-zone-gate-scaffold", "p5-zone-a"]),
        )
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)

    def test_p5_status_green_with_pending_missions_fails(self):
        root = self.fixture(ledger_text(honest_rows()), manifest=manifest_text(p5_status="green"))
        code, output = self.run_checker(root)
        self.assertNotEqual(code, 0)
        self.assertIn("P5 status is green", output)

    def test_p5_status_green_with_all_green_passes(self):
        def all_green(rows):
            for row in rows:
                row["disposition"] = "green"

        root = self.fixture(
            ledger_text(honest_rows(all_green)),
            manifest=manifest_text(p5_status="green"),
        )
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)
        self.assertIn("37/37 missions green", output)

    def test_scaffold_gate_id_is_not_a_zone_gate(self):
        # p5-zone-gate-scaffold must not be captured by the p5-zone-{a..g}
        # consistency rule: the scaffold is green while missions are pending.
        root = self.fixture(
            ledger_text(honest_rows()),
            manifest=manifest_text(["p5-zone-gate-scaffold"]),
        )
        code, output = self.run_checker(root)
        self.assertEqual(code, 0, output)


if __name__ == "__main__":
    unittest.main(verbosity=2)
