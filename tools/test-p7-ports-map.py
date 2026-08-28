#!/usr/bin/env python3
"""Hermetic fail-closed contracts for check-p7-ports-map.py.

Every structural rule of the checker is proven to FAIL LOUDLY on the
specific tampering it guards against, and to pass on the honest
scaffold state (the committed doc + a correctly wired manifest). One
test runs the checker against the REAL repository doc + manifest (the
same thing the gate runs), pinning the honest state, and one proves
the LANDED-state shape is legal (a deliverable flipped to landed with
its proving gate wired into the P7 phase list passes -- the forward
shape P7 grows into).
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("check-p7-ports-map.py")
REPO = CHECKER.parent.parent
PYTHON = sys.executable or "/usr/bin/python3"
DOC_RELATIVE = "docs/P7-PORTS.md"
GATE_ID = "p7-ports-scaffold"
TOML_BLOCK = re.compile(r"(```toml\r?\n)(.*?)(\r?\n```)", re.DOTALL)
REGISTRY_MARK = 'schema = "p7-ports-map-v1"'


def flat(text: str) -> str:
    """Whitespace-collapse like the checker's normalize_ws."""
    return " ".join(text.split())


def tamper(doc: str, sentence: str) -> str:
    """Remove every whitespace-flexible occurrence of a rule sentence.

    The checker matches sentences after whitespace normalization, so
    simply replacing the single-line spelling can leave a wrapped copy
    behind. This builds a newline-tolerant pattern and removes ALL
    occurrences.
    """
    pattern = re.compile(r"\s+".join(re.escape(word) for word in sentence.split()))
    return pattern.sub("TAMPERED-SENTENCE", doc)


def with_registry(doc: str, registry: str) -> str:
    """Replace the p7-ports-map-v1 fenced block's inner TOML."""
    replaced = {"done": False}

    def swap(match: re.Match[str]) -> str:
        if REGISTRY_MARK in match.group(2) and not replaced["done"]:
            replaced["done"] = True
            return match.group(1) + registry + match.group(3)
        return match.group(0)

    result = TOML_BLOCK.sub(swap, doc)
    if not replaced["done"]:
        raise AssertionError("fixture bug: registry block not found")
    return result


def land_steamdeck(doc: str, gate: str) -> str:
    """Flip the honest steamdeck-default row to landed (in place)."""
    before = (
        'id = "steamdeck-default"\nkind = "engineering"\n'
        'plan_anchor = "SteamDeck defaults stretch"\n'
        'status = "pending"\ngate = ""'
    )
    after = (
        'id = "steamdeck-default"\nkind = "engineering"\n'
        'plan_anchor = "SteamDeck defaults stretch"\n'
        f'status = "landed"\ngate = "{gate}"'
    )
    result = doc.replace(before, after, 1)
    if result == doc:
        raise AssertionError("fixture bug: steamdeck row not found")
    return result


def append_registry(doc: str, extra: str) -> str:
    """Append rows to the p7-ports-map-v1 fenced block (keeps the rest)."""
    replaced = {"done": False}

    def swap(match: re.Match[str]) -> str:
        if REGISTRY_MARK in match.group(2) and not replaced["done"]:
            replaced["done"] = True
            inner = match.group(2)
            sep = "" if inner.endswith("\n") else "\n"
            return match.group(1) + inner + sep + extra + match.group(3)
        return match.group(0)

    result = TOML_BLOCK.sub(swap, doc)
    if not replaced["done"]:
        raise AssertionError("fixture bug: registry block not found")
    return result


def manifest_text(
    p7_gates: list[str] | None = None,
    gate_blocks: list[dict] | None = None,
    p7_status: str = "pending",
) -> str:
    # The default mirrors the REAL manifest's honest state since the
    # p7-cdda-user-supply unit (D223): the scaffold first, then the
    # gates proving the landed rows (p7-ci-artifacts proves
    # ci-artifacts-per-push + linux-native; p7-cdda-user-supply
    # proves cdda-user-supply), wired exactly like
    # docs/required-gates.toml.
    if p7_gates is None:
        p7_gates = [GATE_ID, "p7-ci-artifacts", "p7-cdda-user-supply"]
    if gate_blocks is None:
        gate_blocks = [
            {
                "id": GATE_ID,
                "timeout_seconds": 120,
                "commands": [
                    ["/usr/bin/python3", "tools/check-p7-ports-map.py"],
                    ["/usr/bin/python3", "tools/test-p7-ports-map.py"],
                ],
                "tracked_paths": [
                    DOC_RELATIVE,
                    "tools/check-p7-ports-map.py",
                    "tools/test-p7-ports-map.py",
                    "docs/required-gates.toml",
                ],
            },
            {
                "id": "p7-ci-artifacts",
                "timeout_seconds": 120,
                "commands": [
                    ["/usr/bin/python3", "tools/check-p7-ci-artifacts.py"],
                    ["/usr/bin/python3", "tools/check-p7-ports-map.py"],
                    ["/usr/bin/python3", "tools/test-p7-ci-artifacts.py"],
                ],
                "tracked_paths": [
                    ".github/workflows/ci.yml",
                    "tools/check-p7-ci-artifacts.py",
                    "tools/test-p7-ci-artifacts.py",
                    DOC_RELATIVE,
                    "docs/required-gates.toml",
                ],
            },
            {
                "id": "p7-cdda-user-supply",
                "timeout_seconds": 1800,
                "commands": [
                    [
                        "/usr/bin/cargo",
                        "test",
                        "--release",
                        "--locked",
                        "--offline",
                        "-p",
                        "bedlam-shell",
                        "--lib",
                    ],
                    ["/usr/bin/python3", "tools/check-p7-ports-map.py"],
                ],
                "tracked_paths": [
                    "engine/bedlam-shell/src/cdda.rs",
                    "engine/bedlam-shell/src/lib.rs",
                    "engine/bedlam-shell/src/window.rs",
                    "engine/bedlam-shell/src/main.rs",
                    DOC_RELATIVE,
                    "docs/required-gates.toml",
                ],
            },
        ]
    lines = ['schema = "required-gates-v1"']
    for number in range(8):
        gates = p7_gates if number == 7 else []
        rendered = ", ".join(f'"{gate}"' for gate in gates)
        status = p7_status if number == 7 else "green"
        lines += [
            "",
            "[[phase]]",
            f'id = "P{number}"',
            f'status = "{status}"',
            f"required_gates = [{rendered}]",
        ]
    for block in gate_blocks:
        lines += ["", "[[gate]]", f'id = "{block["id"]}"']
        if "timeout_seconds" in block:
            lines.append(f'timeout_seconds = {block["timeout_seconds"]}')
        commands = block.get(
            "commands",
            [[PYTHON, "tools/check-p7-ports-map.py"]],
        )
        lines.append(
            "commands = ["
            + ", ".join(
                "[" + ", ".join(f'"{part}"' for part in command) + "]"
                for command in commands
            )
            + "]"
        )
        tracked = block.get(
            "tracked_paths",
            [
                DOC_RELATIVE,
                "tools/check-p7-ports-map.py",
                "tools/test-p7-ports-map.py",
                "docs/required-gates.toml",
            ],
        )
        lines.append(
            "tracked_paths = ["
            + ", ".join(f'"{path}"' for path in tracked)
            + "]"
        )
    return "\n".join(lines) + "\n"


class PortsMapCheckerTests(unittest.TestCase):
    def run_checker(
        self,
        doc: str | None,
        manifest: str | None,
        *,
        with_doc: bool = True,
        with_manifest: bool = True,
    ) -> subprocess.CompletedProcess[bytes]:
        with tempfile.TemporaryDirectory(prefix="p7-ports-map-") as scratch:
            root = Path(scratch)
            (root / "docs").mkdir()
            (root / "tools").mkdir()
            if with_doc and doc is not None:
                (root / DOC_RELATIVE).write_text(doc, encoding="utf-8")
            if with_manifest and manifest is not None:
                (root / "docs" / "required-gates.toml").write_text(
                    manifest, encoding="utf-8"
                )
            # Fixtures invoke the real checker by path; --root resolution
            # needs no other repository content (the checker reads only
            # the two docs under test).
            return subprocess.run(
                [PYTHON, str(CHECKER), "--root", str(root)],
                capture_output=True,
                timeout=120,
            )

    def honest_doc(self) -> str:
        return (REPO / DOC_RELATIVE).read_text(encoding="utf-8")

    # ---- the honest state passes --------------------------------------

    def test_honest_fixture_passes(self) -> None:
        result = self.run_checker(self.honest_doc(), manifest_text())
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-ports-map: OK", result.stdout)

    def test_real_repo_passes(self) -> None:
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(REPO)],
            capture_output=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-ports-map: OK", result.stdout)
        # Deliberate re-baseline with the p7-cdda-user-supply unit
        # (D223): the three landed rows (ci-artifacts-per-push +
        # linux-native on p7-ci-artifacts, cdda-user-supply on its
        # own gate) leave four engineering rows pending.
        self.assertIn(b"7 engineering (3 landed, 4 pending)", result.stdout)
        self.assertIn(
            b"landed: cdda-user-supply (gate p7-cdda-user-supply),"
            b" ci-artifacts-per-push (gate p7-ci-artifacts),"
            b" linux-native (gate p7-ci-artifacts)",
            result.stdout,
        )
        self.assertIn(b"3 recorded exclusions", result.stdout)

    # ---- structural doc rules -----------------------------------------

    def test_missing_doc_fails(self) -> None:
        result = self.run_checker(None, manifest_text(), with_doc=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"contract doc is missing", result.stderr)

    def test_missing_section_fails(self) -> None:
        doc = self.honest_doc().replace(
            "## 5. The SteamDeck stretch default",
            "## 5. TAMPERED header",
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing section", result.stderr)

    def test_tampered_plan_surface_sentence_fails(self) -> None:
        doc = tamper(
            self.honest_doc(),
            "Linux native + Flatpak; Windows installer; macOS universal2"
            " through automated CI.",
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the required rule sentence", result.stderr)

    def test_tampered_external_conditions_sentence_fails(self) -> None:
        doc = tamper(
            self.honest_doc(),
            "Runner, signing, and publication availability are external"
            " conditions and do not block engineering completion.",
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the required rule sentence", result.stderr)

    def test_tampered_cdda_boundary_fails(self) -> None:
        doc = tamper(self.honest_doc(), "never redistributed")
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the required rule sentence", result.stderr)

    def test_tampered_unit_bounds_fails(self) -> None:
        doc = tamper(
            self.honest_doc(),
            "no engine change and no packaging build lands in this unit",
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the required rule sentence", result.stderr)

    def test_tampered_phase_close_sentence_fails(self) -> None:
        doc = tamper(
            self.honest_doc(),
            "P7 status stays pending until every engineering deliverable"
            " is landed",
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the required rule sentence", result.stderr)

    # ---- registry discipline -------------------------------------------

    def test_missing_registry_fails(self) -> None:
        doc = TOML_BLOCK.sub(
            lambda match: (
                "```toml\nschema = \"something-else-v1\"\n```"
                if REGISTRY_MARK in match.group(2)
                else match.group(0)
            ),
            self.honest_doc(),
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no fenced toml registry", result.stderr)

    def test_two_registries_fails(self) -> None:
        doc = self.honest_doc() + (
            "\n```toml\nschema = \"p7-ports-map-v1\"\n"
            "[[deliverable]]\nid = \"x\"\nkind = \"engineering\"\n"
            "plan_anchor = \"p\"\nstatus = \"pending\"\ngate = \"\"\n```\n"
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"blocks (want 1)", result.stderr)

    def test_registry_row_unknown_key_fails(self) -> None:
        doc = with_registry(
            self.honest_doc(),
            'schema = "p7-ports-map-v1"\n\n[[deliverable]]\n'
            'id = "linux-native"\nkind = "engineering"\n'
            'plan_anchor = "Linux native + Flatpak"\n'
            'status = "pending"\ngate = ""\nmystery = true\n',
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unknown keys", result.stderr)

    def test_registry_duplicate_id_fails(self) -> None:
        row = (
            "[[deliverable]]\nid = \"linux-native\"\nkind = \"engineering\"\n"
            "plan_anchor = \"Linux native + Flatpak\"\n"
            "status = \"pending\"\ngate = \"\"\n"
        )
        doc = with_registry(
            self.honest_doc(), f'schema = "p7-ports-map-v1"\n\n{row}{row}'
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"duplicate deliverable id", result.stderr)

    def test_missing_engineering_deliverable_fails(self) -> None:
        doc = with_registry(
            self.honest_doc(),
            'schema = "p7-ports-map-v1"\n\n[[deliverable]]\n'
            'id = "flatpak-manifest"\nkind = "engineering"\n'
            'plan_anchor = "Linux native + Flatpak"\n'
            'status = "pending"\ngate = ""\n',
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the engineering deliverables", result.stderr)
        self.assertIn(b"ci-artifacts-per-push", result.stderr)

    def test_extra_engineering_deliverable_fails(self) -> None:
        doc = append_registry(
            self.honest_doc(),
            "\n[[deliverable]]\n"
            'id = "spontaneous-deliverable"\nkind = "engineering"\n'
            'plan_anchor = "nowhere in the plan"\n'
            'status = "pending"\ngate = ""\n',
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"outside the decided P7 scope", result.stderr)

    def test_missing_external_exclusion_fails(self) -> None:
        # Drop the signing-keys exclusion row only.
        doc = re.sub(
            r"\[\[deliverable\]\]\nid = \"signing-keys\".*?(?=\[\[deliverable\]\]|\Z)",
            "",
            self.honest_doc(),
            flags=re.DOTALL,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the recorded exclusions", result.stderr)
        self.assertIn(b"signing-keys", result.stderr)

    def test_wrong_kind_on_required_id_fails(self) -> None:
        doc = with_registry(
            self.honest_doc(),
            'schema = "p7-ports-map-v1"\n\n[[deliverable]]\n'
            'id = "windows-installer"\nkind = "external-conditional"\n'
            'plan_anchor = "Windows installer"\n'
            'note = "engineering work masquerading as an exclusion"\n',
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the engineering deliverables", result.stderr)

    # ---- evidence + exclusion discipline ---------------------------------

    def test_landed_without_gate_fails(self) -> None:
        doc = with_registry(
            self.honest_doc(),
            'schema = "p7-ports-map-v1"\n\n[[deliverable]]\n'
            'id = "steamdeck-default"\nkind = "engineering"\n'
            'plan_anchor = "SteamDeck defaults stretch"\n'
            'status = "landed"\ngate = ""\n',
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"landed but names no proving gate", result.stderr)

    def test_pending_with_gate_fails(self) -> None:
        doc = with_registry(
            self.honest_doc(),
            'schema = "p7-ports-map-v1"\n\n[[deliverable]]\n'
            'id = "steamdeck-default"\nkind = "engineering"\n'
            'plan_anchor = "SteamDeck defaults stretch"\n'
            'status = "pending"\ngate = "p7-steamdeck-default"\n',
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"is pending but carries gate", result.stderr)

    def test_external_row_with_status_fails(self) -> None:
        doc = with_registry(
            self.honest_doc(),
            'schema = "p7-ports-map-v1"\n\n[[deliverable]]\n'
            'id = "signing-keys"\nkind = "external-conditional"\n'
            'plan_anchor = "Runner, signing, and publication availability'
            ' are external conditions"\n'
            'status = "pending"\nnote = "an exclusion cannot be landed"\n',
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must not carry status", result.stderr)

    def test_external_row_without_note_fails(self) -> None:
        doc = with_registry(
            self.honest_doc(),
            'schema = "p7-ports-map-v1"\n\n[[deliverable]]\n'
            'id = "signing-keys"\nkind = "external-conditional"\n'
            'plan_anchor = "Runner, signing, and publication availability'
            ' are external conditions"\n',
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"recorded exclusion note", result.stderr)

    # ---- the gate join + manifest wiring ---------------------------------

    def test_landed_gate_not_defined_fails(self) -> None:
        doc = land_steamdeck(self.honest_doc(), "p7-steamdeck-default")
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no [[gate]] with that id is defined", result.stderr)

    def test_landed_gate_not_in_phase_list_fails(self) -> None:
        doc = land_steamdeck(self.honest_doc(), "p7-steamdeck-default")
        # The honest doc already lands ci-artifacts-per-push +
        # linux-native on p7-ci-artifacts and cdda-user-supply on its
        # own gate, so the fixture wires BOTH in (defined + listed)
        # to reach the steamdeck-specific failure:
        # p7-steamdeck-default is defined but NOT in the list.
        manifest = manifest_text(
            p7_gates=[GATE_ID, "p7-ci-artifacts", "p7-cdda-user-supply"],
            gate_blocks=[
                {
                    "id": "p7-steamdeck-default",
                    "commands": [["/usr/bin/true"]],
                    "tracked_paths": [DOC_RELATIVE],
                },
                {
                    "id": "p7-ci-artifacts",
                    "commands": [["/usr/bin/true"]],
                    "tracked_paths": [".github/workflows/ci.yml"],
                },
                {
                    "id": "p7-cdda-user-supply",
                    "commands": [["/usr/bin/true"]],
                    "tracked_paths": ["engine/bedlam-shell/src/cdda.rs"],
                },
                {
                    "id": GATE_ID,
                    "commands": [
                        ["/usr/bin/python3", "tools/check-p7-ports-map.py"]
                    ],
                    "tracked_paths": [
                        DOC_RELATIVE,
                        "tools/check-p7-ports-map.py",
                        "tools/test-p7-ports-map.py",
                        "docs/required-gates.toml",
                    ],
                },
            ],
        )
        result = self.run_checker(doc, manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not in the P7 required_gates list", result.stderr)

    def test_scaffold_not_first_fails(self) -> None:
        manifest = manifest_text(p7_gates=["some-other-gate", GATE_ID])
        result = self.run_checker(self.honest_doc(), manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must start with p7-ports-scaffold", result.stderr)

    def test_scaffold_gate_block_missing_fails(self) -> None:
        manifest = manifest_text(
            p7_gates=[GATE_ID],
            gate_blocks=[
                {
                    "id": "not-the-scaffold",
                    "commands": [["/usr/bin/true"]],
                    "tracked_paths": [DOC_RELATIVE],
                }
            ],
        )
        result = self.run_checker(self.honest_doc(), manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no [[gate]] with that id is defined", result.stderr)

    def test_scaffold_commands_do_not_run_checker_fails(self) -> None:
        manifest = manifest_text(
            gate_blocks=[
                {
                    "id": GATE_ID,
                    "commands": [["/usr/bin/python3", "tools/someone-else.py"]],
                    "tracked_paths": [
                        DOC_RELATIVE,
                        "tools/check-p7-ports-map.py",
                        "tools/test-p7-ports-map.py",
                        "docs/required-gates.toml",
                    ],
                }
            ]
        )
        result = self.run_checker(self.honest_doc(), manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"do not run tools/check-p7-ports-map.py", result.stderr)

    def test_scaffold_tracked_paths_missing_doc_fails(self) -> None:
        manifest = manifest_text(
            gate_blocks=[
                {
                    "id": GATE_ID,
                    "commands": [
                        ["/usr/bin/python3", "tools/check-p7-ports-map.py"]
                    ],
                    "tracked_paths": [
                        "tools/check-p7-ports-map.py",
                        "tools/test-p7-ports-map.py",
                        "docs/required-gates.toml",
                    ],
                }
            ]
        )
        result = self.run_checker(self.honest_doc(), manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"tracked_paths do not include docs/P7-PORTS.md", result.stderr)

    # ---- phase-close consistency -----------------------------------------

    def test_premature_phase_flip_fails(self) -> None:
        manifest = manifest_text(p7_status="green")
        result = self.run_checker(self.honest_doc(), manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"status is green but", result.stderr)
        self.assertIn(b"engineering", result.stderr)

    # ---- the forward shape is legal ---------------------------------------

    def test_landed_deliverable_with_wired_gate_passes(self) -> None:
        doc = land_steamdeck(self.honest_doc(), "p7-steamdeck-default")
        # The honest doc already lands three rows (two on
        # p7-ci-artifacts, cdda-user-supply on its own gate); the
        # fixture wires all four gates so the flipped steamdeck row
        # reaches its own proving gate: 4 landed, 3 pending.
        manifest = manifest_text(
            p7_gates=[
                GATE_ID,
                "p7-ci-artifacts",
                "p7-cdda-user-supply",
                "p7-steamdeck-default",
            ],
            gate_blocks=[
                {
                    "id": GATE_ID,
                    "commands": [
                        ["/usr/bin/python3", "tools/check-p7-ports-map.py"],
                        ["/usr/bin/python3", "tools/test-p7-ports-map.py"],
                    ],
                    "tracked_paths": [
                        DOC_RELATIVE,
                        "tools/check-p7-ports-map.py",
                        "tools/test-p7-ports-map.py",
                        "docs/required-gates.toml",
                    ],
                },
                {
                    "id": "p7-ci-artifacts",
                    "commands": [["/usr/bin/python3", "tools/check-p7-ci-artifacts.py"]],
                    "tracked_paths": [".github/workflows/ci.yml"],
                },
                {
                    "id": "p7-cdda-user-supply",
                    "commands": [["/usr/bin/true"]],
                    "tracked_paths": ["engine/bedlam-shell/src/cdda.rs"],
                },
                {
                    "id": "p7-steamdeck-default",
                    "commands": [["/usr/bin/true"]],
                    "tracked_paths": ["docs/P7-PORTS.md"],
                },
            ],
        )
        result = self.run_checker(doc, manifest)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"4 landed, 3 pending", result.stdout)


if __name__ == "__main__":
    unittest.main()
