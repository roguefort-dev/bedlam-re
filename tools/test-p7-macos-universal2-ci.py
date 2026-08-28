#!/usr/bin/env python3
"""Hermetic fail-closed contracts for check-p7-macos-universal2-ci.py.

Every structural rule of the checker is proven to FAIL LOUDLY on the
specific tampering it guards against, and to pass on the honest
landed state. Three tests run the checker against the REAL repository
workflow (the same thing the gate runs), pinning the honest state and
its summary; one proves a MINIMAL synthetic workflow carrying exactly
the contracted surface passes (the rules, not incidental file
content); the rest prove the refusals: the parse disciplines, the
scheduled-cadence rules (a push or pull_request trigger is itself a
failure -- no push may ever be gated on a macOS runner existing), the
universal2 build/join/upload pins, the no-test boundary (goldens
never run on macOS CI), and the denylists.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("check-p7-macos-universal2-ci.py")
REPO = CHECKER.parent.parent
PYTHON = sys.executable or "/usr/bin/python3"
WORKFLOW_RELATIVE = ".github/workflows/macos-universal2.yml"

MINIMAL_HONEST = """\
name: macos-universal2
on:
  schedule:
    - cron: "41 4 * * 1"
  workflow_dispatch:
permissions:
  contents: read
jobs:
  macos-universal2:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin
      - run: cargo build --release --locked -p bedlam-shell --target aarch64-apple-darwin
      - run: cargo build --release --locked -p bedlam-shell --target x86_64-apple-darwin
      - name: join
        run: |
          mkdir -p staging
          lipo -create -output staging/bedlam-shell target/aarch64-apple-darwin/release/bedlam-shell target/x86_64-apple-darwin/release/bedlam-shell
      - name: upload
        uses: actions/upload-artifact@v4
        with:
          name: bedlam-shell-macos-universal2
          path: staging/bedlam-shell
          if-no-files-found: error
          retention-days: 14
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


def replace_once(doc: str, old: str, new: str) -> str:
    result = doc.replace(old, new, 1)
    if result == doc:
        raise AssertionError(f"fixture bug: text not found: {old!r}")
    return result


class MacosUniversal2CheckerTests(unittest.TestCase):
    def run_checker(
        self,
        doc: str | None,
        *,
        with_workflow: bool = True,
    ) -> subprocess.CompletedProcess[bytes]:
        with tempfile.TemporaryDirectory(prefix="p7-macos-u2-") as scratch:
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
        self.assertIn(b"p7-macos-universal2-ci: OK", result.stdout)
        self.assertIn(b"trigger: scheduled (cron '41 4 * * 1')", result.stdout)
        self.assertIn(
            b"no push/pull_request trigger (PLAN sec 3 posture", result.stdout
        )
        self.assertIn(
            b"aarch64-apple-darwin + x86_64-apple-darwin -> lipo -create",
            result.stdout,
        )
        self.assertIn(b"bedlam-shell-macos-universal2 <- staging/bedlam-shell", result.stdout)
        self.assertIn(b"tests: none in the job", result.stdout)
        self.assertIn(b"signing material: none", result.stdout)

    def test_honest_fixture_passes(self) -> None:
        result = self.run_checker(self.honest_workflow())
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-macos-universal2-ci: OK", result.stdout)

    def test_minimal_honest_fixture_passes(self) -> None:
        result = self.run_checker(MINIMAL_HONEST)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-macos-universal2-ci: OK", result.stdout)

    # ---- file + parse discipline ----------------------------------------

    def test_missing_workflow_fails(self) -> None:
        result = self.run_checker(None, with_workflow=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"workflow file is missing", result.stderr)

    def test_tab_indentation_fails(self) -> None:
        doc = self.honest_workflow().replace("\npermissions:", "\n\tpermissions:", 1)
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"tab in indentation", result.stderr)

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

    # ---- the scheduled cadence (PLAN sec 3 posture) ----------------------

    def test_schedule_trigger_removed_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "on:\n  schedule:\n    - cron: \"41 4 * * 1\"\n"
            "  workflow_dispatch:\n",
            "on:\n  workflow_dispatch:\n",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must carry a scheduled trigger", result.stderr)

    def test_cron_entry_removed_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "- cron: \"41 4 * * 1\"",
            "- nothing: here",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must carry a `cron` string", result.stderr)

    def test_malformed_cron_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(), "- cron: \"41 4 * * 1\"", "- cron: \"41 4 *\""
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"exactly 5 non-empty", result.stderr)

    def test_workflow_dispatch_removed_fails(self) -> None:
        doc = drop_line(self.honest_workflow(), "  workflow_dispatch:")
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"workflow_dispatch", result.stderr)

    def test_push_trigger_added_fails(self) -> None:
        doc = replace_once(self.honest_workflow(), "on:\n", "on:\n  push:\n")
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must NOT carry a 'push' trigger", result.stderr)
        self.assertIn(b"no push is ever gated", result.stderr)

    def test_pull_request_trigger_added_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(), "on:\n", "on:\n  pull_request:\n"
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must NOT carry a 'pull_request' trigger", result.stderr)

    def test_permissions_removed_fails(self) -> None:
        doc = drop_line(self.honest_workflow(), "  contents: read")
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"least-privilege", result.stderr)

    # ---- the universal2 job ----------------------------------------------

    def test_job_removed_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(), "  macos-universal2:\n", "  macos-other:\n"
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"defines no 'macos-universal2' job", result.stderr)

    def test_wrong_runner_label_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "runs-on: macos-latest",
            "runs-on: ubuntu-latest",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must run on a macOS runner label", result.stderr)

    def test_toolchain_step_removed_fails(self) -> None:
        doc = drop_step(
            self.honest_workflow(), "- uses: dtolnay/rust-toolchain@stable"
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must install BOTH universal2 slices", result.stderr)

    def test_toolchain_one_target_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "targets: aarch64-apple-darwin,x86_64-apple-darwin",
            "targets: aarch64-apple-darwin",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must install BOTH universal2 slices", result.stderr)

    def test_aarch64_build_removed_fails(self) -> None:
        doc = drop_step(
            self.honest_workflow(),
            "- run: cargo build --release --locked -p bedlam-shell"
            " --target aarch64-apple-darwin",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"aarch64-apple-darwin build step must run exactly", result.stderr)

    def test_x86_64_build_removed_fails(self) -> None:
        doc = drop_step(
            self.honest_workflow(),
            "- run: cargo build --release --locked -p bedlam-shell"
            " --target x86_64-apple-darwin",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"x86_64-apple-darwin build step must run exactly", result.stderr)

    def test_build_unlocked_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "cargo build --release --locked -p bedlam-shell"
            " --target aarch64-apple-darwin",
            "cargo build --release -p bedlam-shell"
            " --target aarch64-apple-darwin",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"aarch64-apple-darwin build step must run exactly", result.stderr)

    def test_lipo_step_removed_fails(self) -> None:
        doc = drop_step(
            self.honest_workflow(),
            "- name: join the two slices into the universal2 binary",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must join the two slices with `lipo -create`", result.stderr)

    def test_lipo_output_tampered_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "-output staging/bedlam-shell",
            "-output staging/other",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must write exactly -output", result.stderr)

    def test_lipo_input_dropped_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "target/aarch64-apple-darwin/release/bedlam-shell",
            "target/other/release/bedlam-shell",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must consume exactly the built binary", result.stderr)

    # ---- the upload -------------------------------------------------------

    def test_upload_step_removed_fails(self) -> None:
        doc = drop_step(
            self.honest_workflow(),
            "- name: upload macos universal2 binary (scheduled artifact)",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"uploads the artifact 'bedlam-shell-macos-universal2'", result.stderr)

    def test_upload_action_downgraded_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "actions/upload-artifact@v4",
            "actions/upload-artifact@v3",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"uploads the artifact 'bedlam-shell-macos-universal2'", result.stderr)

    def test_artifact_name_tampered_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "name: bedlam-shell-macos-universal2",
            "name: bedlam-shell-macos",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"uploads the artifact 'bedlam-shell-macos-universal2'", result.stderr)

    def test_artifact_path_tampered_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "path: staging/bedlam-shell",
            "path: target/release/bedlam-shell",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"upload path must be exactly 'staging/bedlam-shell'", result.stderr)

    def test_if_no_files_found_relaxed_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "if-no-files-found: error",
            "if-no-files-found: warn",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"if-no-files-found: error", result.stderr)
        self.assertIn(b"found 'warn'", result.stderr)

    def test_retention_unbounded_fails(self) -> None:
        doc = drop_line(self.honest_workflow(), "          retention-days: 14")
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must bound retention at retention-days: 14", result.stderr)

    # ---- goldens never run on macOS CI ------------------------------------

    def test_cargo_test_injected_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "      - run: cargo build --release --locked -p bedlam-shell"
            " --target x86_64-apple-darwin\n",
            "      - run: cargo build --release --locked -p bedlam-shell"
            " --target x86_64-apple-darwin\n"
            "      - run: cargo test --workspace\n",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries a test command", result.stderr)
        self.assertIn(b"goldens never run on macOS CI", result.stderr)

    def test_diffharness_injected_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "      - run: cargo build --release --locked -p bedlam-shell"
            " --target x86_64-apple-darwin\n",
            "      - run: cargo build --release --locked -p bedlam-shell"
            " --target x86_64-apple-darwin\n"
            "      - run: ./tools/diffharness/run-goldens.sh\n",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries a test command", result.stderr)

    # ---- the denylists ----------------------------------------------------

    def test_corpus_token_injected_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "name: macos-universal2\n",
            "name: macos-universal2\n# the runner stages game-data first\n",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"mentions the original-install directory", result.stderr)

    def test_secret_reference_fails(self) -> None:
        doc = replace_once(
            self.honest_workflow(),
            "          retention-days: 14\n",
            "          retention-days: 14\n"
            "          leak: ${{ secrets.MACOS_KEY }}\n",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)
        self.assertIn(b"secrets", result.stderr)

    def test_codesign_comment_fails(self) -> None:
        # The denylist includes comments by design: the workflow file
        # must carry no signing vocabulary at all.
        doc = replace_once(
            self.honest_workflow(),
            "name: macos-universal2\n",
            "name: macos-universal2\n# then we codesign it\n",
        )
        result = self.run_checker(doc)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)
        self.assertIn(b"codesign", result.stderr)


if __name__ == "__main__":
    unittest.main()
