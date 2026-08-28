#!/usr/bin/env python3
"""Hermetic fail-closed contracts for check-p7-ci-artifacts.py.

Every structural rule of the checker is proven to FAIL LOUDLY on the
specific tampering it guards against, and to pass on the honest
landed state. Two tests run the checker against the REAL repository
workflow (the same thing the gate runs), pinning the honest state and
its summary; one proves a MINIMAL synthetic workflow carrying exactly
the contracted surface passes (the rules, not incidental file
content); one proves an upload step in the WRONG job does not satisfy
the deliverable (the uploads must live in the release-matrix job).
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("check-p7-ci-artifacts.py")
REPO = CHECKER.parent.parent
PYTHON = sys.executable or "/usr/bin/python3"
WORKFLOW_RELATIVE = ".github/workflows/ci.yml"

MINIMAL_HONEST = """\
name: ci
on:
  push:
jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - run: cargo build --release --workspace
      - name: upload linux release binary
        if: runner.os == 'Linux'
        uses: actions/upload-artifact@v4
        with:
          name: bedlam-shell-linux-x86_64
          path: target/release/bedlam-shell
          if-no-files-found: error
      - name: upload windows release binary
        if: runner.os == 'Windows'
        uses: actions/upload-artifact@v4
        with:
          name: bedlam-shell-windows-x86_64
          path: target/release/bedlam-shell.exe
          if-no-files-found: error
"""

# Same surface, but the upload steps live OUTSIDE the release-matrix
# job: the artifacts would not be produced by the matrix legs, so the
# checker must refuse it.
UPLOADS_IN_WRONG_JOB = """\
name: ci
on:
  push:
jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - run: cargo build --release --workspace
  artifacts:
    runs-on: ubuntu-latest
    steps:
      - name: upload linux release binary
        if: runner.os == 'Linux'
        uses: actions/upload-artifact@v4
        with:
          name: bedlam-shell-linux-x86_64
          path: target/release/bedlam-shell
          if-no-files-found: error
"""


def drop_step(doc: str, needle: str) -> str:
    """Remove the whole list-item block whose first line carries needle."""
    out: list[str] = []
    skipping = False
    step_indent = 0
    for line in doc.splitlines(keepends=True):
        indent = len(line) - len(line.lstrip(" "))
        if not skipping:
            stripped = line.strip()
            if stripped.startswith("- ") and needle in stripped:
                skipping = True
                step_indent = indent
                continue
            out.append(line)
        else:
            if line.strip() == "":
                continue
            if indent > step_indent:
                continue
            skipping = False
            out.append(line)
    result = "".join(out)
    if result == doc:
        raise AssertionError(f"fixture bug: step not found: {needle!r}")
    return result


def drop_line(doc: str, exact: str) -> str:
    result = doc.replace(f"{exact}\n", "", 1)
    if result == doc:
        raise AssertionError(f"fixture bug: line not found: {exact!r}")
    return result


class CiArtifactsCheckerTests(unittest.TestCase):
    def run_checker(
        self,
        doc: str | None,
        *,
        with_workflow: bool = True,
    ) -> subprocess.CompletedProcess[bytes]:
        with tempfile.TemporaryDirectory(prefix="p7-ci-artifacts-") as scratch:
            root = Path(scratch)
            (root / ".github" / "workflows").mkdir(parents=True)
            if with_workflow and doc is not None:
                (root / WORKFLOW_RELATIVE).write_text(doc, encoding="utf-8")
            # Fixtures invoke the real checker by path; --root resolution
            # needs no other repository content (the checker reads only
            # the workflow under test).
            return subprocess.run(
                [PYTHON, str(CHECKER), "--root", str(root)],
                capture_output=True,
                timeout=120,
            )

    def honest_workflow(self) -> str:
        return (REPO / WORKFLOW_RELATIVE).read_text(encoding="utf-8")

    # ---- the honest state passes ----------------------------------------

    def test_real_repo_passes(self) -> None:
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(REPO)],
            capture_output=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-ci-artifacts: OK", result.stdout)
        self.assertIn(b"trigger: push", result.stdout)
        self.assertIn(b"ubuntu-latest + windows-latest", result.stdout)
        self.assertIn(b"target/release/bedlam-shell.exe", result.stdout)
        self.assertIn(b"signing material: none", result.stdout)

    def test_honest_fixture_passes(self) -> None:
        result = self.run_checker(self.honest_workflow())
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-ci-artifacts: OK", result.stdout)

    def test_minimal_honest_fixture_passes(self) -> None:
        result = self.run_checker(MINIMAL_HONEST)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-ci-artifacts: OK", result.stdout)

    # ---- file + parse discipline ----------------------------------------

    def test_missing_workflow_fails(self) -> None:
        result = self.run_checker(None, with_workflow=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"workflow file is missing", result.stderr)

    def test_tab_indentation_fails(self) -> None:
        doc = self.honest_workflow().replace("\njobs:", "\n\tjobs:", 1)
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"tab in indentation", result.stderr)

    def test_unterminated_flow_sequence_fails(self) -> None:
        doc = self.honest_workflow().replace(
            "os: [ubuntu-latest, windows-latest]",
            "os: [ubuntu-latest, windows-latest",
            1,
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unterminated flow sequence", result.stderr)

    def test_unparsable_line_fails(self) -> None:
        doc = self.honest_workflow().replace("jobs:", "::::", 1)
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unparsable mapping line", result.stderr)

    def test_unexpected_deeper_line_fails(self) -> None:
        doc = self.honest_workflow() + " stray: line\n"
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            b"unexpected deeper line where a key was due", result.stderr
        )

    # ---- the per-push trigger --------------------------------------------

    def test_push_trigger_removed_fails(self) -> None:
        doc = drop_line(self.honest_workflow(), "  push:")
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must carry a top-level push trigger", result.stderr)

    # ---- the release matrix ----------------------------------------------

    def test_matrix_leg_removed_fails(self) -> None:
        doc = self.honest_workflow().replace(
            "os: [ubuntu-latest, windows-latest]",
            "os: [ubuntu-latest]",
            1,
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no job builds the release binary", result.stderr)
        self.assertIn(b"ubuntu-latest + windows-latest", result.stderr)

    def test_release_build_step_removed_fails(self) -> None:
        doc = drop_step(
            self.honest_workflow(), "- run: cargo build --release --workspace"
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no job builds the release binary", result.stderr)

    # ---- the artifact uploads ---------------------------------------------

    def test_linux_upload_step_removed_fails(self) -> None:
        doc = drop_step(
            self.honest_workflow(),
            "- name: upload linux release binary (per-push artifact)",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            b"no actions/upload-artifact@v4 step gated on"
            b" runner.os == 'Linux'",
            result.stderr,
        )

    def test_windows_upload_step_removed_fails(self) -> None:
        doc = drop_step(
            self.honest_workflow(),
            "- name: upload windows release binary (per-push artifact)",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            b"no actions/upload-artifact@v4 step gated on"
            b" runner.os == 'Windows'",
            result.stderr,
        )

    def test_uploads_in_wrong_job_fails(self) -> None:
        result = self.run_checker(UPLOADS_IN_WRONG_JOB)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            b"no actions/upload-artifact@v4 step gated on"
            b" runner.os == 'Linux'",
            result.stderr,
        )

    def test_upload_action_downgraded_fails(self) -> None:
        doc = self.honest_workflow().replace(
            "actions/upload-artifact@v4",
            "actions/upload-artifact@v3",
            1,
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"step gated on runner.os == 'Linux'", result.stderr)

    def test_upload_gating_removed_fails(self) -> None:
        # Anchored on the upload step's own header line: the alsa step
        # carries an identical `if:` line that must stay untouched.
        doc = self.honest_workflow().replace(
            "(per-push artifact)\n        if: runner.os == 'Linux'",
            "(per-push artifact)\n",
            1,
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"step gated on runner.os == 'Linux'", result.stderr)

    def test_artifact_path_tampered_fails(self) -> None:
        doc = self.honest_workflow().replace(
            "path: target/release/bedlam-shell\n",
            "path: target/release/bedlam-shell.bin\n",
            1,
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"path must be exactly", result.stderr)
        self.assertIn(b"target/release/bedlam-shell'", result.stderr)

    def test_artifact_name_removed_fails(self) -> None:
        doc = drop_line(
            self.honest_workflow(), "          name: bedlam-shell-linux-x86_64"
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no artifact name", result.stderr)

    def test_if_no_files_found_relaxed_fails(self) -> None:
        doc = self.honest_workflow().replace(
            "if-no-files-found: error",
            "if-no-files-found: warn",
            1,
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"if-no-files-found: error", result.stderr)
        self.assertIn(b"found 'warn'", result.stderr)

    # ---- no signing material ----------------------------------------------

    def test_secret_reference_fails(self) -> None:
        doc = self.honest_workflow().replace(
            "          MIRIFLAGS: -Zmiri-isolation-error=warn\n",
            "          MIRIFLAGS: -Zmiri-isolation-error=warn\n"
            "          LEAK: ${{ secrets.SIGNING_KEY }}\n",
            1,
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)
        self.assertIn(b"secrets", result.stderr)

    def test_signing_tool_invocation_fails(self) -> None:
        doc = self.honest_workflow().replace(
            "      - run: cargo build --release --workspace\n",
            "      - run: cargo build --release --workspace\n"
            "      - run: signtool sign /fd SHA256"
            " target/release/bedlam-shell.exe\n",
            1,
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)
        self.assertIn(b"signtool", result.stderr)

    def test_signing_comment_fails(self) -> None:
        # The denylist includes comments by design: the workflow file
        # must carry no signing vocabulary at all.
        doc = self.honest_workflow() + "      # then we Authenticode it\n"
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)


if __name__ == "__main__":
    unittest.main()
