#!/usr/bin/env python3
"""Hermetic contracts for validate-required-gates.py."""

import json
import os
import re
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
        wrap_phases: bool = True,
    ) -> Path:
        # Fixtures live under HOME: inside the gates-validator gate that is
        # the validator's own repo-anchored scratch home (writable through
        # the sandbox, and visible to a nested validator, unlike /tmp which
        # every sandbox layer replaces with a fresh private tmpfs).
        # base_dir overrides that for controller-shaped fixtures that must
        # live under /tmp itself (the sandbox tmpfs is writable and the
        # validator re-exposes the invocation root's own chain there).
        # wrap_phases: required-gates-v2 refuses a phase-less manifest, so
        # single-gate fixtures are auto-wrapped in the eight-phase
        # product-green shell (every phase wires the fixture's one gate);
        # tests that deliberately build phase shapes pass wrap_phases=False
        # or carry their own [[phase]] blocks.
        if wrap_phases and "[[phase]]" not in manifest:
            match = re.search(r'\[\[gate\]\][^\[]*?id\s*=\s*"([^"]+)"', manifest)
            if not match:
                raise AssertionError("fixture bug: no gate id to wrap phases around")
            gate_id = match.group(1)
            manifest += "".join(
                f'[[phase]]\nid = "P{number}"\nstatus = "green"\n'
                f'required_gates = ["{gate_id}"]\n\n'
                for number in range(8)
            )
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
        # A fixture commit can dispatch a DETACHED `git maintenance run
        # --auto` that briefly creates and unlinks .git/objects/
        # maintenance.lock AFTER `git commit` returns. Anything that then
        # walks or stats the tree in good faith (the sealed-root test's
        # read-only walk) races that transient and flakes the suite --
        # which fails the gates-validator gate and with it the whole
        # completion validation. Fixtures never need background
        # maintenance; disable it at the source.
        subprocess.run([GIT, "-C", str(root), "config", "maintenance.auto", "false"], check=True)
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
        phase: str | None = None,
        phase_output: Path | None = None,
    ) -> tuple[int, dict]:
        report = report or root / "report.json"
        command = [sys.executable, str(VALIDATOR), "--root", str(root), "--report", str(report)]
        if phase is not None:
            command.extend(["--phase", phase])
        if phase_output is not None:
            command.extend(["--phase-output", str(phase_output)])
        result = subprocess.run(
            command,
            env=env,
            cwd=str(cwd or root),
            timeout=60,
        )
        return result.returncode, json.loads(report.read_text())

    @staticmethod
    def full_manifest(
        phases: list[tuple[str, list[str]]],
        gates: list[tuple[str, str]],
    ) -> str:
        """An eight-phase P0..P7 v2 manifest with explicit evidence classes."""
        blocks = ['schema="required-gates-v2"\n']
        for number, (status, required) in enumerate(phases):
            blocks.append(
                "[[phase]]\nid=\"P%d\"\nstatus=\"%s\"\nrequired_gates=[%s]\n\n"
                % (number, status, ", ".join('"%s"' % gate_id for gate_id in required))
            )
        for gate_id, evidence in gates:
            blocks.append(
                "[[gate]]\nevidence=\"%s\"\nid=\"%s\"\n"
                "command=[\"/usr/bin/true\"]\ntimeout_seconds=2\n\n" % (evidence, gate_id)
            )
        return "".join(blocks)

    def test_pass_is_deterministic_and_head_bound(self):
        root = self.fixture('schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="ok"\ncommand=["/usr/bin/test","-f","ok"]\ntimeout_seconds=2\n', {"ok": ""})
        rc, first = self.run_validator(root)
        rc2, second = self.run_validator(root)
        self.assertEqual((rc, rc2), (0, 0))
        self.assertEqual(first, second)
        self.assertEqual(first["schema"], "required-gates-report-v2")
        self.assertTrue(first["plan_complete"])

    def test_missing_corpus_fails_closed(self):
        root = self.fixture('schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="corpus"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\ncorpus=["missing.bin"]\n')
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("not tracked", report["error"])

    def test_shell_and_unlocked_cargo_are_rejected(self):
        for command in (
            '["curl","example.invalid"]',
            '["cargo","test"]',
            '["cargo","test","--locked"]',
        ):
            root = self.fixture(f'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="bad"\ncommand={command}\ntimeout_seconds=2\n')
            rc, _report = self.run_validator(root)
            self.assertNotEqual(rc, 0)

    def test_dirty_required_path_fails(self):
        root = self.fixture('schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\ntracked_paths=["proof.txt"]\n', {"proof.txt": "original\n"})
        (root / "proof.txt").write_text("changed\n")
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("differs from HEAD", report["error"])

    def test_missing_head_tracked_corpus_manifest_fails_closed(self):
        root = self.fixture(
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
        )
        subprocess.run([GIT, "-C", str(root), "rm", "-q", "MANIFEST.sha256"], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "remove manifest"], check=True)
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("MANIFEST.sha256", report["error"])

    def test_commands_require_absolute_allowlisted_executables(self):
        root = self.fixture(
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="relative"\ncommand=["true"]\ntimeout_seconds=2\n'
        )
        rc, _report = self.run_validator(root)
        self.assertNotEqual(rc, 0)

    def test_path_cannot_select_a_malicious_executable(self):
        root = self.fixture(
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="python"\ncommand=["python3","tools/gate.py"]\ntimeout_seconds=2\n',
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="env"\ncommand=["/usr/bin/python3","tools/env_gate.py"]\ntimeout_seconds=2\n',
            {"tools/env_gate.py": script},
        )
        env = dict(os.environ)
        for variable in variables:
            env[variable] = "/tmp/opencode/attacker-value"
        rc, report = self.run_validator(root, env=env)
        self.assertEqual(rc, 0, report)

    def test_dirty_reachable_gate_script_never_executes(self):
        root = self.fixture(
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="script"\ncommand=["/bin/bash","tools/gate.sh"]\ntimeout_seconds=2\n',
            {"tools/gate.sh": "#!/bin/bash\nexit 0\n"},
        )
        sentinel = root / "dirty-script-ran"
        (root / "tools/gate.sh").write_text(f"#!/bin/bash\ntouch {sentinel}\n")
        rc, _report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertFalse(sentinel.exists())

    def test_tracked_corpus_is_rechecked_after_each_command(self):
        root = self.fixture(
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="corpus"\ncommand=["/bin/bash","tools/mutate.sh"]\ntimeout_seconds=2\ncorpus=["corpus.bin"]\n',
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="head"\ncommand=["/bin/bash","tools/commit.sh"]\ntimeout_seconds=2\n',
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="smoke"\ncommand=["/bin/bash","tools/smoke.sh"]\ntimeout_seconds=2\n',
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
        # The descendant's observable touch must sit FAR after the reap
        # deadline, never on top of it. The validator reaps by collapsing
        # the bwrap PID namespace (killpg of the sandbox at timeout, or at
        # command exit), and that kill path legitimately takes
        # scheduler-dependent milliseconds: the Python timeout raise ->
        # killpg(SIGTERM) -> cleanup_group's own 50ms TERM->KILL spacing
        # -> SIGKILL -> namespace teardown. A "sleep 1" descendant against
        # a timeout_seconds=1 gate TIES the touch deadline to the kill
        # deadline, so a loaded completion host (the sealed runs peak
        # ~12G RSS plus swap while the cargo gates saturate the cores)
        # can schedule the awakened touch first and flake this suite ->
        # the gates-validator gate -> the whole completion validation
        # (the 2026-08-29T00:22Z completion-missing, the ONLY gate
        # failure in an otherwise all-green report). sleep 5 keeps the
        # pinned property exactly -- a reaped descendant must NEVER
        # touch -- with seconds of scheduling margin on both reap paths,
        # and the fail-fast poll still catches a surviving descendant
        # the moment it lands the sentinel.
        for mode in ("timeout", "success"):
            with self.subTest(mode=mode):
                sentinel_name = f"{mode}-descendant-ran"
                body = (
                    "#!/bin/bash\n"
                    f"setsid sh -c 'sleep 5; touch {sentinel_name}' >/dev/null 2>&1 &\n"
                    + ("sleep 30\n" if mode == "timeout" else "exit 0\n")
                )
                root = self.fixture(
                    f'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="{mode}"\ncommand=["/bin/bash","tools/group.sh"]\ntimeout_seconds=1\n',
                    {"tools/group.sh": body},
                )
                rc, _report = self.run_validator(root)
                if mode == "success":
                    self.assertEqual(rc, 0)
                deadline = time.monotonic() + 6.5
                while time.monotonic() < deadline:
                    self.assertFalse(
                        (root / sentinel_name).exists(),
                        f"reap failed in {mode} mode: descendant survived and touched {sentinel_name}",
                    )
                    time.sleep(0.1)

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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="w"\n'
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="w"\n'
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="c"\n'
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="env-probe"\n'
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="env-probe"\n'
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="env-probe"\n'
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
                    try:
                        os.chmod(os.path.join(current, name), 0o600)
                    except FileNotFoundError:
                        # A git-internal transient (lock/pid) removed
                        # itself mid-walk; nothing to restore.
                        pass

        self.addCleanup(unseal)
        for current, directories, files in os.walk(root, topdown=False):
            for name in files:
                path = os.path.join(current, name)
                try:
                    os.chmod(path, stat.S_IMODE(os.stat(path).st_mode) & ~0o222)
                except FileNotFoundError:
                    # The path vanished between os.walk listing it and
                    # this stat/chmod: a self-removing git-internal
                    # transient (e.g. objects/maintenance.lock from a
                    # detached auto-maintenance dispatch). A file that
                    # no longer exists needs no seal; skipping it is
                    # the correct read-only walk, not a weakened one.
                    continue
            for name in directories:
                path = os.path.join(current, name)
                try:
                    os.chmod(path, stat.S_IMODE(os.stat(path).st_mode) & ~0o222)
                except FileNotFoundError:
                    continue
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
        phase_toml = "".join(
            f'[[phase]]\nid = "P{number}"\nstatus = "green"\n'
            f'required_gates = ["ok"]\n\n'
            for number in range(8)
        )
        nested = (
            "import hashlib, json, os, pathlib, subprocess, sys, tempfile\n"
            "validator = pathlib.Path(__file__).with_name('validate-required-gates.py')\n"
            "root = pathlib.Path(tempfile.mkdtemp(prefix='nested-', "
            "dir=pathlib.Path(os.environ['HOME'])))\n"
            "(root / 'docs').mkdir()\n"
            "manifest = (\n"
            "    'schema=\"required-gates-v2\"\\n[[gate]]\\nevidence=\"product\"\\nid=\"ok\"\\n'\n"
            "    'command=[\"/usr/bin/true\"]\\ntimeout_seconds=10\\n' + " + repr(phase_toml) + ")\n"
            "(root / 'docs/required-gates.toml').write_text(manifest)\n"
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
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="nested"\n'
            'command=["/usr/bin/python3","tools/nested.py"]\ntimeout_seconds=60\n',
            {
                "tools/nested.py": nested,
                "tools/validate-required-gates.py": validator_source,
            },
        )
        rc, report = self.run_validator(root)
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])


    # ---- required-gates-v2 evidence contract (D238) -------------------
    # The v2 false-green rejection: the OLD manifest shape — no product
    # gates, corpus-skip-prone replay evidence, synthetic journeys — can
    # NEVER yield product or global completion, no matter how green its
    # commands or statuses claim to be. No existing repository gate is
    # relabeled product to make anything here pass: the only completing
    # shape is a synthetic fixture that wires a future product gate.

    NON_PRODUCT_CLASSES = (
        "supporting", "static", "paperwork", "synthetic", "corpus-required", "infrastructure",
    )

    def test_v1_manifest_schema_is_revoked(self):
        root = self.fixture(
            'schema="required-gates-v1"\n[[gate]]\nevidence="product"\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
        )
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("required-gates-v2", report["error"])
        self.assertIn("revoked", report["error"])
        self.assertFalse(report["plan_complete"])

    def test_gate_requires_a_known_evidence_classification(self):
        for evidence in ("artifact", ""):
            root = self.fixture(
                'schema="required-gates-v2"\n[[gate]]\nevidence="%s"\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n' % evidence
            )
            rc, report = self.run_validator(root)
            self.assertNotEqual(rc, 0, evidence)
            self.assertIn("evidence classification", report["error"])
        root = self.fixture(
            'schema="required-gates-v2"\n[[gate]]\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
        )
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("evidence classification", report["error"])

    def test_phase_status_vocabulary_is_enforced(self):
        pending = [("pending", [])] * 8
        for status, legal in (("bogus", False), ("complete", False)):
            root = self.fixture(
                self.full_manifest([("pending", [])] * 7 + [(status, ["g0"])], [("g0", "product")])
            )
            rc, report = self.run_validator(root)
            self.assertNotEqual(rc, 0, status)
            self.assertIn("status must be one of", report["error"])
        root = self.fixture(
            self.full_manifest([("engineering-green", ["g0"])] + pending[1:], [("g0", "supporting")])
        )
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertFalse(report["plan_complete"])
        self.assertTrue(any("engineering-green" in reason for reason in report["why_incomplete"]))

    def test_green_phase_without_product_gate_is_rejected_for_every_non_product_class(self):
        for evidence in self.NON_PRODUCT_CLASSES:
            with self.subTest(evidence=evidence):
                root = self.fixture(
                    self.full_manifest(
                        [("green", ["g0"])] + [("pending", [])] * 7,
                        [("g0", evidence)],
                    )
                )
                rc, report = self.run_validator(root)
                self.assertNotEqual(rc, 0)
                self.assertIn("no product gate is wired", report["error"])
                self.assertFalse(report["plan_complete"])

    def test_old_manifest_shape_never_yields_product_completion(self):
        # The pre-D238 lie, replayed as a fixture: P0-P3 gateless, P4-P7
        # wired with the six non-product classes, every command passing,
        # every status flipped green. v2 rejects it outright.
        classes = self.NON_PRODUCT_CLASSES
        gates = [("g%d" % number, classes[number % len(classes)]) for number in range(4)]
        phases = [("green", [])] * 4 + [("green", [gate_id]) for gate_id, _ in gates]
        root = self.fixture(self.full_manifest(phases, gates))
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertFalse(report["plan_complete"])
        self.assertIn("no product gate is wired", report["error"])

    def test_all_passing_non_product_gates_never_complete_the_plan(self):
        # The strongest legal non-product claim: every phase
        # engineering-green, every command rc=0. The plan stays open and
        # the report says exactly why.
        gates = [("g%d" % number, "supporting") for number in range(8)]
        phases = [("engineering-green", ["g%d" % number]) for number in range(8)]
        root = self.fixture(self.full_manifest(phases, gates))
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertFalse(report["plan_complete"])
        self.assertEqual(report["product_gates"], [])
        self.assertTrue(any("no product gate" in reason for reason in report["why_incomplete"]))
        self.assertTrue(all(coverage == 0 for coverage in report["phase_product_coverage"].values()))

    def test_product_gates_green_forward_shape_completes(self):
        # The one shape that can ever complete: every phase product-green
        # with a wired product gate, every command passing.
        gates = [("g%d" % number, "product") for number in range(8)]
        phases = [("green", ["g%d" % number]) for number in range(8)]
        root = self.fixture(self.full_manifest(phases, gates))
        rc, report = self.run_validator(root)
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])
        self.assertEqual(len(report["product_gates"]), 8)
        self.assertEqual(report["gates"][0]["evidence"], "product")
        self.assertEqual(report["why_incomplete"], [])
        self.assertEqual(report["phase_product_coverage"], {"P%d" % number: 1 for number in range(8)})

    def test_engineering_green_phase_verdict_never_claims_completion(self):
        # A bounded --phase run over an engineering-green phase writes a
        # phase-verdict-v2 artifact with product_complete false — even when
        # the output path is a legacy *-COMPLETE filename, the content
        # defeats the name. The old phase-complete-v1 authority is gone.
        root = self.fixture(
            self.full_manifest(
                [("pending", [])] * 7 + [("engineering-green", ["g7"])],
                [("g7", "supporting")],
            )
        )
        phase_output = root / "P7-COMPLETE"
        rc, report = self.run_validator(root, phase="P7", phase_output=phase_output)
        self.assertEqual(rc, 0, report)
        self.assertFalse(report["plan_complete"])
        verdict = json.loads(phase_output.read_text())
        self.assertEqual(verdict["schema"], "phase-verdict-v2")
        self.assertEqual(verdict["phase"], "P7")
        self.assertEqual(verdict["phase_status"], "engineering-green")
        self.assertTrue(verdict["engineering_complete"])
        self.assertFalse(verdict["product_complete"])

    def test_empty_phase_enumeration_never_completes(self):
        # The review-reproduced bypass, pinned: zero [[phase]] blocks plus
        # one product gate plus a trivially-true command must NEVER yield
        # product completion. A product plan must enumerate its phases;
        # the validator fails closed on the empty phase array.
        root = self.fixture(
            'schema="required-gates-v2"\n[[gate]]\nevidence="product"\nid="ok"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n',
            wrap_phases=False,
        )
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("non-empty gate and phase arrays", report["error"])
        self.assertFalse(report["plan_complete"])

    def test_duplicate_phase_ids_are_rejected(self):
        # Duplicate phase ids must not silently collapse (last-wins) in the
        # phase index while completion iterates every entry.
        manifest = (
            'schema="required-gates-v2"\n'
            '[[phase]]\nid="P0"\nstatus="green"\nrequired_gates=["g0"]\n'
            '[[phase]]\nid="P0"\nstatus="pending"\nrequired_gates=[]\n'
            + "".join(
                f'[[phase]]\nid="P{number}"\nstatus="pending"\nrequired_gates=[]\n'
                for number in range(1, 8)
            )
            + '[[gate]]\nevidence="product"\nid="g0"\ncommand=["/usr/bin/true"]\ntimeout_seconds=2\n'
        )
        root = self.fixture(manifest, wrap_phases=False)
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertIn("duplicate phase id", report["error"])
        self.assertFalse(report["plan_complete"])

    def test_all_product_phase_verdict_claims_no_engineering_coverage(self):
        # engineering_complete is fail-closed: a phase wiring only product
        # gates has no engineering evidence to claim, so the verdict must
        # not read as vacuously engineering-complete.
        root = self.fixture(
            self.full_manifest(
                [("green", ["g0"])] + [("pending", [])] * 7,
                [("g0", "product")],
            )
        )
        phase_output = root / "P0-verdict.json"
        rc, report = self.run_validator(root, phase="P0", phase_output=phase_output)
        self.assertEqual(rc, 0, report)
        verdict = json.loads(phase_output.read_text())
        self.assertFalse(verdict["engineering_complete"])
        self.assertTrue(verdict["product_complete"])

    def test_legacy_completion_markers_are_non_authoritative(self):
        # v1-era .state residue — PLAN-COMPLETE, P4..P7-COMPLETE, an old
        # report claiming a passed plan — must not move any v2 verdict:
        # the validator derives everything from tracked HEAD content and
        # leaves the residue byte-for-byte intact.
        root = self.fixture(
            self.full_manifest([("pending", [])] * 8, [("g0", "supporting")])
        )
        state = root / ".state"
        state.mkdir()
        legacy = {
            "PLAN-COMPLETE": '{"schema":"plan-complete-v1","status":"accepted"}',
            "P4-COMPLETE": '{"schema":"phase-complete-v1","phase":"P4"}',
            "P5-COMPLETE": '{"schema":"phase-complete-v1","phase":"P5"}',
            "P6-COMPLETE": '{"schema":"phase-complete-v1","phase":"P6"}',
            "P7-COMPLETE": '{"schema":"phase-complete-v1","phase":"P7"}',
            "required-gates-report.json": '{"schema":"required-gates-report-v1","status":"passed","plan_complete":true}',
        }
        before = {}
        for name, contents in legacy.items():
            marker = state / name
            marker.write_text(contents + "\n")
            before[name] = marker.read_bytes()
        rc, report = self.run_validator(root)
        self.assertNotEqual(rc, 0)
        self.assertFalse(report["plan_complete"])
        self.assertEqual(report["schema"], "required-gates-report-v2")
        for name, raw in before.items():
            self.assertEqual((state / name).read_bytes(), raw, name)


if __name__ == "__main__":
    unittest.main()
