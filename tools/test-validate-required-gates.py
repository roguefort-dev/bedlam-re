#!/usr/bin/env python3
"""Hermetic contracts for validate-required-gates.py."""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path


VALIDATOR = Path(__file__).with_name("validate-required-gates.py")


class ValidatorTests(unittest.TestCase):
    def fixture(self, manifest: str, files: dict[str, str] | None = None) -> Path:
        root = Path(tempfile.mkdtemp(prefix="required-gates-", dir="/tmp/opencode"))
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        (root / "docs").mkdir()
        (root / "docs/required-gates.toml").write_text(manifest)
        for name, contents in (files or {}).items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)
        anchor = root / "manifest-anchor.txt"
        anchor.write_text("manifest anchor\n")
        digest = subprocess.run(
            ["sha256sum", str(anchor)], check=True, capture_output=True, text=True
        ).stdout.split()[0]
        (root / "MANIFEST.sha256").write_text(f"{digest}  manifest-anchor.txt\n")
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.name", "test"], check=True)
        subprocess.run(["git", "-C", str(root), "add", "."], check=True)
        subprocess.run(["git", "-C", str(root), "commit", "-qm", "fixture"], check=True)
        return root

    def run_validator(
        self, root: Path, *, env: dict[str, str] | None = None
    ) -> tuple[int, dict]:
        report = root / "report.json"
        result = subprocess.run(
            [sys.executable, str(VALIDATOR), "--root", str(root), "--report", str(report)],
            env=env,
            timeout=8,
        )
        return result.returncode, json.loads(report.read_text())

    def test_pass_is_deterministic_and_head_bound(self):
        root = self.fixture('schema="required-gates-v1"\n[[gate]]\nid="ok"\ncommand=["/usr/bin/test","-f","ok"]\ntimeout_seconds=2\n', {"ok": ""})
        rc, first = self.run_validator(root)
        rc2, second = self.run_validator(root)
        self.assertEqual((rc, rc2), (0, 0))
        self.assertEqual(first, second)
        self.assertTrue(first["plan_complete"])

    def test_missing_corpus_fails_closed(self):
        root = self.fixture('schema="required-gates-v1"\n[[gate]]\nid="corpus"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\ncorpus=["missing.bin"]\n')
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("not tracked", report["error"])

    def test_shell_and_unlocked_cargo_are_rejected(self):
        for command in (
            '["curl","example.invalid"]',
            '["cargo","test"]',
            '["cargo","test","--locked"]',
        ):
            root = self.fixture(f'schema="required-gates-v1"\n[[gate]]\nid="bad"\ncommand={command}\ntimeout_seconds=2\n')
            rc, _report = self.run_validator(root)
            self.assertNotEqual(rc, 0)

    def test_dirty_required_path_fails(self):
        root = self.fixture('schema="required-gates-v1"\n[[gate]]\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\ntracked_paths=["proof.txt"]\n', {"proof.txt": "original\n"})
        (root / "proof.txt").write_text("changed\n")
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("differs from HEAD", report["error"])

    def test_missing_head_tracked_corpus_manifest_fails_closed(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
        )
        subprocess.run(["git", "-C", str(root), "rm", "-q", "MANIFEST.sha256"], check=True)
        subprocess.run(["git", "-C", str(root), "commit", "-qm", "remove manifest"], check=True)
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("MANIFEST.sha256", report["error"])

    def test_commands_require_absolute_allowlisted_executables(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="relative"\ncommand=["true"]\ntimeout_seconds=2\n'
        )
        rc, _report = self.run_validator(root)
        self.assertNotEqual(rc, 0)

    def test_path_cannot_select_a_malicious_executable(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="python"\ncommand=["python3","tools/gate.py"]\ntimeout_seconds=2\n',
            {"tools/gate.py": "raise SystemExit(0)\n"},
        )
        malicious = root / "malicious-bin"
        malicious.mkdir()
        sentinel = root / "path-executable-ran"
        (malicious / "python3").write_text(
            f"#!/bin/sh\ntouch {sentinel}\nexit 0\n"
        )
        (malicious / "python3").chmod(0o755)
        env = dict(os.environ, PATH=f"{malicious}:{os.environ.get('PATH', '')}")
        rc, _report = self.run_validator(root, env=env)
        self.assertNotEqual(rc, 0)
        self.assertFalse(sentinel.exists())

    def test_gate_environment_clears_code_and_network_injection_variables(self):
        variables = [
            "PATH",
            "PYTHONPATH",
            "BASH_ENV",
            "RUSTC_WRAPPER",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
        ]
        script = "import os\nraise SystemExit(any(os.environ.get(k) for k in " + repr(variables) + "))\n"
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="env"\ncommand=["/usr/bin/python3","tools/env_gate.py"]\ntimeout_seconds=2\n',
            {"tools/env_gate.py": script},
        )
        env = dict(os.environ)
        for variable in variables:
            env[variable] = "/tmp/opencode/attacker-value"
        rc, report = self.run_validator(root, env=env)
        self.assertEqual(rc, 0, report)

    def test_dirty_reachable_gate_script_never_executes(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="script"\ncommand=["/bin/bash","tools/gate.sh"]\ntimeout_seconds=2\n',
            {"tools/gate.sh": "#!/bin/bash\nexit 0\n"},
        )
        sentinel = root / "dirty-script-ran"
        (root / "tools/gate.sh").write_text(f"#!/bin/bash\ntouch {sentinel}\n")
        rc, _report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertFalse(sentinel.exists())

    def test_tracked_corpus_is_rechecked_after_each_command(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="corpus"\ncommand=["/bin/bash","tools/mutate.sh"]\ntimeout_seconds=2\ncorpus=["corpus.bin"]\n',
            {
                "corpus.bin": "original\n",
                "tools/mutate.sh": "#!/bin/bash\nprintf changed > corpus.bin\n",
            },
        )
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertFalse(report["plan_complete"])

    def test_head_is_rechecked_after_commands(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="head"\ncommand=["/bin/bash","tools/commit.sh"]\ntimeout_seconds=2\n',
            {
                "code.txt": "before\n",
                "tools/commit.sh": (
                    "#!/bin/bash\nset -e\necho after >> code.txt\n"
                    "git add code.txt\ngit commit -qm raced-head\n"
                ),
            },
        )
        original = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertEqual(report["head"], original)

    def test_nested_smoke_cargo_is_locked_and_offline(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="smoke"\ncommand=["/bin/bash","tools/smoke.sh"]\ntimeout_seconds=2\n',
            {"tools/smoke.sh": "#!/bin/bash\ncargo test\n"},
        )
        malicious = root / "cargo-bin"
        malicious.mkdir()
        sentinel = root / "nested-cargo-ran"
        (malicious / "cargo").write_text(f"#!/bin/sh\ntouch {sentinel}\nexit 0\n")
        (malicious / "cargo").chmod(0o755)
        env = dict(os.environ, PATH=f"{malicious}:{os.environ.get('PATH', '')}")
        rc, _report = self.run_validator(root, env=env)
        self.assertNotEqual(rc, 0)
        self.assertFalse(sentinel.exists())

    def test_timeout_and_success_reap_command_descendants(self):
        for mode in ("timeout", "success"):
            with self.subTest(mode=mode):
                sentinel_name = f"{mode}-descendant-ran"
                body = (
                    "#!/bin/bash\n"
                    f"setsid sh -c 'sleep 1; touch {sentinel_name}' >/dev/null 2>&1 &\n"
                    + ("sleep 30\n" if mode == "timeout" else "exit 0\n")
                )
                root = self.fixture(
                    f'schema="required-gates-v1"\n[[gate]]\nid="{mode}"\ncommand=["/bin/bash","tools/group.sh"]\ntimeout_seconds=1\n',
                    {"tools/group.sh": body},
                )
                rc, _report = self.run_validator(root)
                if mode == "success":
                    self.assertEqual(rc, 0)
                time.sleep(1.3)
                self.assertFalse((root / sentinel_name).exists())


if __name__ == "__main__":
    unittest.main()
