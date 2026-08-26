#!/usr/bin/env python3
"""Regressions for source-correct post-reentry readiness probes."""

import importlib.util
import os
import unittest
from unittest import mock


CAPGEN_PATH = os.path.join(os.path.dirname(__file__), "dbx-capgen.py")
SPEC = importlib.util.spec_from_file_location("dbx_capgen_resume", CAPGEN_PATH)
CAPGEN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CAPGEN)


class FakeClock:
    def __init__(self):
        self.now = 0.0

    def monotonic(self):
        return self.now

    def sleep(self, duration):
        if duration < 0:
            raise AssertionError(f"negative fake sleep: {duration}")
        self.now += duration


class PinnedLog:
    """Logfile side of a deterministic DOSBox-X debugger model."""

    expect_new_marker = CAPGEN.LogTail.expect_new_marker
    expect_new_pattern = CAPGEN.LogTail.expect_new_pattern

    def __init__(self, stale=b""):
        self.data = bytearray(b"TYPE HELP\n" + stale)
        self.session = None

    def snapshot(self):
        return bytes(self.data)

    def append_marker(self, marker):
        self.data.extend(f"NOTICE: {marker}\n".encode())

    def append_memory_hit(self):
        self.data.extend(b"DEBUG: Memory breakpoint : 0000:046C - 00 -> AA\n")


class PinnedDebuggerSession:
    """PTY model with DEBUG_FlushInput ordering and controllable races."""

    REDRAW_RE = CAPGEN.PtySession.REDRAW_RE
    redraw_after = CAPGEN.PtySession.redraw_after
    wait_redraw = CAPGEN.PtySession.wait_redraw

    def __init__(self, dblog, mode="flushed-retry", clock=None):
        self.dblog = dblog
        dblog.session = self
        self.mode = mode
        self.clock = clock
        self.output = bytearray()
        self.commands = []
        self.command_times = []
        self.running = False
        self.reentered = False
        self.pending_input = []
        self.ready_input = []
        self.flushed_markers = []
        self.authoritative_marker = None
        self.redraw_candidates = []
        self.split_tail = b""
        self.split_reads = 0

    def output_mark(self):
        return len(self.output)

    def send_marked(self, command):
        mark = self.output_mark()
        if (
            command.startswith("ADDLOG ")
            and self.mode == "marker-during-wait"
            and self.running
        ):
            self._breakpoint_reentry()
        self.send(command)
        return mark

    def send(self, command):
        self.commands.append(command)
        self.command_times.append(
            self.clock.monotonic() if self.clock is not None else None
        )
        if command == "RUN":
            self.running = True
            self._append_redraw("run-command-redraw", b"RUN\r\n01AF:0005FBB0\r\n")
            if self.mode == "fast-combined":
                self._breakpoint_reentry()
            return
        if command == "RUNWATCH":
            self.running = True
            self._append_redraw("runwatch-redraw", b"01AF:0005FBB0\r\n")
            self.dblog.append_memory_hit()
            return
        if command.startswith("ADDLOG "):
            marker = command.removeprefix("ADDLOG ")
            if self.running:
                self.pending_input.append(marker)
            elif self.mode == "marker-during-wait" and not self.authoritative_marker:
                self.ready_input.append(marker)
            else:
                self._log_marker(marker)
            return
        raise AssertionError(f"unexpected debugger command {command!r}")

    def _output_after(self, mark):
        if self.split_tail and self.split_reads:
            self.output.extend(self.split_tail)
            self.split_tail = b""
        elif self.split_tail:
            self.split_reads += 1
        return bytes(self.output[mark:]), len(self.output)

    def _append_redraw(self, label, data):
        self.redraw_candidates.append(label)
        self.output.extend(data)

    def _breakpoint_reentry(self):
        # debug.cpp calls DEBUG_FlushInput before the re-entry redraw.
        self.flushed_markers.extend(self.pending_input)
        self.pending_input.clear()
        self.running = False
        self.reentered = True
        if self.mode == "split-reentry":
            self._append_redraw("breakpoint-reentry-redraw", b"stop\r\n01AF:")
            self.split_tail = b"0005A6EB\r\n"
        else:
            self._append_redraw(
                "breakpoint-reentry-redraw", b"stop\r\n01AF:0005A6EB\r\n"
            )

    def _log_marker(self, marker):
        self.dblog.append_marker(marker)
        self.authoritative_marker = marker

    def advance_marker_wait(self):
        if self.ready_input:
            self._log_marker(self.ready_input.pop(0))
        elif self.running:
            self._breakpoint_reentry()


def expect_new_marker_with_debugger_progress(self, marker, before, timeout):
    """Run one deterministic event while production waits for the marker."""
    self.session.advance_marker_wait()
    current = self.snapshot()
    if not current.startswith(before):
        raise RuntimeError("capgen: logfile replaced or truncated")
    line = f"NOTICE: {marker}\n".encode()
    if current[len(before) :].count(line) != 1:
        raise TimeoutError(f"marker {marker} was flushed")
    return current


class ResumeUntilHitTest(unittest.TestCase):
    TOKEN = "AB" * 16

    def run_resume(self, mode="flushed-retry", command="RUN", stop_signal="redraw"):
        dblog = PinnedLog()
        session = PinnedDebuggerSession(dblog, mode=mode)
        with (
            mock.patch.object(CAPGEN.time, "sleep", return_value=None),
            mock.patch.object(CAPGEN.secrets, "token_hex", return_value=self.TOKEN),
            mock.patch.object(
                PinnedLog,
                "expect_new_marker",
                expect_new_marker_with_debugger_progress,
            ),
        ):
            CAPGEN.resume_until_hit(
                session, dblog, command, timeout=1, stop_signal=stop_signal
            )
        return session

    def test_flushed_probe_is_retried_fresh_after_reentry(self):
        session = self.run_resume()
        first = f"CAPGEN_STOP_{self.TOKEN}_PROBE_0001"
        second = f"CAPGEN_STOP_{self.TOKEN}_PROBE_0002"

        self.assertEqual(
            session.commands, ["RUN", f"ADDLOG {first}", f"ADDLOG {second}"]
        )
        self.assertEqual(session.flushed_markers, [first])
        self.assertEqual(session.authoritative_marker, second)
        self.assertEqual(
            session.redraw_candidates,
            ["run-command-redraw", "breakpoint-reentry-redraw"],
        )

    def test_fast_stop_combined_redraw_needs_only_one_post_stop_probe(self):
        session = self.run_resume(mode="fast-combined")
        marker = f"CAPGEN_STOP_{self.TOKEN}_PROBE_0001"

        self.assertEqual(session.commands, ["RUN", f"ADDLOG {marker}"])
        self.assertEqual(session.flushed_markers, [])
        self.assertEqual(session.authoritative_marker, marker)

    def test_marker_logged_during_wait_is_authoritative(self):
        session = self.run_resume(mode="marker-during-wait")
        marker = f"CAPGEN_STOP_{self.TOKEN}_PROBE_0001"

        self.assertEqual(session.commands, ["RUN", f"ADDLOG {marker}"])
        self.assertEqual(session.authoritative_marker, marker)
        self.assertTrue(session.reentered)

    def test_split_reentry_redraw_is_not_lost_after_probe_timeout(self):
        session = self.run_resume(mode="split-reentry")

        self.assertEqual(len(session.flushed_markers), 1)
        self.assertTrue(session.authoritative_marker.endswith("PROBE_0002"))
        self.assertEqual(session.split_tail, b"")

    def test_memory_hit_is_first_stage_then_flushed_probe_is_retried(self):
        session = self.run_resume(
            command="RUNWATCH", stop_signal="memory", mode="flushed-retry"
        )

        self.assertEqual(session.commands[0], "RUNWATCH")
        self.assertEqual(len(session.flushed_markers), 1)
        self.assertTrue(session.authoritative_marker.endswith("PROBE_0002"))

    def test_stale_probe_nonce_fails_before_probe_is_sent(self):
        stale_marker = f"CAPGEN_STOP_{self.TOKEN}_PROBE_0001"
        dblog = PinnedLog(stale=f"NOTICE: {stale_marker}\n".encode())
        session = PinnedDebuggerSession(dblog, mode="fast-combined")

        with (
            mock.patch.object(CAPGEN.time, "sleep", return_value=None),
            mock.patch.object(CAPGEN.secrets, "token_hex", return_value=self.TOKEN),
            self.assertRaisesRegex(RuntimeError, "nonce already exists"),
        ):
            CAPGEN.resume_until_hit(session, dblog, "RUN", timeout=1)

        self.assertEqual(session.commands, [])

    def test_replacement_containing_marker_cannot_make_it_fresh(self):
        marker = f"CAPGEN_STOP_{self.TOKEN}_PROBE_0001"
        before = b"original logfile\n"
        replacement = f"stale replacement\nNOTICE: {marker}\n".encode()
        dblog = CAPGEN.LogTail("unused")

        with (
            mock.patch.object(dblog, "snapshot", return_value=replacement),
            self.assertRaisesRegex(RuntimeError, "replaced or truncated"),
        ):
            dblog.expect_new_marker(marker, before, timeout=1)

    def test_send_marked_captures_boundary_with_the_command_write(self):
        session = CAPGEN.PtySession.__new__(CAPGEN.PtySession)
        session.master = 7
        session.total_bytes = 123
        session._io_lock = CAPGEN.threading.Lock()
        payload = b"ADDLOG probe\r"

        with mock.patch.object(CAPGEN.os, "write", return_value=len(payload)) as write:
            mark = session.send_marked("ADDLOG probe")

        self.assertEqual(mark, 123)
        write.assert_called_once_with(7, mock.ANY)
        self.assertEqual(bytes(write.call_args.args[1]), payload)

    def test_global_deadline_includes_settle_and_blocks_resume_write(self):
        clock = FakeClock()
        dblog = PinnedLog()
        session = PinnedDebuggerSession(dblog, mode="fast-combined", clock=clock)

        with (
            mock.patch.object(CAPGEN.time, "monotonic", side_effect=clock.monotonic),
            mock.patch.object(CAPGEN.time, "sleep", side_effect=clock.sleep),
            mock.patch.object(CAPGEN.secrets, "token_hex", return_value=self.TOKEN),
            self.assertRaisesRegex(TimeoutError, "resume command after settle"),
        ):
            CAPGEN.resume_until_hit(session, dblog, "RUN", timeout=0.5)

        self.assertEqual(clock.monotonic(), 1.0)
        self.assertEqual(session.commands, [])

    def test_expiry_during_probe_preparation_emits_no_addlog(self):
        clock = FakeClock()

        class ExpiringLog(PinnedLog):
            def __init__(self):
                super().__init__()
                self.snapshots = 0

            def snapshot(self):
                self.snapshots += 1
                if self.snapshots == 2:
                    clock.sleep(0.6)
                return super().snapshot()

        dblog = ExpiringLog()
        session = PinnedDebuggerSession(dblog, mode="fast-combined", clock=clock)
        with (
            mock.patch.object(CAPGEN.time, "monotonic", side_effect=clock.monotonic),
            mock.patch.object(CAPGEN.time, "sleep", side_effect=clock.sleep),
            mock.patch.object(CAPGEN.secrets, "token_hex", return_value=self.TOKEN),
            self.assertRaisesRegex(TimeoutError, "before probe write"),
        ):
            CAPGEN.resume_until_hit(session, dblog, "RUN", timeout=1.5)

        self.assertEqual(session.commands, ["RUN"])
        self.assertEqual(session.command_times, [1.0])

    def test_memory_retry_has_bounded_probe_count_and_no_late_write(self):
        clock = FakeClock()
        deadline = 3.2
        dblog = PinnedLog()
        session = PinnedDebuggerSession(dblog, clock=clock)

        def consume_probe_window(_log, _marker, _before, timeout):
            clock.sleep(timeout + 0.001)
            raise TimeoutError("probe remained unready")

        with (
            mock.patch.object(CAPGEN.time, "monotonic", side_effect=clock.monotonic),
            mock.patch.object(CAPGEN.time, "sleep", side_effect=clock.sleep),
            mock.patch.object(CAPGEN.secrets, "token_hex", return_value=self.TOKEN),
            mock.patch.object(
                PinnedLog,
                "expect_new_marker",
                autospec=True,
                side_effect=consume_probe_window,
            ),
            self.assertRaisesRegex(TimeoutError, "debugger readiness"),
        ):
            CAPGEN.resume_until_hit(
                session,
                dblog,
                "RUNWATCH",
                timeout=deadline,
                stop_signal="memory",
            )

        probes = [
            (command, sent_at)
            for command, sent_at in zip(session.commands, session.command_times)
            if command.startswith("ADDLOG ")
        ]
        self.assertEqual(len(probes), 3)
        self.assertEqual(
            [command.rsplit("_", 1)[-1] for command, _ in probes],
            ["0001", "0002", "0003"],
        )
        self.assertTrue(all(sent_at < deadline for _, sent_at in probes))
        self.assertEqual(session.commands.count("RUNWATCH"), 1)


if __name__ == "__main__":
    unittest.main()
