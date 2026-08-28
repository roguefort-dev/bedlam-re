#!/usr/bin/python3
"""Hermetic fail-closed contracts for check-p6-hd-asset-research.py.

Every structural rule of the checker is proven to FAIL LOUDLY on the specific
tampering it guards against, and to pass on the honest research state (the
committed doc + a correctly wired manifest). One test also runs the checker
against the REAL repository doc + manifest (the same thing the gate runs),
pinning the honest state.
"""

from __future__ import annotations

import datetime
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("check-p6-hd-asset-research.py")
REPO = CHECKER.parent.parent
PYTHON = sys.executable or "/usr/bin/python3"
DOC_RELATIVE = "docs/RESEARCH-HD-ASSET-PIPELINE.md"
GATE_ID = "p6-hd-asset-research"
TOMORROW = (
    datetime.datetime.now(datetime.timezone.utc).date() + datetime.timedelta(days=1)
).isoformat()


def flat(text: str) -> str:
    """Whitespace-collapse like the checker's normalize_ws."""
    return " ".join(text.split())


def tamper(doc: str, sentence: str) -> str:
    """Remove every whitespace-flexible occurrence of a rule sentence.

    The checker matches sentences after whitespace normalization, so simply
    replacing the single-line spelling can leave a wrapped copy behind. This
    builds a newline-tolerant pattern and removes ALL occurrences.
    """
    pattern = re.compile(r"\s+".join(re.escape(word) for word in sentence.split()))
    return pattern.sub("TAMPERED-SENTENCE", doc)


def manifest_text(
    p6_gates: list[str] | None = None,
    with_gate_block: bool = True,
    gate_commands: list[list[str]] | None = None,
    tracked_paths: list[str] | None = None,
) -> str:
    if p6_gates is None:
        p6_gates = [GATE_ID]
    lines = ['schema = "required-gates-v1"']
    for number in range(8):
        gates = p6_gates if number == 6 else []
        rendered = ", ".join(f'"{gate}"' for gate in gates)
        lines += [
            "",
            "[[phase]]",
            f'id = "P{number}"',
            'status = "pending"',
            f"required_gates = [{rendered}]",
        ]
    if with_gate_block:
        if gate_commands is None:
            gate_commands = [
                ["/usr/bin/python3", "tools/check-p6-hd-asset-research.py"],
                ["/usr/bin/python3", "tools/test-p6-hd-asset-research.py"],
            ]
        if tracked_paths is None:
            tracked_paths = [
                DOC_RELATIVE,
                "tools/check-p6-hd-asset-research.py",
                "tools/test-p6-hd-asset-research.py",
                "docs/required-gates.toml",
            ]
        lines += [
            "",
            "[[gate]]",
            f'id = "{GATE_ID}"',
            "timeout_seconds = 120",
            "commands = ["
            + ", ".join(
                "[" + ", ".join(f'"{part}"' for part in command) + "]"
                for command in gate_commands
            )
            + "]",
            "tracked_paths = ["
            + ", ".join(f'"{path}"' for path in tracked_paths)
            + "]",
        ]
    return "\n".join(lines) + "\n"


class ResearchCheckerTests(unittest.TestCase):
    def run_checker(
        self,
        doc: str | None,
        manifest: str | None,
        *,
        with_doc: bool = True,
        with_manifest: bool = True,
    ) -> subprocess.CompletedProcess[bytes]:
        with tempfile.TemporaryDirectory(prefix="p6-hd-asset-research-") as scratch:
            root = Path(scratch)
            (root / "docs").mkdir()
            (root / "tools").mkdir()
            if with_doc and doc is not None:
                (root / DOC_RELATIVE).write_text(doc, encoding="utf-8")
            if with_manifest and manifest is not None:
                (root / "docs" / "required-gates.toml").write_text(
                    manifest, encoding="utf-8"
                )
            # The checker script itself must exist for --root resolution only
            # in the real repo; fixtures invoke the real checker by path.
            return subprocess.run(
                [PYTHON, str(CHECKER), "--root", str(root)],
                capture_output=True,
                timeout=120,
            )

    def honest_doc(self) -> str:
        return (REPO / DOC_RELATIVE).read_text(encoding="utf-8")

    # ---- the honest state passes -------------------------------------

    def test_honest_fixture_passes(self) -> None:
        result = self.run_checker(self.honest_doc(), manifest_text())
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p6-hd-asset-research: OK", result.stdout)

    def test_real_repo_passes(self) -> None:
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(REPO)],
            capture_output=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p6-hd-asset-research: OK", result.stdout)

    # ---- structural doc rules ----------------------------------------

    def test_missing_doc_fails(self) -> None:
        result = self.run_checker(None, manifest_text(), with_doc=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"research doc is missing", result.stderr)

    def test_missing_section_fails(self) -> None:
        doc = self.honest_doc().replace(
            "## 7. Automated gate criteria design", "## 7. Gates", 1
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing section", result.stderr)

    def test_missing_category_section_fails(self) -> None:
        doc = self.honest_doc().replace(
            "### 5.D (d) Portraits / UI art", "### 5.D Portraits", 1
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing section", result.stderr)

    def test_missing_boundary_sentence_fails(self) -> None:
        doc = tamper(
            self.honest_doc(),
            "outputs without recorded provenance are excluded from shipping",
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"required rule sentence", result.stderr)

    def test_missing_fallback_sentence_fails(self) -> None:
        doc = tamper(self.honest_doc(), "falls back to the original asset")
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"required rule sentence", result.stderr)

    def test_missing_engine_render_sentence_fails(self) -> None:
        doc = tamper(
            self.honest_doc(),
            "engine renders all text, controls, click targets and gameplay"
            " information",
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"required rule sentence", result.stderr)

    # ---- pin registry rules ------------------------------------------

    def test_missing_pin_registry_fails(self) -> None:
        doc = self.honest_doc().replace('schema = "hd-asset-pins-v1"', 'schema = "x"', 1)
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no fenced toml pin registry", result.stderr)

    def test_duplicate_pin_id_fails(self) -> None:
        doc = self.honest_doc().replace(
            'id = "swinir-real-sr-m-x4"', 'id = "real-esrgan-x2plus"', 1
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"duplicate pin id", result.stderr)

    def test_model_pin_without_license_fails(self) -> None:
        doc = self.honest_doc().replace(
            'license = "Apache-2.0"\nretrieved = "2026-08-28"\nnote = "checkpoint'
            ' filename pinned',
            'retrieved = "2026-08-28"\nnote = "checkpoint filename pinned',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must carry a license", result.stderr)

    def test_unverified_license_primary_fails(self) -> None:
        doc = self.honest_doc().replace(
            'license = "FLUX.1 dev Non-Commercial License (gated access)"',
            'license = "unverified (nobody looked)"',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"PRIMARY model pin", result.stderr)

    def test_non_first_party_url_fails(self) -> None:
        doc = self.honest_doc().replace(
            "https://huggingface.co/black-forest-labs/FLUX.1-Fill-dev",
            "https://example.com/flux",
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not a first-party source", result.stderr)

    def test_future_retrieved_fails(self) -> None:
        doc = self.honest_doc().replace(
            'id = "flux-1-fill-dev"',
            f'id = "flux-1-fill-dev-recheck"',
        ).replace(
            'retrieved = "2026-08-28"\nnote = "model card documents',
            f'retrieved = "{TOMORROW}"\nnote = "model card documents',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"in the future", result.stderr)

    def test_stale_retrieved_fails(self) -> None:
        doc = self.honest_doc().replace(
            'retrieved = "2026-08-28"\nnote = "model card documents',
            'retrieved = "2025-01-01"\nnote = "model card documents',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"predates the verification window", result.stderr)

    def test_unknown_pin_key_fails(self) -> None:
        doc = self.honest_doc().replace(
            '[[pin]]\nid = "flux-1-fill-dev"',
            '[[pin]]\nverified_by = "trust me"\nid = "flux-1-fill-dev"',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unknown keys", result.stderr)

    def test_deferred_without_note_fails(self) -> None:
        doc = self.honest_doc().replace(
            '\nnote = "non-commercial license excludes it from any distributable'
            ' HD pack; surveyed for completeness"',
            "",
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"deferred and must carry a note", result.stderr)

    def test_model_without_categories_fails(self) -> None:
        doc = self.honest_doc().replace(
            'role = "primary"\ncategories = ["tile-texture-upscale"]',
            'role = "primary"',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must declare its workflow categories", result.stderr)

    def test_tool_with_categories_fails(self) -> None:
        doc = self.honest_doc().replace(
            'id = "comfyui"\nkind = "tool"',
            'id = "comfyui"\nkind = "tool"\ncategories = ["sprite-upscale"]',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must not carry categories", result.stderr)

    # ---- coverage rules ----------------------------------------------

    def test_category_without_primary_fails(self) -> None:
        # Demote BOTH sprite-upscale primaries (x2plus and anime_6B).
        doc = self.honest_doc().replace(
            'role = "primary"\ncategories = ["sprite-upscale",'
            ' "tile-texture-upscale", "portrait-ui"]\nversion = "v0.2.1 release'
            ' weights"',
            'role = "fallback"\ncategories = ["tile-texture-upscale",'
            ' "portrait-ui"]\nversion = "v0.2.1 release weights"',
            1,
        ).replace(
            'role = "primary"\ncategories = ["sprite-upscale", "portrait-ui"]',
            'role = "fallback"\ncategories = ["portrait-ui"]',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            b"category 'sprite-upscale' has no primary model pin", result.stderr
        )

    def test_outpaint_without_fallback_fails(self) -> None:
        doc = self.honest_doc().replace(
            'id = "sd2-inpainting"\nkind = "model"\nrole = "fallback"',
            'id = "sd2-inpainting"\nkind = "model"\nrole = "deferred"',
            1,
        ).replace(
            'id = "sdxl-base-1.0"\nkind = "model"\nrole = "fallback"',
            'id = "sdxl-base-1.0"\nkind = "model"\nrole = "deferred"',
            1,
        )
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no fallback model pin", result.stderr)

    def test_missing_comfy_cli_pin_fails(self) -> None:
        doc = self.honest_doc().replace('id = "comfy-cli"', 'id = "cli-thing"', 1)
        result = self.run_checker(doc, manifest_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"primary comfy-cli tool pin", result.stderr)

    # ---- manifest wiring rules ---------------------------------------

    def test_gate_missing_from_phase_list_fails(self) -> None:
        result = self.run_checker(
            self.honest_doc(), manifest_text(p6_gates=["some-other-gate"])
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"does not include p6-hd-asset-research", result.stderr)

    def test_gate_block_missing_fails(self) -> None:
        result = self.run_checker(
            self.honest_doc(), manifest_text(with_gate_block=False)
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no [[gate]] with that id", result.stderr)

    def test_gate_not_running_checker_fails(self) -> None:
        result = self.run_checker(
            self.honest_doc(),
            manifest_text(
                gate_commands=[["/usr/bin/python3", "tools/other-checker.py"]]
            ),
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"do not run tools/check-p6-hd-asset-research.py", result.stderr)

    def test_gate_not_tracking_doc_fails(self) -> None:
        result = self.run_checker(
            self.honest_doc(),
            manifest_text(tracked_paths=["tools/check-p6-hd-asset-research.py"]),
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"tracked_paths do not include", result.stderr)

    def test_missing_manifest_fails(self) -> None:
        result = self.run_checker(self.honest_doc(), None, with_manifest=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"required-gates manifest is missing", result.stderr)


if __name__ == "__main__":
    unittest.main()
