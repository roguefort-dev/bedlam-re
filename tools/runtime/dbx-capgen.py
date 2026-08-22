#!/usr/bin/env python3
"""dbx-capgen — the DH-G0 O1 capture-channel driver (D80).

Drives the SELF-BUILT debug DOSBox-X (runtime/dosbox-x-build/src/dosbox-x,
--enable-debug=heavy, commit e522642; see docs/RUNTIME.md "DH-G0 channel
re-pin") under a host PTY and emits the channel-agnostic DBXCAP capture
transcript that tools/diffharness/src/bin/dbx-stitch consumes (W4).

Channel facts this driver is built on (source-pinned at e522642):
- The Linux debugger refuses to open unless isatty(0/1/2)
  (src/debug/debug.cpp DEBUG_Enable_Handler) → run under a PTY.
- Every command ack goes through DEBUG_ShowMsg, which ALSO writes the
  [log] logfile line-oriented with fflush (src/debug/debug_gui.cpp:744) —
  acks are read from the LOGFILE, never scraped off the ncurses screen
  (redraws re-emit old pane text; screen-scraping is unreliable).
- Input while the machine runs (RUNWATCH) queues in the tty buffer; the
  queued command executes at the next stop. A stop emits a screen-redraw
  burst on the PTY — used as the hit observable, with a proceed-anyway
  fallback because the ack validation (logfile) catches a machine that
  never stopped.

Capture loop (one guest frame per iteration):
    RUNWATCH                       -> guest resumes until the bp hit
    (hit: PTY redraw burst + machine stopped)
    for each watch in the plan:
        MEMDUMPBIN <seg>:<off> <len-hex> -> MEMDUMP.BIN (host CWD,
                                             overwritten per call)
        ack "Memory dump binary success." in the logfile
        rename to dumps/<frame>.<n>.bin, validate length
    -> next frame

The watch plan is a resolved JSON list ({watches:[{id,addr,len}]}, plus
optional pre_commands [{cmd,expect}]); extent EXPRESSIONS in watches.toml
(count*0xA8, w*h*2, ...) need runtime counts and are compiled by the
caller — capgen only moves bytes. `--probe` mode pins the plumbing
headless WITHOUT launching the game: empty-autoexec conf, BPINT 8 (the
18.2 Hz timer) as the hit surrogate, real-mode addresses. The live game
capture is interactive-gated in dosbox-harness.sh (`diff capture`,
FORCE_DIFF_RUN=1).

Stdlib only (pty, select, json) — matches the tools/ charter.
"""

import argparse
import json
import os
import pty
import re
import select
import shutil
import signal
import subprocess
import sys
import threading
import time

# ---------------------------------------------------------------- PTY session


class PtySession:
    """A child process on a PTY (the debugger's isatty gate) whose
    command acks are tailed from the [log] logfile.

    A daemon thread drains the master fd continuously: ncurses redraws
    several KB per screen update and the pty kernel buffer is ~64KB —
    once full, the child's wrefresh() blocks and the debugger loop
    stalls forever (empirically pinned: first command acks in ~40ms,
    everything after deadlocks without a reader).
    """

    def __init__(self, argv, cwd, env, log_path, cols=100, rows=50):
        self.master, slave = pty.openpty()
        # ncurses needs a sane window; 100x50 keeps the message pane roomy.
        import fcntl
        import struct
        import termios

        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        self.pty_log = open(log_path, "wb")
        self.proc = subprocess.Popen(
            argv,
            stdin=slave,
            stdout=slave,
            stderr=slave,
            cwd=cwd,
            env=env,
            close_fds=True,
            preexec_fn=os.setsid,
        )
        os.close(slave)
        self.quiet_since = time.monotonic()
        self.total_bytes = 0
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._drain_forever, daemon=True)
        self._thread.start()

    def _drain_forever(self):
        while not self._stop.is_set():
            try:
                ready, _, _ = select.select([self.master], [], [], 0.05)
                if not ready:
                    continue
                chunk = os.read(self.master, 65536)
            except OSError:
                return
            if not chunk:
                return
            self.total_bytes += len(chunk)
            self.quiet_since = time.monotonic()
            self.pty_log.write(chunk)
            self.pty_log.flush()

    def quiesce(self, quiet=1.2, timeout=30):
        """Wait until `quiet` seconds pass with no new PTY bytes."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if time.monotonic() - self.quiet_since >= quiet:
                return True
            time.sleep(0.02)
        return False

    def wait_hit(self, timeout=120):
        """Wait for the breakpoint-hit redraw burst: absorb the RUNWATCH
        redraw first, then any NEW bytes after a quiet gap = the stop.
        Returns False on timeout (caller proceeds anyway: the logfile ack
        validates whether the machine actually stopped)."""
        self.quiesce(quiet=1.0, timeout=20)  # absorb the RUNWATCH redraw
        deadline = time.monotonic() + timeout
        base = self.total_bytes
        while time.monotonic() < deadline:
            if self.total_bytes > base:
                # burst started: drain it out, then return
                self.quiesce(quiet=1.2, timeout=20)
                return True
            time.sleep(0.02)
        return False

    def send(self, line):
        os.write(self.master, line.encode() + b"\r")

    def alive(self):
        return self.proc.poll() is None

    def close(self, timeout=10):
        if self.alive():
            try:
                self.proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                os.killpg(self.proc.pid, signal.SIGTERM)
                try:
                    self.proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(self.proc.pid, signal.SIGKILL)
                    self.proc.wait()
        self._stop.set()
        self._thread.join(timeout=2)
        os.close(self.master)
        self.pty_log.close()


# ---------------------------------------------------------------- logfile tail


class LogTail:
    """Ack reader for the [log] logfile.

    CRITICAL FACT (pinned empirically 2026-08-22, dbgprobe2 matrix runs):
    the child REWRITES (truncates + re-emits) the logfile when the
    debugger initializes — byte offsets observed across polls are NOT
    stable, and a seek-based tail anchors into a stale pre-rewrite copy
    and never sees new acks (cur=2700 > filesize=2212 measured). Acks are
    therefore COUNT-matched over full reads: expect() records the
    occurrence count of the pattern at entry and waits for it to grow.
    Identical ack texts (N MEMDUMPBINs) stay distinguishable because the
    caller waits between sends; the rewrite preserves prior lines, so
    counts stay monotonic. DEBUG_ShowMsg writes one line + \\n + fflush
    per ack (source: debug_gui.cpp:744).
    """

    def __init__(self, path):
        self.path = path

    def count(self, needle):
        try:
            with open(self.path, "rb") as f:
                return f.read().count(needle)
        except FileNotFoundError:
            return 0

    def wait_present(self, pattern, timeout):
        """Wait for the pattern to appear at all (count >= 1). Use for
        one-shot lines (the help banner); `expect` (count growth) is for
        command acks issued after entry."""
        rx = re.compile(pattern, re.DOTALL)
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self._count_rx(rx) >= 1:
                return True
            time.sleep(0.05)
        return False

    def expect(self, pattern, timeout):
        """Wait until the pattern's occurrence count exceeds the count at
        entry; returns the new count."""
        rx = re.compile(pattern, re.DOTALL)
        base = self._count_rx(rx)
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            n = self._count_rx(rx)
            if n > base:
                return n
            time.sleep(0.02)
        data = b""
        try:
            with open(self.path, "rb") as f:
                data = f.read()[-1500:]
        except FileNotFoundError:
            pass
        raise TimeoutError(
            f"logfile expect {pattern!r} timed out (base={base}); tail:\n{data!r}"
        )

    def _count_rx(self, rx):
        try:
            with open(self.path, "rb") as f:
                return len(rx.findall(f.read()))
        except FileNotFoundError:
            return 0


# ---------------------------------------------------------------- capture


def send_cmd(sess, line, settle=1.0):
    """Send one debugger command line after a fixed post-ack settle.

    Empirically pinned (dbgprobe2 rate test, 2026-08-22): a command sent
    ~0.01s after the previous ack stalls for tens of seconds (input sits
    in the tty queue; the ack eventually lands but far too late), while
    the same command sent 1.0s after the last activity acks in ~40ms.
    The 1.0s fixed settle is the proven-safe value; quiesce alone at
    0.35s proved too tight."""
    time.sleep(settle)
    sess.send(line)


def dump_watch(sess, dblog, workdir, dest, addr, length):
    """One MEMDUMPBIN round-trip; returns the dumped bytes."""
    seg, off = addr.split(":", 1)
    cmd = f"MEMDUMPBIN {seg}:{off} {length:X}"
    send_cmd(sess, cmd)
    dblog.expect(rb"Memory dump binary success", timeout=20)
    src = os.path.join(workdir, "MEMDUMP.BIN")
    # the file is complete when the ack printed; move it out of the way
    # before the next overwriting call.
    if not os.path.exists(src):
        raise RuntimeError(f"MEMDUMP.BIN never appeared after {cmd!r}")
    shutil.move(src, dest)
    with open(dest, "rb") as f:
        data = f.read()
    if len(data) != length:
        raise RuntimeError(
            f"MEMDUMP.BIN short read: {len(data)} bytes, wanted {length} ({addr})"
        )
    return data


def run_capture(args):
    with open(args.plan) as f:
        plan = json.load(f)
    watches = plan["watches"]
    if not watches:
        sys.exit("capgen: empty watch plan")
    os.makedirs(args.workdir, exist_ok=True)
    dumps = os.path.join(args.workdir, "dumps")
    shutil.rmtree(dumps, ignore_errors=True)
    os.makedirs(dumps)

    # the [log] logfile is process-CWD-relative: it lands in workdir.
    # (probes use dbgprobe.log; game confs use dosbox-harness.log — take
    # the plan's word if it names one, else auto-detect after start)
    logfile = plan.get("logfile")

    # The [log] logfile APPENDS across runs (pinned empirically: stale
    # acks from a previous run in the same workdir cause instant-false
    # matches). Purge stale logs so every logfile byte is from this run.
    for name in os.listdir(args.workdir):
        if name.endswith(".log") and name != "pty.log":
            os.unlink(os.path.join(args.workdir, name))

    env = dict(os.environ)
    env.update(
        {
            "TERM": "xterm",
            "SDL_VIDEODRIVER": "dummy",
            "SDL_AUDIODRIVER": "dummy",
        }
    )
    argv = [args.dbx, "-conf", os.path.abspath(args.conf), "-break-start"]
    if args.time_limit:
        argv += ["-time-limit", str(args.time_limit)]
    sess = PtySession(argv, args.workdir, env, os.path.join(args.workdir, "pty.log"))

    # logfile: create the tail before launch output lands; detect the name
    if not logfile:
        time.sleep(1.5)  # let DOSBox-X open it at startup
        cands = [
            n for n in os.listdir(args.workdir)
            if n.endswith(".log") and n != "pty.log"
        ]
        if len(cands) != 1:
            sess.close()
            sys.exit(
                f"capgen: cannot auto-detect the [log] logfile in {args.workdir} "
                f"(found {cands}); set 'logfile' in the plan JSON"
            )
        logfile = os.path.join(args.workdir, cands[0])
    dblog = LogTail(logfile)

    frames = []
    try:
        # Liveness: the one-time help banner goes through DEBUG_ShowMsg.
        dblog.wait_present(rb"TYPE HELP", timeout=90)

        if args.probe:
            # Plumbing probe (NO game): the 18.2 Hz timer interrupt is the
            # breakpoint-hit surrogate for the frame-tail bp. Ack: "Set
            # interrupt breakpoint at INT 08" (source: debug.cpp:2590).
            send_cmd(sess, "BPINT 8")
            dblog.expect(rb"Set interrupt breakpoint at INT 08", timeout=15)
        for pre in plan.get("pre_commands", []):
            send_cmd(sess, pre["cmd"])
            dblog.expect(pre["expect"].encode(), timeout=pre.get("timeout", 30))

        for frame in range(1, args.frames + 1):
            if frame > 1:
                send_cmd(sess, "RUNWATCH")
                if not sess.wait_hit(timeout=args.hit_timeout):
                    # No burst seen; proceed anyway — the next ack check
                    # fails loudly if the machine never stopped.
                    print(f"capgen: frame {frame}: no hit burst seen; relying on ack", file=sys.stderr)
            rows = []
            for n, w in enumerate(watches):
                dest = os.path.join(dumps, f"f{frame:06d}.w{n:03d}.bin")
                data = dump_watch(sess, dblog, args.workdir, dest, w["addr"], int(w["len"]))
                rows.append((w["id"], data))
            frames.append((frame, rows))

        if args.probe:
            send_cmd(sess, "BPDEL *")
            dblog.expect(rb"Breakpoints deleted", timeout=10)
    finally:
        try:
            if sess.alive():
                sess.send("QUIT")
        except OSError:
            pass
        sess.close()

    with open(args.out, "w") as f:
        f.write("DBXCAP v1\n")
        f.write(f"# capgen {'probe' if args.probe else 'capture'} ")
        f.write(f"frames={args.frames} dbx={os.path.basename(args.dbx)}\n")
        for frame, rows in frames:
            f.write(f"frame {frame}\n")
            for wid, data in rows:
                f.write(f"watch {wid} {data.hex()}\n")
    print(f"capgen: wrote {args.out} ({args.frames} frames, {len(watches)} watches/frame)")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--dbx", required=True, help="self-built dosbox-x binary")
    ap.add_argument("--conf", required=True, help="run conf (empty autoexec + debuggerrun=debugger for --probe)")
    ap.add_argument("--plan", required=True, help="watch plan JSON {watches:[{id,addr,len}]}")
    ap.add_argument("--workdir", required=True, help="CWD for the binary (MEMDUMP.BIN + logfile land here)")
    ap.add_argument("--out", required=True, help="DBXCAP transcript path")
    ap.add_argument("--frames", type=int, default=3)
    ap.add_argument("--time-limit", type=int, default=180, help="-time-limit safety net")
    ap.add_argument("--hit-timeout", type=int, default=120, help="per-frame breakpoint-hit wait")
    ap.add_argument(
        "--probe",
        action="store_true",
        help="headless plumbing probe: BPINT 8 hit surrogate, NO game launch",
    )
    args = ap.parse_args()
    run_capture(args)


if __name__ == "__main__":
    main()
