#!/usr/bin/env python3
"""Hermetic contracts for the per-gate verdict cache (D239).

The cache is an opt-in accelerator behind validate-required-gates.py
--gate-cache. Gate commands run inside the validator's airtight sandbox
(read-only root, fresh writables, fresh /tmp), so NOTHING a command writes
survives a validator invocation -- the observable proof of (re-)execution
is the cache entry itself: remember_green_verdict() atomically REPLACES
the entry file (fresh inode) after every green execution, while a cache
hit leaves the entry byte-for-byte and inode-for-inode untouched. Every
"did it re-run?" assertion below rides that oracle.
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


VALIDATOR = Path(__file__).with_name("validate-required-gates.py")
# Gate commands run with no PATH: every spawned executable must be
# absolute, exactly like the validator's own allowlist.
GIT = "/usr/bin/git"
SHA256SUM = "/usr/bin/sha256sum"

GATE_CACHE_SCHEMA = "required-gate-cache-v1"
SINGLE_GATE = (
    '[[gate]]\nevidence="product"\nid="g"\n'
    'command=["/bin/bash","tools/gate.sh"]\ntimeout_seconds=30\n'
    'tracked_paths=["proof.txt"]\n'
)
SINGLE_GATE_FILES = {
    "tools/gate.sh": "#!/bin/bash\nexit 0\n",
    "proof.txt": "proof\n",
    "docs/notes.txt": "notes\n",
}


class GateCacheTests(unittest.TestCase):
    def fixture(
        self,
        manifest: str,
        files: dict[str, str] | None = None,
        ignore: list[str] | None = None,
        gate_ids: tuple[str, ...] = ("g",),
    ) -> Path:
        """A committed git fixture root with an eight-phase product shell."""
        # Fixtures live under HOME: inside the gates-validator gate that is
        # the validator's own repo-anchored scratch home (writable through
        # the sandbox, unlike its private tmpfs /tmp).
        base = Path(os.environ.get("HOME") or tempfile.gettempdir())
        root = Path(tempfile.mkdtemp(prefix="gate-cache-", dir=base))
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        phases = "".join(
            "[[phase]]\nid = \"P%d\"\nstatus = \"green\"\nrequired_gates = [%s]\n\n"
            % (number, ", ".join('"%s"' % gate_id for gate_id in gate_ids))
            for number in range(8)
        )
        (root / "docs").mkdir()
        (root / "docs/required-gates.toml").write_text(
            'schema="required-gates-v2"\n\n' + manifest + "\n" + phases
        )
        for name, contents in {**SINGLE_GATE_FILES, **(files or {})}.items():
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
        # A fixture commit can dispatch DETACHED auto-maintenance that
        # races later walks with transient .git lock files (D236).
        subprocess.run([GIT, "-C", str(root), "config", "maintenance.auto", "false"], check=True)
        subprocess.run([GIT, "-C", str(root), "add", "."], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "fixture"], check=True)
        return root

    def commit_file(self, root: Path, relative: str, message: str) -> None:
        subprocess.run([GIT, "-C", str(root), "add", "--", relative], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", message], check=True)

    def run_validator(
        self,
        root: Path,
        *,
        gate_cache: Path | str | None = None,
        report: Path | None = None,
        phase: str | None = None,
    ) -> tuple[int, dict]:
        report = report or root / "report.json"
        command = [sys.executable, str(VALIDATOR), "--root", str(root), "--report", str(report)]
        if gate_cache is not None:
            command.extend(["--gate-cache", str(gate_cache)])
        if phase is not None:
            command.extend(["--phase", phase])
        result = subprocess.run(command, cwd=str(root), timeout=120)
        return result.returncode, json.loads(report.read_text())

    @staticmethod
    def entry_path(root: Path, gate_id: str = "g") -> Path:
        return root / "cache" / (hashlib.sha256(gate_id.encode("utf-8")).hexdigest() + ".json")

    @staticmethod
    def entry_identity(path: Path) -> tuple[int, int]:
        """(inode, mtime_ns): the rewritten-on-execution oracle."""
        info = path.lstat()
        return (info.st_ino, info.st_mtime_ns)

    def test_cache_disabled_by_default_never_writes_or_reads(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        first = root / "one.json"
        second = root / "two.json"
        rc_one, _ = self.run_validator(root, report=first)
        rc_two, _ = self.run_validator(root, report=second)
        self.assertEqual((rc_one, rc_two), (0, 0))
        # Byte-identical reports, and no implicit cache directory at all.
        self.assertEqual(first.read_bytes(), second.read_bytes())
        self.assertFalse((root / "cache").exists())

    def test_first_run_is_a_miss_that_remembers_the_green(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])
        entry_file = self.entry_path(root)
        self.assertTrue(entry_file.is_file(), "a green gate must be remembered")
        entry = json.loads(entry_file.read_text())
        self.assertEqual(set(entry), {"schema", "id", "basis_sha256"})
        self.assertEqual(entry["schema"], GATE_CACHE_SCHEMA)
        self.assertEqual(entry["id"], "g")
        self.assertRegex(entry["basis_sha256"], r"^[0-9a-f]{64}$")

    def test_second_run_is_a_deterministic_hit(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        first = root / "one.json"
        second = root / "two.json"
        rc_one, report_one = self.run_validator(root, gate_cache="cache", report=first)
        entry_file = self.entry_path(root)
        self.assertEqual(rc_one, 0, report_one)
        identity = self.entry_identity(entry_file)
        contents = entry_file.read_bytes()
        rc_two, report_two = self.run_validator(root, gate_cache="cache", report=second)
        self.assertEqual(rc_two, 0)
        self.assertTrue(report_two["plan_complete"])
        # The hit replays the green verdict byte-identically...
        self.assertEqual(first.read_bytes(), second.read_bytes())
        # ...without re-executing anything: the remembered entry is not
        # even opened for writing (same inode, same mtime, same bytes).
        self.assertEqual(self.entry_identity(entry_file), identity)
        self.assertEqual(entry_file.read_bytes(), contents)

    def test_deterministic_hit_is_proven_by_entry_rewrite_absence(self):
        # The oracle in its strongest form: a forced miss (below) must
        # CHANGE the entry identity, proving hits above are meaningful.
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        _rc, _ = self.run_validator(root, gate_cache="cache")
        entry_file = self.entry_path(root)
        before = self.entry_identity(entry_file)
        # A cache hit changes nothing...
        _rc, _ = self.run_validator(root, gate_cache="cache")
        self.assertEqual(self.entry_identity(entry_file), before)
        # ...while deleting the entry forces a miss: the gate re-executes
        # and the fresh green verdict is remembered under a NEW inode.
        entry_file.unlink()
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertEqual(rc, 0, report)
        after = self.entry_identity(entry_file)
        self.assertNotEqual(after, before)

    def test_basis_change_re_runs_the_gate(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        _rc, _ = self.run_validator(root, gate_cache="cache")
        entry_file = self.entry_path(root)
        first = self.entry_identity(entry_file)

        # (a) the gate's own tracked path changes: new basis, re-run.
        (root / "proof.txt").write_text("proof v2\n")
        self.commit_file(root, "proof.txt", "gate input changed")
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertEqual(rc, 0, report)
        second = self.entry_identity(entry_file)
        self.assertNotEqual(second, first)

        # (b) an UNRELATED tracked change still re-runs: the whole HEAD
        # tree is bound into every gate's basis by design.
        (root / "docs/notes.txt").write_text("notes v2\n")
        self.commit_file(root, "docs/notes.txt", "unrelated tracked change")
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertEqual(rc, 0, report)
        self.assertNotEqual(self.entry_identity(entry_file), second)

    def test_corrupt_entry_fails_closed_to_a_re_run(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        _rc, _ = self.run_validator(root, gate_cache="cache")
        entry_file = self.entry_path(root)

        for corruption in (
            b"{not json",
            b"",
            json.dumps(
                {
                    "schema": GATE_CACHE_SCHEMA,
                    "id": "g",
                    "basis_sha256": "0" * 64,
                    "padding": " " * 5000,
                }
            ).encode() + b" " * 5000,  # oversized, even though it parses
        ):
            with self.subTest(corruption=corruption[:16]):
                entry_file.write_bytes(corruption)
                before = self.entry_identity(entry_file)
                rc, report = self.run_validator(root, gate_cache="cache")
                # Fail closed to a RE-RUN, never to a false verdict: the
                # gate executes again, goes green, and repairs the entry.
                self.assertEqual(rc, 0, report)
                self.assertTrue(report["plan_complete"])
                self.assertNotEqual(self.entry_identity(entry_file), before)
                entry = json.loads(entry_file.read_text())
                self.assertEqual(entry["schema"], GATE_CACHE_SCHEMA)
                self.assertEqual(entry["id"], "g")

    def test_poisoned_entries_are_refused(self):
        valid = {
            "schema": GATE_CACHE_SCHEMA,
            "id": "g",
            "basis_sha256": "0" * 64,
        }
        poisons = {
            "wrong-basis": dict(valid),
            "foreign-id": {**valid, "id": "other-gate"},
            "unknown-schema": {**valid, "schema": "required-gate-cache-v0"},
            "extra-verdict-key": {**valid, "verdict": "green"},
            "json-array": ["green"],
            "bare-string": "green",
        }
        for label, poison in poisons.items():
            with self.subTest(poison=label):
                root = self.fixture(SINGLE_GATE, ignore=["/cache"])
                entry_file = self.entry_path(root)
                entry_file.parent.mkdir(parents=True, exist_ok=True)
                entry_file.write_text(json.dumps(poison) + "\n")
                planted = self.entry_identity(entry_file)
                rc, report = self.run_validator(root, gate_cache="cache")
                # A foreign entry can never yield a green: the gate runs,
                # and only its own real green verdict is remembered.
                self.assertEqual(rc, 0, report)
                self.assertTrue(report["plan_complete"])
                self.assertNotEqual(self.entry_identity(entry_file), planted)
                entry = json.loads(entry_file.read_text())
                self.assertEqual(entry["id"], "g")
                self.assertNotEqual(entry["basis_sha256"], "0" * 64)

    def test_symlinked_entry_is_refused_even_with_valid_content(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        _rc, _ = self.run_validator(root, gate_cache="cache")
        entry_file = self.entry_path(root)
        # A fully valid entry, reachable only through a symlink: the cache
        # must not honor indirection -- refuse and re-run.
        aside = entry_file.with_name("aside.json")
        entry_file.rename(aside)
        entry_file.symlink_to(aside)
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertEqual(rc, 0, report)
        self.assertTrue(report["plan_complete"])
        self.assertFalse(entry_file.is_symlink(), "the symlink must be replaced by a fresh entry")
        self.assertEqual(entry_file.read_bytes(), aside.read_bytes())
        self.assertNotEqual(self.entry_identity(entry_file), self.entry_identity(aside))

    def test_dirty_tracked_path_still_fails_closed_despite_cache(self):
        # The cache never bypasses path-policy validation: a dirty tracked
        # path rejects the run before any cache consultation.
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        _rc, _ = self.run_validator(root, gate_cache="cache")
        (root / "proof.txt").write_text("dirty\n")
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertNotEqual(rc, 0)
        self.assertIn("differs from HEAD", report["error"])
        self.assertFalse(report["plan_complete"])

    def test_dependency_failure_writes_no_entries_and_stays_red(self):
        manifest = (
            '[[gate]]\nevidence="product"\nid="a"\n'
            'command=["/usr/bin/test","-f","no-such-file"]\ntimeout_seconds=30\n'
            '[[gate]]\nevidence="product"\nid="b"\n'
            'command=["/bin/bash","tools/gate.sh"]\ntimeout_seconds=30\n'
            'depends=["a"]\n'
        )
        root = self.fixture(manifest, ignore=["/cache"], gate_ids=("a", "b"))
        for attempt in (1, 2):
            with self.subTest(attempt=attempt):
                rc, report = self.run_validator(root, gate_cache="cache")
                self.assertNotEqual(rc, 0)
                self.assertFalse(report["plan_complete"])
                verdicts = {gate["id"]: gate["passed"] for gate in report["gates"]}
                self.assertEqual(verdicts, {"a": False, "b": False})
                # Only greens are ever remembered: the failing gate and the
                # dependency-failed gate leave the cache empty, so redness
                # is re-derived live on every run.
                cache_dir = root / "cache"
                entries = sorted(path.name for path in cache_dir.glob("*.json")) if cache_dir.is_dir() else []
                self.assertEqual(entries, [])

    def test_phase_run_reuses_the_full_run_verdict(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        _rc, _ = self.run_validator(root, gate_cache="cache")
        entry_file = self.entry_path(root)
        before = self.entry_identity(entry_file)
        rc, report = self.run_validator(root, gate_cache="cache", phase="P3")
        self.assertEqual(rc, 0, report)
        self.assertFalse(report["plan_complete"])  # bounded phase runs never complete
        self.assertEqual(self.entry_identity(entry_file), before)

    def test_out_of_root_host_cache_dir_is_allowed(self):
        # The controller shape: a host-side persistent cache outside the
        # (possibly sealed, read-only) invocation root. Content-keyed
        # verdicts make it usable across roots at the same HEAD.
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        base = Path(os.environ.get("HOME") or tempfile.gettempdir())
        outside = Path(tempfile.mkdtemp(prefix="gate-cache-host-", dir=base))
        self.addCleanup(shutil.rmtree, outside, ignore_errors=True)
        rc_one, _ = self.run_validator(root, gate_cache=outside)
        rc_two, _ = self.run_validator(root, gate_cache=outside)
        self.assertEqual((rc_one, rc_two), (0, 0))
        entry_file = outside / (hashlib.sha256(b"g").hexdigest() + ".json")
        self.assertTrue(entry_file.is_file())
        before = entry_file.read_bytes()
        identity = self.entry_identity(entry_file)
        _rc, _ = self.run_validator(root, gate_cache=outside)
        self.assertEqual(entry_file.read_bytes(), before)
        self.assertEqual(self.entry_identity(entry_file), identity)

    def test_in_root_cache_path_must_be_gitignored(self):
        root = self.fixture(SINGLE_GATE)  # no /cache/ ignore rule
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertNotEqual(rc, 0)
        self.assertIn("gitignored", report["error"])
        self.assertFalse(report["plan_complete"])

    def test_in_root_cache_refuses_tracked_content(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        (root / "cache").mkdir()
        (root / "cache/seed.txt").write_text("tracked\n")
        # Tracked content can exist under an ignored path only when
        # force-added -- exactly the evidence-fabrication shape refused.
        subprocess.run([GIT, "-C", str(root), "add", "-f", "--", "cache/seed.txt"], check=True)
        subprocess.run([GIT, "-C", str(root), "commit", "-qm", "seed the cache"], check=True)
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertNotEqual(rc, 0)
        self.assertIn("must not contain tracked files", report["error"])

    def test_in_root_cache_refuses_symlink_traversal(self):
        root = self.fixture(SINGLE_GATE, ignore=["/cache"])
        target = Path(tempfile.mkdtemp(prefix="gate-cache-target-"))
        self.addCleanup(shutil.rmtree, target, ignore_errors=True)
        (root / "cache").symlink_to(target)
        rc, report = self.run_validator(root, gate_cache="cache")
        self.assertNotEqual(rc, 0)
        self.assertIn("traverses a symlink", report["error"])


if __name__ == "__main__":
    unittest.main()
