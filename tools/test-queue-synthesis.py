#!/usr/bin/env python3
"""Hermetic contracts for deterministic failed-product-gate synthesis (D240).

The controller hook is tools/nudge-state.py synthesize-product-work, called
by tools/nudge.sh's completion branch when the required queue is empty and
the sealed full-battery required-gates validation failed. These tests drive
the REAL action CLI over committed git fixtures and pin the classification:

  - a red product gate (own evidence ran) synthesizes a READY repair item
    citing the gate id, its first red command, and the exit code;
  - a phase wiring no product gate synthesizes a READY wiring item;
  - red non-product gates, dependency consequences without a red product
    root, error-shaped (validator/sandbox/corpus/harness) reports, stale
    reports, non-empty queues, and claim residue NEVER synthesize -- the
    queue stays byte-identical for the caller to beacon instead;
  - every published queue parses under the strict grammar
    (tools/nudge-free-items.py) and no synthesized item ever asserts a
    phase status (the word "green" never appears in the queue).
"""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


TOOLS = Path(__file__).parent
STATE_HELPER = TOOLS / "nudge-state.py"
QUEUE_PARSER = TOOLS / "nudge-free-items.py"
GIT = "/usr/bin/git"

EMPTY_QUEUE = (
    "# NEXT - task queue (synthesis fixture)\n"
    "\n"
    "## Now\n"
    "\n"
    "## Backlog\n"
    "\n"
    "## Done\n"
    "1. DONE fixture history line\n"
)

FULL_COVERAGE = {f"P{number}": 1 for number in range(8)}


def gate_entry(gate_id, evidence, *, passed, commands):
    return {
        "commands": [{"argv": argv, "rc": rc} for argv, rc in commands],
        "evidence": evidence,
        "id": gate_id,
        "passed": passed,
        "writable": [],
    }


class QueueSynthesisTests(unittest.TestCase):
    def fixture(self, report: dict) -> Path:
        """A committed fixture root with an empty active queue and a report."""
        base = Path(os.environ.get("HOME") or tempfile.gettempdir())
        root = Path(tempfile.mkdtemp(prefix="queue-synthesis-", dir=base))
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        state = root / ".state"
        (state / "claims").mkdir(parents=True)
        (state / "NEXT.md").write_text(EMPTY_QUEUE)
        subprocess.run([GIT, "init", "-q", str(root)], check=True)
        subprocess.run([GIT, "-C", str(root), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run([GIT, "-C", str(root), "config", "user.name", "test"], check=True)
        subprocess.run([GIT, "-C", str(root), "add", ".state/NEXT.md"], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "fixture queue"], check=True)
        head = subprocess.run(
            [GIT, "-C", str(root), "rev-parse", "HEAD"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
        report = {"head": head, **report}
        evidence = {gate["id"]: gate["evidence"] for gate in report["gates"]}
        report.setdefault("evidence", evidence)
        report_path = state / "required-gates-report.json"
        payload = __import__("json").dumps(report)
        report_path.write_text(payload)
        os.chmod(report_path, 0o600)
        return root

    def run_synthesis(self, root: Path):
        return subprocess.run(
            [sys.executable, str(STATE_HELPER), "synthesize-product-work",
             str(root / ".state/required-gates-report.json"),
             str(root / ".state/NEXT.md"),
             str(root / ".state/claims")],
            capture_output=True, text=True,
        )

    def queue_state(self, root: Path) -> str:
        result = subprocess.run(
            [sys.executable, str(QUEUE_PARSER),
             str(root / ".state/NEXT.md"), str(root / ".state/claims"), "--state-v1"],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def queue_text(self, root: Path) -> str:
        return (root / ".state/NEXT.md").read_text()

    # --- failure type 1: a red product gate synthesizes a repair item ---

    def test_red_product_gate_synthesizes_repair_item(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": dict(FULL_COVERAGE),
            "gates": [
                gate_entry("menu-journey", "product", passed=False,
                           commands=[(["/usr/bin/python3", "tools/test-menu-journey.py"], 1)]),
                gate_entry("eng-legacy", "supporting", passed=True, commands=[]),
            ],
        })
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = __import__("json").loads(result.stdout)
        self.assertEqual(summary["schema"], "queue-synthesis-v1")
        self.assertEqual(summary["ids"], ["synth-repair-menu-journey"])
        text = self.queue_text(root)
        self.assertIn("[id=synth-repair-menu-journey] [gate=menu-journey]", text)
        self.assertIn('command "/usr/bin/python3 tools/test-menu-journey.py" exited rc=1', text)
        self.assertIn("SYNTHESIZED BY THE CONTROLLER from failed product gate menu-journey", text)
        self.assertNotIn("synth-wire-", text)
        # The published queue parses under the strict grammar and the item
        # is claimable work.
        self.assertEqual(self.queue_state(root), "RUNNABLE 1")
        # No synthesized item ever asserts a phase status.
        self.assertNotIn("green", text)
        self.assertNotIn(text, before)  # the queue actually changed

    def test_red_product_gate_timeout_cites_rc_and_withholds_uncitable_argv(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": dict(FULL_COVERAGE),
            "gates": [
                gate_entry("trace-gate", "product", passed=False,
                           commands=[(["/usr/bin/bash", "tools/run [odd] manual.sh"], 124)]),
            ],
        })
        result = self.run_synthesis(root)
        self.assertEqual(result.returncode, 0, result.stderr)
        text = self.queue_text(root)
        # Brackets and forbidden tokens are withheld, never quoted.
        self.assertIn("withheld from the queue for grammar safety", text)
        self.assertIn("rc=124", text)
        self.assertNotIn("[odd]", text)
        self.assertEqual(self.queue_state(root), "RUNNABLE 1")

    # --- failure type 2: an infrastructure-shaped failure never synthesizes ---

    def test_error_shaped_report_never_synthesizes(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "gates": [],
            "error": "required network/PID containment is unavailable",
        })
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refused", (result.stderr + result.stdout))
        self.assertEqual(self.queue_text(root), before)
        self.assertEqual(self.queue_state(root), "REQUIRED-QUEUE-EMPTY")

    def test_stale_head_report_never_synthesizes(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "phase_product_coverage": dict(FULL_COVERAGE),
            "gates": [
                gate_entry("menu-journey", "product", passed=False,
                           commands=[(["/usr/bin/true"], 1)]),
            ],
        })
        # Move HEAD after the report was bound: the report is now stale.
        (root / "code.txt").write_text("drift\n")
        subprocess.run([GIT, "-C", str(root), "add", "code.txt"], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "drift"], check=True)
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("stale", (result.stderr + result.stdout))
        self.assertEqual(self.queue_text(root), before)

    def test_passed_report_never_synthesizes(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "passed",
            "plan_complete": True,
            "phase_product_coverage": dict(FULL_COVERAGE),
            "gates": [],
        })
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.queue_text(root), before)

    # --- the non-product refusal ---

    def test_red_non_product_gate_never_synthesizes(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": dict(FULL_COVERAGE),
            "gates": [
                gate_entry("gates-validator", "infrastructure", passed=False,
                           commands=[(["/usr/bin/python3", "tools/test-validate-required-gates.py"], 1)]),
                gate_entry("menu-journey", "product", passed=True,
                           commands=[(["/usr/bin/true"], 0)]),
            ],
        })
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        combined = result.stderr + result.stdout
        self.assertIn("non-product gates never synthesize", combined)
        self.assertIn("gates-validator", combined)
        self.assertEqual(self.queue_text(root), before)
        self.assertEqual(self.queue_state(root), "REQUIRED-QUEUE-EMPTY")

    def test_dependency_consequence_of_red_product_root_adds_no_item(self):
        # A product gate that never ran (dependency consequence) must not be
        # synthesized; only its red product root is.
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": dict(FULL_COVERAGE),
            "gates": [
                gate_entry("journey-core", "product", passed=False,
                           commands=[(["/usr/bin/python3", "tools/test-core.py"], 3)]),
                gate_entry("journey-dependent", "product", passed=False, commands=[]),
            ],
        })
        result = self.run_synthesis(root)
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = __import__("json").loads(result.stdout)
        self.assertEqual(summary["ids"], ["synth-repair-journey-core"])
        self.assertEqual(self.queue_state(root), "RUNNABLE 1")

    def test_blocked_product_gate_with_non_product_root_never_synthesizes(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": dict(FULL_COVERAGE),
            "gates": [
                gate_entry("eng-base", "static", passed=False,
                           commands=[(["/usr/bin/test", "-f", "proof.txt"], 1)]),
                gate_entry("journey-dependent", "product", passed=False, commands=[]),
            ],
        })
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("non-product gates never synthesize", result.stderr + result.stdout)
        self.assertEqual(self.queue_text(root), before)

    # --- the absent-product-gate synthesis path ---

    def test_absent_product_gates_synthesize_wiring_items(self):
        coverage = dict(FULL_COVERAGE)
        coverage["P0"] = 0
        coverage["P5"] = 0
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": coverage,
            "gates": [
                gate_entry("eng-legacy", "supporting", passed=True,
                           commands=[(["/usr/bin/true"], 0)]),
            ],
        })
        result = self.run_synthesis(root)
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = __import__("json").loads(result.stdout)
        self.assertEqual(summary["ids"], ["synth-wire-p0", "synth-wire-p5"])
        text = self.queue_text(root)
        self.assertIn("[id=synth-wire-p5] [gate=synth-wire-p5]", text)
        self.assertIn("the empty-queue completion validation found this phase wires no evidence=product gate", text)
        self.assertIn("phase P5 at HEAD", text)
        self.assertNotIn("green", text)
        self.assertEqual(self.queue_state(root), "RUNNABLE 1 2")

    def test_red_product_gate_and_absent_phases_synthesize_both(self):
        coverage = {f"P{number}": 0 for number in range(8)}
        coverage["P6"] = 1
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": coverage,
            "gates": [
                gate_entry("menu-journey", "product", passed=False,
                           commands=[(["/usr/bin/python3", "tools/test-menu.py"], 2)]),
            ],
        })
        result = self.run_synthesis(root)
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = __import__("json").loads(result.stdout)
        # Repairs first, then wiring items by phase id.
        self.assertEqual(summary["ids"], ["synth-repair-menu-journey"] + [
            f"synth-wire-p{number}" for number in (0, 1, 2, 3, 4, 5, 7)
        ])
        self.assertEqual(self.queue_state(root), "RUNNABLE 1 2 3 4 5 6 7 8")

    # --- fail-closed publication guards ---

    def test_non_empty_queue_refuses(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": {f"P{number}": 0 for number in range(8)},
            "gates": [],
        })
        (root / ".state/NEXT.md").write_text(
            "# NEXT\n\n## Now\n1. [READY] [id=live-item] [gate=live-gate] live work\n\n## Backlog\n"
        )
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not empty", result.stderr + result.stdout)

    def test_claim_residue_refuses(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": {f"P{number}": 0 for number in range(8)},
            "gates": [],
        })
        (root / ".state/claims/1-owner.claim").write_text("lock-v2\n")
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("claims block queue synthesis", result.stderr + result.stdout)
        self.assertEqual(self.queue_text(root), before)

    def test_missing_report_refuses(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "phase_product_coverage": {f"P{number}": 0 for number in range(8)},
            "gates": [],
        })
        (root / ".state/required-gates-report.json").unlink()
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.queue_text(root), before)

    def test_all_passing_gates_with_full_coverage_refuses(self):
        # Every phase wires a product gate and every gate that ran is green:
        # the failure shape is not product-class, so nothing synthesizes.
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": None,
            "phase_product_coverage": dict(FULL_COVERAGE),
            "gates": [
                gate_entry("menu-journey", "product", passed=True,
                           commands=[(["/usr/bin/true"], 0)]),
            ],
        })
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not synthesis-class", result.stderr + result.stdout)
        self.assertEqual(self.queue_text(root), before)

    def test_phase_run_report_refuses(self):
        root = self.fixture({
            "schema": "required-gates-report-v2",
            "status": "failed",
            "plan_complete": False,
            "selected_phase": "P5",
            "phase_product_coverage": {"P5": 0},
            "gates": [],
        })
        before = self.queue_text(root)
        result = self.run_synthesis(root)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("bounded phase run", result.stderr + result.stdout)
        self.assertEqual(self.queue_text(root), before)


if __name__ == "__main__":
    unittest.main()
