#!/usr/bin/env python3
"""Hermetic fail-closed contracts for check-p7-windows-installer.py.

Every structural rule of the checker is proven to FAIL LOUDLY on the
specific tampering it guards against, and to pass on the honest
landed state. Two tests run the checker against the REAL repository
definition (the same thing the gate runs), pinning the honest state
and its summary; the rest build a scratch tree from the honest
files, tamper one bounded piece, and require the matching refusal.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("check-p7-windows-installer.py")
REPO = CHECKER.parent.parent
PYTHON = sys.executable or "/usr/bin/python3"
SCRIPT_RELATIVE = "packaging/bedlam-shell.nsi"
README_RELATIVE = "packaging/windows-installer-README.txt"
WORKFLOW_RELATIVE = ".github/workflows/ci.yml"


def replace_once(text: str, old: str, new: str) -> str:
    result = text.replace(old, new, 1)
    if result == text:
        raise AssertionError(f"fixture bug: needle not found: {old!r}")
    return result


def drop_line(text: str, exact: str) -> str:
    result = text.replace(f"{exact}\n", "", 1)
    if result == text:
        raise AssertionError(f"fixture bug: line not found: {exact!r}")
    return result


def drop_trailing_job(text: str, job: str) -> str:
    lines = text.splitlines(keepends=True)
    start = None
    for index, line in enumerate(lines):
        if line.rstrip("\n") == f"  {job}:":
            start = index
    if start is None:
        raise AssertionError(f"fixture bug: job not found: {job!r}")
    while start > 0 and lines[start - 1].strip() == "":
        start -= 1
    return "".join(lines[:start]) + "\n"


class WindowsInstallerCheckerTests(unittest.TestCase):
    def run_checker_tree(
        self,
        script: str | None,
        readme: str | None,
        workflow: str | None,
    ) -> subprocess.CompletedProcess[bytes]:
        with tempfile.TemporaryDirectory(prefix="p7-wininst-") as scratch:
            root = Path(scratch)
            (root / "packaging").mkdir(parents=True)
            (root / ".github" / "workflows").mkdir(parents=True)
            if script is not None:
                (root / SCRIPT_RELATIVE).write_text(
                    script, encoding="utf-8"
                )
            if readme is not None:
                (root / README_RELATIVE).write_text(
                    readme, encoding="utf-8"
                )
            if workflow is not None:
                (root / WORKFLOW_RELATIVE).write_text(
                    workflow, encoding="utf-8"
                )
            return subprocess.run(
                [PYTHON, str(CHECKER), "--root", str(root)],
                capture_output=True,
                timeout=120,
            )

    def run_checker(
        self, **overrides: str | None
    ) -> subprocess.CompletedProcess[bytes]:
        script = overrides.pop(
            "script", (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8")
        )
        readme = overrides.pop(
            "readme", (REPO / README_RELATIVE).read_text(encoding="utf-8")
        )
        workflow = overrides.pop(
            "workflow",
            (REPO / WORKFLOW_RELATIVE).read_text(encoding="utf-8"),
        )
        assert not overrides, overrides
        return self.run_checker_tree(script, readme, workflow)

    # ---- the honest state passes ----------------------------------------

    def test_real_repo_passes(self) -> None:
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(REPO)],
            capture_output=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-windows-installer: OK", result.stdout)
        self.assertIn(b"closed grammar", result.stdout)
        self.assertIn(b"bedlam-shell-setup.exe", result.stdout)
        self.assertIn(b"file set: exactly", result.stdout)
        self.assertIn(b"ci join: job 'windows-installer'", result.stdout)
        self.assertIn(b"signing material: none", result.stdout)

    def test_honest_copies_pass(self) -> None:
        result = self.run_checker()
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-windows-installer: OK", result.stdout)

    def test_minimal_honest_script_passes(self) -> None:
        minimal = (
            'Name "Bedlam engine"\n'
            'OutFile "bedlam-shell-setup.exe"\n'
            "Unicode true\n"
            'InstallDir "$PROGRAMFILES64\\Bedlam"\n'
            "RequestExecutionLevel admin\n"
            "CRCCheck force\n"
            "Page directory\n"
            "Page instfiles\n"
            "UninstPage uninstConfirm\n"
            "UninstPage instfiles\n"
            'Section "Bedlam engine"\n'
            '  SetOutPath "$INSTDIR"\n'
            '  File "bedlam-shell.exe"\n'
            '  File "windows-installer-README.txt"\n'
            '  WriteUninstaller "$INSTDIR\\uninstall.exe"\n'
            '  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BedlamEngine" "DisplayName" "Bedlam engine"\n'
            '  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BedlamEngine" "UninstallString" "$INSTDIR\\uninstall.exe"\n'
            '  CreateDirectory "$SMPROGRAMS\\Bedlam"\n'
            '  CreateShortcut "$SMPROGRAMS\\Bedlam\\Bedlam engine.lnk" "$INSTDIR\\bedlam-shell.exe"\n'
            "SectionEnd\n"
            'Section "un.Uninstall"\n'
            '  Delete "$SMPROGRAMS\\Bedlam\\Bedlam engine.lnk"\n'
            '  RMDir "$SMPROGRAMS\\Bedlam"\n'
            '  Delete "$INSTDIR\\bedlam-shell.exe"\n'
            '  Delete "$INSTDIR\\windows-installer-README.txt"\n'
            '  Delete "$INSTDIR\\uninstall.exe"\n'
            '  DeleteRegKey HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BedlamEngine"\n'
            '  RMDir "$INSTDIR"\n'
            "SectionEnd\n"
        )
        result = self.run_checker(script=minimal)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-windows-installer: OK", result.stdout)

    # ---- file + parse discipline ----------------------------------------

    def test_missing_script_fails(self) -> None:
        result = self.run_checker(script=None)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"file is missing", result.stderr)

    def test_missing_readme_fails(self) -> None:
        result = self.run_checker(readme=None)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"file is missing", result.stderr)

    def test_missing_workflow_fails(self) -> None:
        result = self.run_checker(workflow=None)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"file is missing", result.stderr)

    def test_unknown_command_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'Name "Bedlam engine"',
            'MessageBox MB_OK "hi"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unknown command", result.stderr)

    def test_compiler_directive_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'Name "Bedlam engine"',
            '!define FOO "bar"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unknown command", result.stderr)

    def test_label_line_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            '  SetOutPath "$INSTDIR"',
            "MyLabel:\n"
            '  SetOutPath "$INSTDIR"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unknown command", result.stderr)

    def test_c_style_comment_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'Name "Bedlam engine"',
            "/* block comment */",
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"C-style comments are outside the closed grammar", result.stderr)

    def test_line_continuation_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'Page directory\n',
            'Page \\\n directory\n',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"line continuations are outside the closed grammar", result.stderr)

    def test_unbalanced_quote_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'Name "Bedlam engine"',
            'Name "Bedlam engine',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unbalanced quote", result.stderr)

    def test_quoted_bare_argument_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            "CRCCheck force",
            'CRCCheck "force"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"bare argument arrived quoted", result.stderr)

    def test_instruction_outside_section_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'Page directory\n',
            'Page directory\n  Delete "$INSTDIR\\something"\n',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"outside any section", result.stderr)

    def test_attribute_inside_section_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            '  SetOutPath "$INSTDIR"',
            '  SetOutPath "$INSTDIR"\n  CRCCheck on',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"inside a section", result.stderr)

    def test_unterminated_section_fails(self) -> None:
        script = (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8")
        cut = script.rindex("SectionEnd")
        script = script[:cut].rstrip() + "\n"
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"ends inside an open section", result.stderr)

    # ---- the installer schema -------------------------------------------

    def test_outfile_renamed_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'OutFile "bedlam-shell-setup.exe"',
            'OutFile "setup.exe"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"OutFile must be exactly 'bedlam-shell-setup.exe'", result.stderr)

    def test_installdir_moved_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'InstallDir "$PROGRAMFILES64\\Bedlam"',
            'InstallDir "$PROGRAMFILES\\Bedlam"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"InstallDir must be exactly", result.stderr)

    def test_execution_level_lowered_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            "RequestExecutionLevel admin",
            "RequestExecutionLevel user",
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"RequestExecutionLevel must be admin", result.stderr)

    def test_crccheck_relaxed_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            "CRCCheck force",
            "CRCCheck on",
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"CRCCheck must be force", result.stderr)

    def test_unicode_off_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            "Unicode true",
            "Unicode false",
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"Unicode must be true", result.stderr)

    def test_page_flow_tampered_fails(self) -> None:
        script = drop_line(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            "Page directory",
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"page flow must be exactly", result.stderr)

    def test_custom_page_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            "Page directory",
            "Page custom",
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not a built-in page", result.stderr)

    # ---- the engine-only file set ---------------------------------------

    def test_third_file_rides_along_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            '  File "windows-installer-README.txt"\n',
            '  File "windows-installer-README.txt"\n'
            '  File "extra-bundled-thing.txt"\n',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"closed two-file set", result.stderr)

    def test_wildcard_file_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            '  File "bedlam-shell.exe"',
            '  File "*.exe"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not a staged bare file name", result.stderr)

    def test_pathed_file_source_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            '  File "bedlam-shell.exe"',
            '  File "..\\target\\release\\bedlam-shell.exe"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not a staged bare file name", result.stderr)

    def test_recursive_delete_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            '  RMDir "$INSTDIR"',
            '  RMDir /r "$INSTDIR"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"takes 1 argument(s)", result.stderr)

    def test_uninstall_deleting_uninstalled_file_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            '  Delete "$INSTDIR\\uninstall.exe"',
            '  Delete "$INSTDIR\\uninstall.exe"\n'
            '  Delete "$INSTDIR\\settings.ini"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"which the installer never wrote", result.stderr)

    def test_reordered_install_body_fails(self) -> None:
        # CreateShortcut before SetOutPath would steal the wrong
        # $OUTDIR as the shortcut's working directory.
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            '  SetOutPath "$INSTDIR"\n',
            '  CreateShortcut "$SMPROGRAMS\\Bedlam\\Bedlam engine.lnk"'
            ' "$INSTDIR\\bedlam-shell.exe"\n'
            '  SetOutPath "$INSTDIR"\n',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not the pinned", result.stderr)

    def test_third_section_fails(self) -> None:
        script = replace_once(
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8"),
            'Section "un.Uninstall"',
            'Section "bonus"\nSectionEnd\n\nSection "un.Uninstall"',
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"exactly two sections", result.stderr)

    def test_missing_uninstall_section_fails(self) -> None:
        script = (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8")
        cut = script.index('Section "un.Uninstall"')
        script = script[:cut].rstrip() + "\n"
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"exactly two sections", result.stderr)

    # ---- the README contract ---------------------------------------------

    def test_readme_boundary_sentence_removed_fails(self) -> None:
        readme = replace_once(
            (REPO / README_RELATIVE).read_text(encoding="utf-8"),
            "This install carries the ENGINE ONLY",
            "This install carries everything",
        )
        result = self.run_checker(readme=readme)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the required boundary sentence", result.stderr)

    def test_readme_default_layout_removed_fails(self) -> None:
        readme = (REPO / README_RELATIVE).read_text(encoding="utf-8")
        readme = readme.replace("game-data\\BEDLAM", "the default folder")
        result = self.run_checker(readme=readme)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"missing the required boundary sentence", result.stderr)
        self.assertIn(b"game-data", result.stderr)

    def test_readme_foreign_corpus_reference_fails(self) -> None:
        readme = replace_once(
            (REPO / README_RELATIVE).read_text(encoding="utf-8"),
            "Run bedlam-shell.exe --help",
            "The full asset dump lives in game-data/BEDLAM on disc.",
        )
        result = self.run_checker(readme=readme)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"outside the documented default layout", result.stderr)

    def test_readme_signing_vocabulary_fails(self) -> None:
        readme = (
            (REPO / README_RELATIVE).read_text(encoding="utf-8")
            + "\nThis build is marked with gpg signatures.\n"
        )
        result = self.run_checker(readme=readme)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)

    def test_readme_empty_fails(self) -> None:
        result = self.run_checker(readme="\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"is empty", result.stderr)

    # ---- the CI build join -------------------------------------------------

    def ci_workflow(self) -> str:
        return (REPO / WORKFLOW_RELATIVE).read_text(encoding="utf-8")

    def test_job_removed_fails(self) -> None:
        workflow = drop_trailing_job(self.ci_workflow(), "windows-installer")
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no `windows-installer` job", result.stderr)

    def test_job_moved_to_linux_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "  windows-installer:\n    runs-on: windows-latest",
            "  windows-installer:\n    runs-on: ubuntu-latest",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must run on windows-latest", result.stderr)

    def test_choco_step_removed_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "      - name: install nsis\n        run: choco install nsis -y\n",
            "",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"never installs NSIS via chocolatey", result.stderr)

    def test_staging_copy_removed_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "      - name: stage the engine binary next to the script\n"
            "        run: Copy-Item target\\release\\bedlam-shell.exe"
            " packaging\\bedlam-shell.exe\n",
            "",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"never stages", result.stderr)

    def test_working_directory_dropped_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "      - name: build the installer\n"
            "        working-directory: packaging\n",
            "      - name: build the installer\n",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"working-directory: packaging", result.stderr)

    def test_makensis_compiles_other_script_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "run: '& \"${env:ProgramFiles(x86)}\\NSIS\\makensis.exe\" bedlam-shell.nsi'",
            "run: '& \"${env:ProgramFiles(x86)}\\NSIS\\makensis.exe\""
            " some-other-app.nsi'",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"does not compile THIS script", result.stderr)

    def test_build_without_lockfile_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "      - run: cargo build --release --locked -p bedlam-shell\n",
            "      - run: cargo build --release -p bedlam-shell\n",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not --locked", result.stderr)

    def test_build_offline_impossible_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "      - run: cargo build --release --locked -p bedlam-shell\n",
            "      - run: cargo build --release --locked --offline"
            " -p bedlam-shell\n",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"--offline but no vendored crate set", result.stderr)

    def test_upload_path_diverges_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "          path: packaging/bedlam-shell-setup.exe\n",
            "          path: target/release/bedlam-shell-setup.exe\n",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"upload path must be exactly", result.stderr)

    def test_upload_relaxed_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "          path: packaging/bedlam-shell-setup.exe\n"
            "          if-no-files-found: error",
            "          path: packaging/bedlam-shell-setup.exe\n"
            "          if-no-files-found: warn",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"if-no-files-found: error", result.stderr)

    def test_upload_retention_unbounded_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "          path: packaging/bedlam-shell-setup.exe\n"
            "          if-no-files-found: error\n"
            "          retention-days: 14",
            "          path: packaging/bedlam-shell-setup.exe\n"
            "          if-no-files-found: error",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"bounded retention-days", result.stderr)

    def test_job_mentioning_corpus_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "      - name: install nsis\n        run: choco install nsis -y",
            "      - name: install nsis\n        run: choco install nsis -y\n"
            "        # the corpus never rides along (game-data)\n",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"mentions 'game-data'", result.stderr)

    # ---- no signing material ----------------------------------------------

    def test_script_signing_comment_fails(self) -> None:
        script = (
            (REPO / SCRIPT_RELATIVE).read_text(encoding="utf-8")
            + "; then we gpg-mark the installer\n"
        )
        result = self.run_checker(script=script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)

    def test_job_secret_reference_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "      - name: install nsis\n        run: choco install nsis -y",
            "      - name: install nsis\n        run: choco install nsis -y\n"
            "        # echo ${{ secrets.SOME_KEY }}\n",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)
        self.assertIn(b"secrets", result.stderr)


if __name__ == "__main__":
    unittest.main()
