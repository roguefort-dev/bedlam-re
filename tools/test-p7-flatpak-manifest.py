#!/usr/bin/env python3
"""Hermetic fail-closed contracts for check-p7-flatpak-manifest.py.

Every structural rule of the checker is proven to FAIL LOUDLY on the
specific tampering it guards against, and to pass on the honest
landed state. Two tests run the checker against the REAL repository
definition (the same thing the gate runs), pinning the honest state
and its summary; the rest build a scratch tree from the honest
files, tamper one bounded piece, and require the matching refusal.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("check-p7-flatpak-manifest.py")
REPO = CHECKER.parent.parent
PYTHON = sys.executable or "/usr/bin/python3"
MANIFEST_RELATIVE = "packaging/dev.roguefort.bedlam.yml"
DESKTOP_RELATIVE = "packaging/dev.roguefort.bedlam.desktop"
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
    # the job is the last block in the file; cut the blank separator
    while start > 0 and lines[start - 1].strip() == "":
        start -= 1
    return "".join(lines[:start]) + "\n"


class FlatpakManifestCheckerTests(unittest.TestCase):
    def run_checker_tree(
        self,
        manifest: str | None,
        desktop: str | None,
        workflow: str | None,
    ) -> subprocess.CompletedProcess[bytes]:
        with tempfile.TemporaryDirectory(prefix="p7-flatpak-") as scratch:
            root = Path(scratch)
            (root / "packaging").mkdir(parents=True)
            (root / ".github" / "workflows").mkdir(parents=True)
            if manifest is not None:
                (root / MANIFEST_RELATIVE).write_text(
                    manifest, encoding="utf-8"
                )
            if desktop is not None:
                (root / DESKTOP_RELATIVE).write_text(
                    desktop, encoding="utf-8"
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

    def run_checker(self, **overrides: str | None) -> subprocess.CompletedProcess[bytes]:
        manifest = overrides.pop(
            "manifest",
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
        )
        desktop = overrides.pop(
            "desktop",
            (REPO / DESKTOP_RELATIVE).read_text(encoding="utf-8"),
        )
        workflow = overrides.pop(
            "workflow",
            (REPO / WORKFLOW_RELATIVE).read_text(encoding="utf-8"),
        )
        assert not overrides, overrides
        return self.run_checker_tree(manifest, desktop, workflow)

    # ---- the honest state passes ----------------------------------------

    def test_real_repo_passes(self) -> None:
        result = subprocess.run(
            [PYTHON, str(CHECKER), "--root", str(REPO)],
            capture_output=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-flatpak-manifest: OK", result.stdout)
        self.assertIn(b"app: dev.roguefort.bedlam", result.stdout)
        self.assertIn(b"closed five-token surface", result.stdout)
        self.assertIn(b"never-bundle: dir source at the repo root", result.stdout)
        self.assertIn(b"ci join: job 'flatpak'", result.stdout)
        self.assertIn(b"signing material: none", result.stdout)

    def test_honest_copies_pass(self) -> None:
        result = self.run_checker()
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-flatpak-manifest: OK", result.stdout)

    def test_minimal_honest_manifest_passes(self) -> None:
        minimal = (
            "app-id: dev.roguefort.bedlam\n"
            "runtime: org.freedesktop.Platform\n"
            'runtime-version: "24.08"\n'
            "sdk: org.freedesktop.Sdk\n"
            "command: bedlam-shell\n"
            "finish-args:\n"
            "  - --socket=wayland\n"
            "  - --socket=fallback-x11\n"
            "  - --socket=pulseaudio\n"
            "  - --device=dri\n"
            "  - --share=ipc\n"
            "modules:\n"
            "  - name: bedlam-shell\n"
            "    buildsystem: simple\n"
            "    build-options:\n"
            "      append-path: /usr/lib/sdk/rust-stable/bin\n"
            "      env:\n"
            "        CARGO_HOME: /run/build/bedlam-shell/cargo\n"
            "    build-commands:\n"
            "      - cargo build --release --locked -p bedlam-shell\n"
            "      - install -Dm755 target/release/bedlam-shell -t /app/bin\n"
            "      - install -Dm644 packaging/dev.roguefort.bedlam.desktop -t /app/share/applications\n"
            "    sources:\n"
            "      - type: dir\n"
            "        path: ..\n"
            "        skip:\n"
            "          - .git\n"
            "          - derived\n"
            "          - derived-2\n"
            "          - game-data\n"
            "          - game-data-2\n"
            "          - ghidra-project\n"
            "          - goldens\n"
            "          - target\n"
        )
        result = self.run_checker(manifest=minimal)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"p7-flatpak-manifest: OK", result.stdout)

    # ---- file + parse discipline ----------------------------------------

    def test_missing_manifest_fails(self) -> None:
        result = self.run_checker(manifest=None)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"file is missing", result.stderr)

    def test_missing_desktop_fails(self) -> None:
        result = self.run_checker(desktop=None)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"file is missing", result.stderr)

    def test_missing_workflow_fails(self) -> None:
        result = self.run_checker(workflow=None)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"file is missing", result.stderr)

    def test_tab_indentation_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "\nfinish-args:",
            "\n\tfinish-args:",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"tab in indentation", result.stderr)

    def test_unparsable_line_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "app-id: dev.roguefort.bedlam",
            "::::",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unparsable mapping line", result.stderr)

    def test_unexpected_deeper_line_fails(self) -> None:
        manifest = (
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8")
            + " stray: line\n"
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            b"unexpected deeper line where a key was due", result.stderr
        )

    # ---- the manifest schema ---------------------------------------------

    def test_unknown_top_level_key_fails(self) -> None:
        manifest = (
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8")
            + "rename-icon: bedlam\n"
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"unknown top-level keys", result.stderr)

    def test_app_id_not_reverse_dns_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "app-id: dev.roguefort.bedlam",
            "app-id: bedlam",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not reverse-DNS shaped", result.stderr)

    def test_runtime_swapped_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "runtime: org.freedesktop.Platform",
            "runtime: org.gnome.Platform",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"runtime must be 'org.freedesktop.Platform'", result.stderr)

    def test_runtime_version_unpinned_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            'runtime-version: "24.08"',
            'runtime-version: "stable"',
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not a pinned YY.MM release", result.stderr)

    def test_command_not_the_engine_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "command: bedlam-shell",
            "command: sh",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"command must be the engine binary", result.stderr)

    # ---- the closed sandbox surface ---------------------------------------

    def test_filesystem_grant_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "  - --share=ipc\n",
            "  - --share=ipc\n  - --filesystem=host\n",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"must be exactly the closed five-token surface", result.stderr)
        self.assertIn(b"--filesystem=host", result.stderr)

    def test_missing_pulseaudio_socket_fails(self) -> None:
        manifest = drop_line(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "  - --socket=pulseaudio",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"closed five-token surface", result.stderr)
        self.assertIn(b"--socket=pulseaudio", result.stderr)

    def test_network_share_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "  - --share=ipc",
            "  - --share=network",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"wider grant '--share=network'", result.stderr)

    # ---- the engine-only module -------------------------------------------

    def test_second_module_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "  - name: bedlam-shell",
            "  - name: extra-bundled-thing\n    buildsystem: simple\n    build-commands:\n      - 'true'\n    sources:\n      - type: dir\n        path: ..\n        skip:\n          - .git\n  - name: bedlam-shell",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"modules must be exactly the one engine module", result.stderr)

    def test_build_without_lockfile_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "cargo build --release --locked -p bedlam-shell",
            "cargo build --release -p bedlam-shell",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not --locked", result.stderr)

    def test_offline_build_impossible_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "cargo build --release --locked -p bedlam-shell",
            "cargo build --release --locked --offline -p bedlam-shell",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"--offline but no vendored crate set", result.stderr)

    def test_rust_extension_removed_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "append-path: /usr/lib/sdk/rust-stable/bin",
            "append-path: /usr/bin",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"rust-stable extension", result.stderr)

    def test_binary_install_removed_fails(self) -> None:
        manifest = drop_line(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "      - install -Dm755 target/release/bedlam-shell -t /app/bin",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"installs target/release/bedlam-shell into /app/bin", result.stderr)

    # ---- the never-bundle guard --------------------------------------------

    def test_skip_floor_missing_corpus_fails(self) -> None:
        manifest = drop_line(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "          - game-data",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"never-bundle floor", result.stderr)
        self.assertIn(b"'game-data'", result.stderr)

    def test_source_path_points_at_corpus_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "        path: ..",
            "        path: ../game-data",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'path ".."', result.stderr)

    def test_foreign_url_source_fails(self) -> None:
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "      - type: dir\n",
            "      - type: git\n"
            "        url: https://example.invalid/bedlam.git\n"
            "      - type: dir\n",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"exactly one source", result.stderr)

    def test_corpus_referenced_outside_skip_fails(self) -> None:
        # An EXTRA build-command reading the corpus: the command-level
        # rules stay satisfied, so the refusal is specifically the
        # corpus-outside-the-skip-list guard.
        manifest = replace_once(
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8"),
            "      - cargo build --release --locked -p bedlam-shell\n",
            "      - cargo build --release --locked -p bedlam-shell\n"
            "      - install -Dm644 game-data/BEDLAM/CREDITS"
            " /app/share/doc/CREDITS\n",
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"outside the skip list", result.stderr)

    # ---- the desktop entry ---------------------------------------------

    def test_desktop_icon_fails(self) -> None:
        desktop = replace_once(
            (REPO / DESKTOP_RELATIVE).read_text(encoding="utf-8"),
            "Terminal=false",
            "Terminal=false\nIcon=dev.roguefort.bedlam",
        )
        result = self.run_checker(desktop=desktop)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no Icon key", result.stderr)

    def test_desktop_exec_mismatch_fails(self) -> None:
        desktop = replace_once(
            (REPO / DESKTOP_RELATIVE).read_text(encoding="utf-8"),
            "Exec=bedlam-shell",
            "Exec=sh -c something",
        )
        result = self.run_checker(desktop=desktop)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"Exec must be the manifest command", result.stderr)

    def test_desktop_terminal_true_fails(self) -> None:
        desktop = replace_once(
            (REPO / DESKTOP_RELATIVE).read_text(encoding="utf-8"),
            "Terminal=false",
            "Terminal=true",
        )
        result = self.run_checker(desktop=desktop)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"Terminal must be false", result.stderr)

    def test_desktop_corpus_mention_fails(self) -> None:
        desktop = replace_once(
            (REPO / DESKTOP_RELATIVE).read_text(encoding="utf-8"),
            "Categories=Game;",
            "Categories=Game;\nX-Corpus=game-data/BEDLAM",
        )
        result = self.run_checker(desktop=desktop)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"mentions 'game-data'", result.stderr)

    # ---- the CI build join -----------------------------------------------

    def ci_workflow(self) -> str:
        return (REPO / WORKFLOW_RELATIVE).read_text(encoding="utf-8")

    def test_flatpak_job_removed_fails(self) -> None:
        workflow = drop_trailing_job(self.ci_workflow(), "flatpak")
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"no `flatpak` job", result.stderr)

    def test_sdk_version_diverges_from_manifest_fails(self) -> None:
        workflow = self.ci_workflow().replace(
            "org.freedesktop.Sdk//24.08", "org.freedesktop.Sdk//23.08"
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"runtime-version join", result.stderr)

    def test_rust_extension_not_installed_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "org.freedesktop.Sdk//24.08 org.freedesktop.Sdk.Extension.rust-stable//24.08",
            "org.freedesktop.Sdk//24.08",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"rust-stable extension", result.stderr)

    def test_build_command_leaves_this_manifest_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            '"$RUNNER_TEMP/flatpak-build" packaging/dev.roguefort.bedlam.yml',
            '"$RUNNER_TEMP/flatpak-build" packaging/some-other-app.yml',
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"THIS manifest is what CI builds", result.stderr)

    def test_build_bundle_app_id_diverges_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "dev.roguefort.bedlam\n",
            "some.other.app\n",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"app-id join", result.stderr)

    def test_bundle_upload_relaxed_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "          path: packaging/bedlam-shell.flatpak\n"
            "          if-no-files-found: error",
            "          path: packaging/bedlam-shell.flatpak\n"
            "          if-no-files-found: warn",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"if-no-files-found: error", result.stderr)

    def test_bundle_upload_retention_unbounded_fails(self) -> None:
        # Anchored on the flatpak bundle path so the build job's own
        # upload steps (identical if/retention lines) stay untouched.
        workflow = replace_once(
            self.ci_workflow(),
            "          path: packaging/bedlam-shell.flatpak\n"
            "          if-no-files-found: error\n"
            "          retention-days: 14",
            "          path: packaging/bedlam-shell.flatpak\n"
            "          if-no-files-found: error",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"bounded retention-days", result.stderr)

    def test_flatpak_job_mentioning_corpus_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "flatpak-builder --force-clean",
            "cp -r game-data /tmp && flatpak-builder --force-clean",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"mentions 'game-data'", result.stderr)

    # ---- no signing material ----------------------------------------------

    def test_manifest_signing_comment_fails(self) -> None:
        manifest = (
            (REPO / MANIFEST_RELATIVE).read_text(encoding="utf-8")
            + "# then we gpg-sign the bundle\n"
        )
        result = self.run_checker(manifest=manifest)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)

    def test_ci_job_secret_reference_fails(self) -> None:
        workflow = replace_once(
            self.ci_workflow(),
            "          sudo apt-get install -y flatpak-builder flatpak",
            "          sudo apt-get install -y flatpak-builder flatpak\n"
            "          echo ${{ secrets.FLATHUB_KEY }}",
        )
        result = self.run_checker(workflow=workflow)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"carries signing material", result.stderr)
        self.assertIn(b"secrets", result.stderr)


if __name__ == "__main__":
    unittest.main()
