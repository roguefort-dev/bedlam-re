#!/usr/bin/env python3
"""Fast regression for the opt-in responsive O1 capture command flow."""

import importlib.util
import json
import os
import re
import tempfile
import unittest
from types import SimpleNamespace
from unittest import mock


CAPGEN_PATH = os.path.join(os.path.dirname(__file__), "dbx-capgen.py")
SPEC = importlib.util.spec_from_file_location("dbx_capgen", CAPGEN_PATH)
CAPGEN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CAPGEN)
REAL_LOGTAIL = CAPGEN.LogTail


class FakeSession:
    def __init__(self, _argv, _cwd, _env, _log_path):
        pass

    def alive(self):
        return False

    def close(self):
        pass


class FakeLogTail:
    active = None
    entry_eip = 0x0005FBB0

    def __init__(self, _path):
        self.log = bytearray(b"TYPE HELP\n")
        self.breakpoints = []
        FakeLogTail.active = self

    def snapshot(self):
        return bytes(self.log)

    def wait_present(self, pattern, timeout):
        return re.search(pattern, self.log, re.DOTALL) is not None

    def expect(self, pattern, timeout):
        if re.search(pattern, self.log, re.DOTALL) is None:
            raise AssertionError(f"missing simulated logfile pattern {pattern!r}")
        return 1

    def expect_bracket(self, begin, end, timeout):
        response = REAL_LOGTAIL._bracketed_response(bytes(self.log), begin, end)
        if response is None:
            raise AssertionError(f"missing simulated logfile bracket {begin!r}/{end!r}")
        return response

    def expect_new_marker(self, marker, before, timeout):
        current = bytes(self.log)
        if not current.startswith(before):
            raise AssertionError("simulated logfile was replaced")
        fresh = current[len(before) :]
        if fresh.count(f"NOTICE: {marker}\n".encode()) != 1:
            raise AssertionError(f"missing fresh simulated marker {marker!r}")
        return current

    def last_match(self, pattern):
        matches = re.findall(pattern, self.log)
        return matches[-1] if matches else None

    def record(self, command):
        if command.startswith("ADDLOG "):
            self.log.extend(f"NOTICE: {command.removeprefix('ADDLOG ')}\n".encode())
        elif command == "BPINT 21 4B":
            self.breakpoints.insert(0, "BPINT 21 AH=4B")
            self.log.extend(b"DEBUG: Set interrupt breakpoint at INT 21 AH=4B\n")
        elif command == "BPINT 8":
            self.breakpoints.insert(0, "BPINT 08")
            self.log.extend(b"DEBUG: Set interrupt breakpoint at INT 08\n")
        elif command == "BPDEL *":
            self.breakpoints.clear()
            self.log.extend(b"DEBUG: Breakpoints deleted.\n")
        elif command == "BP 5FBB:0000":
            self.breakpoints.insert(0, "BP 5FBB:0000")
            self.log.extend(b"DEBUG: Set breakpoint at 5FBB:0000\n")
        elif command == "BP CS:0005A6EB":
            self.breakpoints.insert(0, "BP 01AF:5A6EB")
            self.log.extend(b"DEBUG: Set breakpoint at 01AF:5A6EB\n")
        elif command == "BPLM 46C":
            self.breakpoints.insert(0, "BPLM 0000046C (00)")
            self.log.extend(b"DEBUG: Set linear memory breakpoint at 0000046C\n")
        elif command == "BPLIST":
            self.log.extend(b"Breakpoint list:\n")
            self.log.extend(b"-------------------------------------------------------------------------\n")
            for index, entry in enumerate(self.breakpoints):
                self.log.extend(f"{index:02X}. {entry}\n".encode())
        elif command == "EV CS EIP CR0":
            self.log.extend(b"EV of 'CS EIP CR0' is:\n")
            self.log.extend(f"1af {self.entry_eip:x} 1\n".encode())
        elif command == "SELINFO CS":
            self.log.extend(b"SelectorInfo CS:\n")
            self.log.extend(b"CS: b:00000000 type:1B parbg\n")
            self.log.extend(b"    l:FFFFFFFF dpl : 0 11111\n")


class ZeroOverlapRewriteLog:
    """Replaces the logfile while retaining only stale target output."""

    def __init__(self, stale):
        self.log = bytearray(b"unrelated original logfile\n")
        self.stale = stale

    def record(self, command):
        if command.startswith("ADDLOG "):
            marker = command.removeprefix("ADDLOG ")
            if marker.endswith("_BEGIN"):
                self.log = bytearray(self.stale)
            self.log.extend(f"NOTICE: {marker}\n".encode())

    def expect_bracket(self, begin, end, timeout):
        response = REAL_LOGTAIL._bracketed_response(bytes(self.log), begin, end)
        if response is None:
            raise AssertionError("test rewrite did not produce a complete bracket")
        return response


class ResponsiveCaptureTest(unittest.TestCase):
    def setUp(self):
        FakeLogTail.entry_eip = 0x0005FBB0

    def run_plan(self, plan):
        commands = []
        token_counter = iter(range(1, 100))
        with tempfile.TemporaryDirectory() as tmp:
            plan_path = os.path.join(tmp, "plan.json")
            out_path = os.path.join(tmp, "capture.dbxcap")
            with open(plan_path, "w") as out:
                json.dump(plan, out)
            args = SimpleNamespace(
                plan=plan_path,
                frames=None,
                time_limit=0,
                workdir=tmp,
                dbx="unused-dosbox-x",
                conf="unused.conf",
                out=out_path,
                hit_timeout=1,
                probe=False,
            )

            def record_command(_sess, command, settle=1.0):
                commands.append(command)
                FakeLogTail.active.record(command)

            def record_resume(
                _sess, _dblog, command, timeout, stop_signal="redraw"
            ):
                if command == "RUNWATCH" and stop_signal != "memory":
                    raise AssertionError("RUNWATCH requires the memory-hit stop signal")
                if command == "RUN" and stop_signal != "redraw":
                    raise AssertionError("RUN requires the redraw-candidate stop signal")
                commands.append(command)
                token = CAPGEN.secrets.token_hex(16).upper()
                commands.append(f"ADDLOG CAPGEN_STOP_{token}_PROBE_0001")

            with (
                mock.patch.object(CAPGEN, "PtySession", FakeSession),
                mock.patch.object(CAPGEN, "LogTail", FakeLogTail),
                mock.patch.object(CAPGEN, "send_cmd", side_effect=record_command),
                mock.patch.object(
                    CAPGEN, "resume_until_hit", side_effect=record_resume
                ),
                mock.patch.object(
                    CAPGEN.secrets,
                    "token_hex",
                    side_effect=lambda _size: f"{next(token_counter):032x}",
                ),
                mock.patch.object(
                    CAPGEN,
                    "dump_watch",
                    side_effect=lambda _sess, _log, _workdir, _dest, _addr, length: bytes(
                        length
                    ),
                ),
            ):
                CAPGEN.run_capture(args)
        return commands

    @staticmethod
    def bracket(index, command):
        token = f"{index:032X}"
        return [
            f"ADDLOG CAPGEN_{token}_BEGIN",
            command,
            f"ADDLOG CAPGEN_{token}_END",
        ]

    @staticmethod
    def stop_probe(index, command):
        token = f"{index:032X}"
        return [
            command,
            f"ADDLOG CAPGEN_STOP_{token}_PROBE_0001",
        ]

    @staticmethod
    def entry_plan():
        return {
            "boot_trap": "entry",
            "arm_commands": [
                {"cmd": "BPDEL *", "expect": "Breakpoints deleted"},
                {"cmd": "BP CS:0005A6EB", "expect": "Set breakpoint at"},
            ],
            "resolve_at": "anchor",
            "anchor_watches": [
                {"id": "counter", "addr": "CS:001195F0", "len": 4}
            ],
            "watches": [{"id": "counter", "addr": "CS:001195F0", "len": 4}],
            "frames": 3,
            "logfile": "capture.log",
        }

    def test_live_entry_plan_uses_verified_code_breakpoints_and_normal_run(self):
        """The opt-in live path never enters the heavy BPLM/RUNWATCH loop."""
        commands = self.run_plan(self.entry_plan())

        self.assertEqual(
            commands,
            self.bracket(1, "BPINT 21 4B")
            + self.stop_probe(2, "RUN")
            + self.bracket(3, "BPDEL *")
            + self.bracket(4, "BPLIST")
            + self.bracket(5, "BP 5FBB:0000")
            + self.bracket(6, "BPLIST")
            + self.stop_probe(7, "RUN")
            + self.bracket(8, "EV CS EIP CR0")
            + self.bracket(9, "SELINFO CS")
            + self.bracket(10, "BPDEL *")
            + self.bracket(11, "BPLIST")
            + self.bracket(12, "BPDEL *")
            + self.bracket(13, "BPLIST")
            + self.bracket(14, "BP CS:0005A6EB")
            + self.bracket(15, "BPLIST")
            + self.stop_probe(16, "RUN")
            + self.stop_probe(17, "RUN")
            + self.stop_probe(18, "RUN"),
        )
        self.assertNotIn("RUNWATCH", commands)
        self.assertFalse(any(command.startswith("BPLM") for command in commands))

    def test_entry_stop_register_mismatch_fails_closed(self):
        FakeLogTail.entry_eip = 0x0005FBAF
        with self.assertRaisesRegex(RuntimeError, "entry stop EIP mismatch"):
            self.run_plan(self.entry_plan())

    def test_legacy_flow_retains_runwatch_only_for_bplm_stop(self):
        plan = {
            "flat_guard": False,
            "boot_commands": [
                {
                    "cmd": "BPLM 46C",
                    "expect": "Set linear memory breakpoint at 0000046C",
                }
            ],
            "arm_commands": [
                {"cmd": "BPDEL *", "expect": "Breakpoints deleted"},
                {
                    "cmd": "BPINT 8",
                    "expect": "Set interrupt breakpoint at INT 08",
                },
            ],
            "anchor_watches": [{"id": "anchor", "addr": "0000:0000", "len": 1}],
            "watches": [{"id": "frame", "addr": "0000:0001", "len": 1}],
            "frames": 3,
            "logfile": "capture.log",
        }
        commands = self.run_plan(plan)
        self.assertEqual(
            commands,
            ["BPLM 46C"]
            + self.stop_probe(1, "RUNWATCH")
            + ["BPDEL *", "BPINT 8"]
            + self.stop_probe(2, "RUN")
            + self.stop_probe(3, "RUN")
            + self.stop_probe(4, "RUN"),
        )
        self.assertNotIn("EV CS EIP CR0", commands)
        self.assertNotIn("BP 5FBB:0000", commands)

    def test_bplist_requires_complete_heading_and_separator(self):
        with self.assertRaisesRegex(RuntimeError, "incomplete BPLIST"):
            CAPGEN._parse_breakpoint_list(b"Breakpoint list:\n")

    def test_bplist_rejects_malformed_and_noncontiguous_rows(self):
        separator = b"-" * 73
        with self.assertRaisesRegex(RuntimeError, "malformed BPLIST row"):
            CAPGEN._parse_breakpoint_list(
                b"Breakpoint list:\n" + separator + b"\n00. UNKNOWN row\n"
            )
        with self.assertRaisesRegex(RuntimeError, "non-contiguous BPLIST index"):
            CAPGEN._parse_breakpoint_list(
                b"Breakpoint list:\n" + separator + b"\n01. BP 01AF:5A6EB\n"
            )

    def test_bracket_survives_prefix_drop_but_rejects_lost_begin_marker(self):
        begin = "CAPGEN_TEST_BEGIN"
        end = "CAPGEN_TEST_END"
        wrapped = (
            b"NOTICE: CAPGEN_TEST_BEGIN\n"
            b"Breakpoint list:\n"
            + b"-" * 73
            + b"\n00. BP 01AF:5A6EB\nNOTICE: CAPGEN_TEST_END\n"
        )
        self.assertEqual(
            REAL_LOGTAIL._bracketed_response(wrapped, begin, end),
            b"Breakpoint list:\n" + b"-" * 73 + b"\n00. BP 01AF:5A6EB\n",
        )
        self.assertIsNone(
            REAL_LOGTAIL._bracketed_response(
                b"NOTICE: CAPGEN_TEST_BEGIN\nBreakpoint list:\n" + b"-" * 73,
                begin,
                end,
            )
        )
        with self.assertRaisesRegex(RuntimeError, "begin marker"):
            REAL_LOGTAIL._bracketed_response(
                b"stale response\nNOTICE: CAPGEN_TEST_END\n", begin, end
            )

    def test_zero_overlap_rewrite_cannot_reuse_stale_strict_responses(self):
        cases = [
            (
                "EV CS EIP CR0",
                rb"EV of 'CS EIP CR0' is:",
                b"EV of 'CS EIP CR0' is:\n1af 5fbb0 1\n",
            ),
            (
                "SELINFO CS",
                rb"SelectorInfo CS:",
                b"SelectorInfo CS:\nCS: b:00000000\n    l:FFFFFFFF\n",
            ),
            (
                "BPLIST",
                rb"Breakpoint list:",
                b"Breakpoint list:\n" + b"-" * 73 + b"\n",
            ),
        ]
        for command, pattern, stale in cases:
            with self.subTest(command=command):
                dblog = ZeroOverlapRewriteLog(stale)
                with (
                    mock.patch.object(
                        CAPGEN,
                        "send_cmd",
                        side_effect=lambda _sess, cmd: dblog.record(cmd),
                    ),
                    mock.patch.object(
                        CAPGEN.secrets, "token_hex", return_value="a" * 32
                    ),
                    self.assertRaisesRegex(RuntimeError, "fresh response mismatch"),
                ):
                    CAPGEN.fresh_command(object(), dblog, command, pattern, timeout=0.1)


if __name__ == "__main__":
    unittest.main()
