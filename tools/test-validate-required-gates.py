#!/usr/bin/env python3
"""Hermetic contracts for validate-required-gates.py."""

import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path


VALIDATOR = Path(__file__).with_name("validate-required-gates.py")
# Gate commands run with no PATH: every spawned executable must be
# absolute or the suite itself fails inside its own validator containment.
GIT = "/usr/bin/git"
SHA256SUM = "/usr/bin/sha256sum"


class ValidatorTests(unittest.TestCase):
    def fixture(
        self,
        manifest: str,
        files: dict[str, str] | None = None,
        ignore: list[str] | None = None,
        base_dir: str | None = None,
    ) -> Path:
        # Fixtures live under HOME: inside the gates-validator gate that is
        # the validator's own repo-anchored scratch home (writable through
        # the sandbox, and visible to a nested validator, unlike /tmp which
        # every sandbox layer replaces with a fresh private tmpfs).
        # base_dir overrides that for controller-shaped fixtures that must
        # live under /tmp itself (the sandbox tmpfs is writable and the
        # validator re-exposes the invocation root's own chain there).
        base = Path(base_dir) if base_dir else Path(os.environ.get("HOME") or tempfile.gettempdir())
        root = Path(tempfile.mkdtemp(prefix="required-gates-", dir=base))
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
            [SHA256SUM, str(anchor)], check=True, capture_output=True, text=True
        ).stdout.split()[0]
        (root / "MANIFEST.sha256").write_text(f"{digest}  manifest-anchor.txt\n")
        if ignore:
            (root / ".gitignore").write_text("".join(f"{line}\n" for line in ignore))
        subprocess.run([GIT, "init", "-q", str(root)], check=True)
        subprocess.run([GIT, "-C", str(root), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run([GIT, "-C", str(root), "config", "user.name", "test"], check=True)
        subprocess.run([GIT, "-C", str(root), "add", "."], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "fixture"], check=True)
        return root

    def run_validator(
        self,
        root: Path,
        *,
        env: dict[str, str] | None = None,
        cwd: Path | None = None,
        report: Path | None = None,
    ) -> tuple[int, dict]:
        report = report or root / "report.json"
        result = subprocess.run(
            [sys.executable, str(VALIDATOR), "--root", str(root), "--report", str(report)],
            env=env,
            cwd=str(cwd or root),
            timeout=60,
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
        subprocess.run([GIT, "-C", str(root), "rm", "-q", "MANIFEST.sha256"], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "remove manifest"], check=True)
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
            [GIT, "-C", str(root), "rev-parse", "HEAD"],
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

    def test_manifest_writable_dir_is_bound_with_private_tmp_and_read_only_root(self):
        # /tmp holds either nothing (live-root invocation) or exactly the
        # invocation root's own path chain when that root lives under /tmp
        # (the controller's sealed completion basis, re-exposed read-only)
        # -- the single tolerated presence the check-gates-env containment
        # contract pins. HOME must be the per-command scratch home under
        # the invocation root's target/.gate-home, wherever that root
        # lives (a /tmp-rooted basis puts the gate home under /tmp too).
        script = """#!/bin/bash
set -e
root="$PWD"
cd runtime/out
printf '%s\\n' "$HOME" > home.txt
tmp_holds_only_root_chain() {
  local top path rest component
  top="$(ls -A /tmp)" || return 1
  [ -z "$top" ] && return 0
  case "$1" in
    /tmp/*) ;;
    *) return 1 ;;
  esac
  path=/tmp
  rest="${1#/tmp/}"
  while [ -n "$rest" ]; do
    component="${rest%%/*}"
    [ "$(ls -A "$path")" = "$component" ] || return 1
    path="$path/$component"
    if [ "$rest" = "$component" ]; then rest=; else rest="${rest#*/}"; fi
  done
  return 0
}
tmp_holds_only_root_chain "$root" || { ls -A /tmp > listing.txt; exit 3; }
[ -n "$HOME" ] && [ -d "$HOME" ]
case "$HOME" in "$root"/target/.gate-home/*) ;; *) exit 4;; esac
if (echo x > ../../tracked-roots.txt) 2>/dev/null; then exit 7; fi
exit 0
"""
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="w"\n'
            'command=["/bin/bash","tools/w.sh"]\ntimeout_seconds=30\n'
            'writable=["runtime/out"]\n',
            {"tools/w.sh": script},
            ignore=["/runtime/"],
        )
        rc, report = self.run_validator(root)
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])
        self.assertEqual(report["gates"][0]["writable"], ["runtime/out"])

    def test_writable_dir_must_be_gitignored_and_untracked(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="w"\n'
            'command=["/usr/bin/true"]\ntimeout_seconds=2\n'
            'writable=["runtime/out"]\n',
            {"runtime/out/keep.txt": "tracked\n"},
        )
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("gitignored", report["error"])

    def test_manifest_corpus_accepts_controller_scale_assets(self):
        # The external corpus contract: MANIFEST.sha256 lists read-only
        # originals (game-data WAVs, TITLE.SMK) that are untracked-but-
        # present and range 17-40 MB. The validator must accept them at
        # the controller's 128 MiB binding cap, not the 16 MiB tracked
        # content cap, or every real validation fails before any gate.
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
        )
        corpus = root / "game-data" / "BIG0.WAV"
        corpus.parent.mkdir(parents=True, exist_ok=True)
        corpus.touch()
        os.truncate(corpus, 17 * 1024 * 1024)  # sparse: no disk cost
        digest = subprocess.run(
            [SHA256SUM, str(corpus)], check=True, capture_output=True, text=True
        ).stdout.split()[0]
        manifest = root / "MANIFEST.sha256"
        manifest.write_text(manifest.read_text() + f"{digest}  game-data/BIG0.WAV\n")
        subprocess.run([GIT, "-C", str(root), "add", "MANIFEST.sha256"], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "corpus"], check=True)
        rc, report = self.run_validator(root)
        self.assertEqual(rc, 0)
        self.assertTrue(report["plan_complete"])

    def test_manifest_corpus_over_controller_cap_fails_closed(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
        )
        corpus = root / "game-data" / "HUGE.WAV"
        corpus.parent.mkdir(parents=True, exist_ok=True)
        corpus.touch()
        os.truncate(corpus, 128 * 1024 * 1024 + 1)
        digest = subprocess.run(
            [SHA256SUM, str(corpus)], check=True, capture_output=True, text=True
        ).stdout.split()[0]
        manifest = root / "MANIFEST.sha256"
        manifest.write_text(manifest.read_text() + f"{digest}  game-data/HUGE.WAV\n")
        subprocess.run([GIT, "-C", str(root), "add", "MANIFEST.sha256"], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "corpus"], check=True)
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("unsafe or oversized", report["error"])

    def test_cargo_gate_requires_the_account_cache(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="c"\n'
            'command=["/usr/bin/cargo","test","--locked","--offline"]\n'
            "timeout_seconds=30\n"
        )
        rc, report = self.run_validator(root, env={"CARGO_HOME": "/nonexistent-cargo-home"})
        self.assertNotEqual(rc, 0)
        self.assertIn("cargo cache", report["error"])

    def test_env_probe_gate_script_passes_under_containment(self):
        # The env-probe gate's own command, exercised end to end: the real
        # check-gates-env.py runs inside the validator's sandbox and passes
        # only when the HOME parent contract, the empty private /tmp, the
        # read-only root, and the declared writable directory all hold.
        probe = (Path(__file__).with_name("check-gates-env.py")).read_text()
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="env-probe"\n'
            'command=["/usr/bin/python3","tools/check-gates-env.py"]\n'
            'timeout_seconds=30\nwritable=["runtime/env-probe-out"]\n',
            {"tools/check-gates-env.py": probe},
            ignore=["/runtime/"],
        )
        rc, report = self.run_validator(root)
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])

    def test_controller_shaped_root_under_tmp_runs_env_probe_gate(self):
        # complete_from_head roots the detached validation basis under
        # /tmp itself (/tmp/opencode/bedlam-completion-*). The sandbox's
        # private /tmp tmpfs must not hide that basis: the validator
        # re-exposes the invocation root read-only at its own path and
        # check-gates-env tolerates exactly that chain, nothing else.
        probe = (Path(__file__).with_name("check-gates-env.py")).read_text()
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="env-probe"\n'
            'command=["/usr/bin/python3","tools/check-gates-env.py"]\n'
            'timeout_seconds=30\nwritable=["runtime/env-probe-out"]\n',
            {"tools/check-gates-env.py": probe},
            ignore=["/runtime/"],
            base_dir="/tmp",
        )
        self.assertTrue(root.is_relative_to(Path("/tmp")))
        rc, report = self.run_validator(root)
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])

    def test_sealed_read_only_controller_root_pre_creates_mountpoints(self):
        # The controller seals the checkout read-only BEFORE validating
        # and pre-creates target/ plus every gate-declared writable
        # directory (bwrap can only bind over mountpoints that exist);
        # the report is written outside the sealed root, exactly like
        # complete_from_head passes output paths beside the checkout.
        probe = (Path(__file__).with_name("check-gates-env.py")).read_text()
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="env-probe"\n'
            'command=["/usr/bin/python3","tools/check-gates-env.py"]\n'
            'timeout_seconds=30\nwritable=["runtime/env-probe-out"]\n',
            {"tools/check-gates-env.py": probe},
            ignore=["/runtime/"],
            base_dir="/tmp",
        )
        (root / "target").mkdir(mode=0o700)
        (root / "runtime" / "env-probe-out").mkdir(mode=0o700, parents=True)

        def unseal() -> None:
            for current, directories, files in os.walk(root):
                os.chmod(current, 0o700)
                for name in files:
                    os.chmod(os.path.join(current, name), 0o600)

        self.addCleanup(unseal)
        for current, directories, files in os.walk(root, topdown=False):
            for name in files:
                path = os.path.join(current, name)
                os.chmod(path, stat.S_IMODE(os.stat(path).st_mode) & ~0o222)
            for name in directories:
                path = os.path.join(current, name)
                os.chmod(path, stat.S_IMODE(os.stat(path).st_mode) & ~0o222)
        outside = Path(tempfile.mkdtemp(prefix="required-gates-report-"))
        self.addCleanup(shutil.rmtree, outside, ignore_errors=True)
        rc, report = self.run_validator(root, report=outside / "report.json")
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])


    def test_gate_commands_can_run_nested_validators(self):
        # The gates-validator gate runs this very suite inside the sandbox:
        # a gate command must be able to run a full nested validator whose
        # fixtures live under HOME and whose own scratch must not need the
        # gate's private (empty) /tmp.
        validator_source = VALIDATOR.read_text()
        nested = (
            "import hashlib, json, os, pathlib, subprocess, sys, tempfile\n"
            "validator = pathlib.Path(__file__).with_name('validate-required-gates.py')\n"
            "root = pathlib.Path(tempfile.mkdtemp(prefix='nested-', "
            "dir=pathlib.Path(os.environ['HOME'])))\n"
            "(root / 'docs').mkdir()\n"
            "(root / 'docs/required-gates.toml').write_text(\n"
            "    'schema=\"required-gates-v1\"\\n[[gate]]\\nid=\"ok\"\\n'\n"
            "    'command=[\"/usr/bin/true\"]\\ntimeout_seconds=10\\n')\n"
            "anchor = root / 'anchor.txt'\n"
            "anchor.write_text('anchor\\n')\n"
            "digest = hashlib.sha256(anchor.read_bytes()).hexdigest()\n"
            "(root / 'MANIFEST.sha256').write_text(f'{digest}  anchor.txt\\n')\n"
            "subprocess.run(['/usr/bin/git', 'init', '-q', str(root)], check=True)\n"
            "subprocess.run(['/usr/bin/git', '-C', str(root), 'config', "
            "'user.email', 'test@example.invalid'], check=True)\n"
            "subprocess.run(['/usr/bin/git', '-C', str(root), 'config', "
            "'user.name', 'test'], check=True)\n"
            "subprocess.run(['/usr/bin/git', '-C', str(root), 'add', '.'], check=True)\n"
            "subprocess.run(['/usr/bin/git', '-C', str(root), 'commit', "
            "'-qm', 'nested'], check=True)\n"
            "subprocess.run(\n"
            "    [sys.executable, str(validator), '--root', str(root),\n"
            "     '--report', str(root / 'report.json')],\n"
            "    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)\n"
            "report = json.loads((root / 'report.json').read_text())\n"
            "raise SystemExit(0 if report.get('plan_complete') else 1)\n"
        )
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nid="nested"\n'
            'command=["/usr/bin/python3","tools/nested.py"]\ntimeout_seconds=60\n',
            {
                "tools/nested.py": nested,
                "tools/validate-required-gates.py": validator_source,
            },
        )
        rc, report = self.run_validator(root)
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])


if __name__ == "__main__":
    unittest.main()
