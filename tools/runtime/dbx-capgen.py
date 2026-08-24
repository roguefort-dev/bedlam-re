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

The watch plan is a resolved JSON list; two forms:

LEGACY v1: {watches:[{id,addr,len}]} (+ optional pre_commands
[{cmd,expect}] run at the parked -break-start halt). Frame 1 dumps at
that halt; frames 2+ RUNWATCH. `--probe` mode pins the plumbing
headless WITHOUT launching the game (BPINT 8 surrogate) — the dbgprobe
gate uses this.

PLAN v2 (live, D81 — RUNTIME.md "S0 live channel mechanics"): the game
runs from the staged conf's autoexec. Keys:
  boot_commands [{cmd,expect}]  armed at the parked pre-boot halt (the
      BPLM boot trap; BP locations resolve EAGERLY at arm time so a
      game BP armed here would mis-resolve — BPLM is lazy/linear,
      hence the trap).
  flat_guard (default true)     after each boot-trap stop, SELINFO CS
      is parsed from the logfile; the stop is armable iff base==0 and
      limit>=0x12583e (the game flat CS). Non-flat stops (LeLoader
      stub, real mode) retry: BPDEL * + re-arm boot_commands + RUNWATCH.
  boot_retries (default 24), boot_timeout (default hit_timeout)
  arm_commands [{cmd,expect}]   run at the accepted stop (BPDEL * +
      BP CS:<tail> — GetHexValue resolves the CS register name in the
      default MEMDUMPBIN/BP parse path, so no numeric selector is ever
      needed; the BP ack echoes it, captured into the transcript
      header as the selector pin).
  resolve [{name,addr,len}]     little-endian cell reads; values
      become $name symbols. Position is governed by resolve_at:
      "arm" (default, legacy) = read at the arm stop; "anchor" (D84,
      what dbx-plan emits) = read at the ANCHOR stop (mission start)
      — the loader statics (map w/h, TOT/DAT/claim pointers) are
      mission-load values and read garbage pre-mission.
  boot_writes [{addr,bytes}]    W5 BOOT setup (DESIGN §5.5): SMV writes
      applied at the accepted boot stop, before any walk stop (frame
      0; identical stop to the legacy arm position when no walk).
  walk [{stop,addr,bytes}]      W5-walk (D84): the scripted menu walk.
      The BPLM boot trap STAYS ARMED; one stop per counter-writing
      screen frame; stop i's rows apply via SMV at that stop (they
      become screen frame i+1's input — keystore writes re-arm per
      input because the AnyKeyWait twin consumes bytes on read).
      Literal addresses only ($symbols do not exist yet — resolve
      runs at the anchor). arm_commands run at the LAST walk stop
      (BPDEL * drops the BPLM; BP arms the anchor); the machine then
      free-runs through the mission load to the anchor hit.
  walk_watches [{id,addr,len}]  optional calibration rows dumped at
      EVERY walk stop; values ride the transcript as
      "# walk stop N <id> <hex>" comments (the parser skips them) so
      a calibration run maps menu transitions to stop indices.
  inject [{frame,addr,bytes} or {frame,op:"command",base,stride,
      count_cell,bytes} or {frame,op:"pad",bank,slot,target}]
      W5 frame-boundary writes (DESIGN §5): applied at that capture
      frame's stop BEFORE the watch dumps; the transcript record gets
      the injected flag (frame N 1). addr/base/count_cell use the
      SEG:EXPR grammar (CS: = the flat linear identity; numeric segs
      are real-mode seg<<4 for probes). The command op appends one
      record to a count-cell ring: reads count u32, writes the payload
      zero-extended to the stride at base+count*stride, bumps count.
      The pad op (DESIGN §5.4, D86) writes an ORDER to a .PAD slot's
      tile: reads the 8-B slot record {u16 active@+0, x@+2, y@+4,
      z@+6} from bank+slot*8 (999 slots, loader marks active=1,
      x==0xFFFF terminates), fails loud unless active==1 and
      x!=0xFFFF, then writes {x,y,z} as three i32-LE words to the
      target triple (the order-target seam).
  anchor_watches / watches      frame 1 dumps the DEDUPED union
      anchor_watches+watches keep-first by id (the anchor list IS the
      frame-1 row set — the per-frame rows are a subset of
      anchor_watches; a literal concatenation would duplicate ids and
      the stitcher rejects DuplicateWatchId, D140), frames 2+ dump
      watches.
      addr "SEG:<expr>" offsets and len may be arithmetic over $names
      (e.g. "CS:$tot_ptr", "4+16*$map_w*$map_h"). A watch may carry a
      "prefix" {addr, len} sub-row (D109): the prefix cell is dumped
      FIRST and concatenated onto the span — the O1 bank-row grammar
      (u32 count cell + records; trt-array/object-instances).
  frames / time_limit           plan-level defaults (CLI overrides).
  env {KEY: val}                "" removes capgen's default override —
      live plans unset SDL_VIDEODRIVER so the desktop session provides
      the window+keyboard the operator needs to walk the title menu.
Extent EXPRESSIONS in watches.toml (count*0xA8, w*h*2, ...) are compiled
by the caller into these resolve+expr forms — capgen only moves bytes.
The live game capture is interactive-gated in dosbox-harness.sh
(`diff capture`, FORCE_DIFF_RUN=1).

Stdlib only (pty, select, json, ast) — matches the tools/ charter.
"""

import argparse
import ast
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

    def last_match(self, pattern):
        """Last regex match in the whole logfile (SELINFO parse path) or
        None. Full re-read each call: the log is small and this is used
        once per boot-trap stop, not per frame."""
        rx = re.compile(pattern)
        try:
            with open(self.path, "rb") as f:
                matches = rx.findall(f.read())
        except FileNotFoundError:
            return None
        return matches[-1] if matches else None


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


# ---------------------------------------------------------------- W5 inject


def addr_to_linear(addr, symbols):
    """Plan addr form (SEG:EXPR, same grammar as watches) -> the LINEAR
    address SMV takes. CS: uses the flat-selector identity (base 0,
    asserted by the boot guard: linear == the EXD object offset); a
    numeric segment is real-mode seg<<4 (the probe's BDA form)."""
    seg, off = addr.split(":", 1)
    if not _HEX_LIT.match(off):
        off_val = resolve_expr(off, symbols)
    else:
        off_val = int(off, 16)
    seg = seg.strip()
    if seg.upper() == "CS":
        if off_val > 0x12583E:
            raise ValueError(
                f"CS-linear {off_val:#x} exceeds the EXD image top 0x12583e"
            )
        return off_val
    seg_val = resolve_expr(seg, symbols) if not _HEX_LIT.match(seg) else int(seg, 16)
    return (seg_val << 4) + off_val


def smv_bytes(sess, dblog, linear, data):
    """SMV <linear> <byte tokens> — the D80-verified write primitive.
    One round-trip; every token is a 2-hex-digit BYTE (never a register
    name), ack 'DEBUG: Memory changed (N bytes)'."""
    if not data:
        raise ValueError("smv_bytes: empty payload")
    toks = " ".join(f"{b:02X}" for b in data)
    send_cmd(sess, f"SMV {linear:X} {toks}")
    dblog.expect(rb"DEBUG: Memory changed", timeout=20)


def apply_inject(sess, dblog, args, dumps, row, symbols):
    """One plan inject row, applied at a frame-boundary stop BEFORE the
    watch dumps. Forms:
      {frame, addr, bytes}                      plain seam write
      {frame, op:command, base, stride,
       count_cell, bytes}                       command-ring append:
                                                read count u32 (LE),
                                                write the record at
                                                base+count*stride
                                                (zero-extended), bump
                                                the count cell.
      {frame, op:pad, bank, slot, target}      .PAD step-on order
                                                (DESIGN §5.4): read the
                                                8-B slot record at
                                                bank+slot*8, validate
                                                the loader marks, write
                                                {x,y,z} i32-LE x3 to
                                                the target triple."""
    data = bytes.fromhex(row.get("bytes", ""))
    if row.get("op") == "command":
        stride = int(row["stride"], 0) if isinstance(row["stride"], str) else int(row["stride"])
        if stride <= 0 or len(data) > stride:
            raise ValueError(
                f"command inject payload {len(data)} does not fit stride {stride}"
            )
        base_l = addr_to_linear(row["base"], symbols)
        cell_l = addr_to_linear(row["count_cell"], symbols)
        dest = os.path.join(dumps, "inject.count.bin")
        # the read goes through the plan's own SEG:OFF form verbatim
        # (MEMDUMPBIN resolves register names + pmode selectors itself;
        # the SMV writes use the linear conversion).
        cur = dump_watch(sess, dblog, args.workdir, dest, row["count_cell"], 4)
        count = int.from_bytes(cur, "little")
        rec = data + b"\x00" * (stride - len(data))
        smv_bytes(sess, dblog, base_l + count * stride, rec)
        smv_bytes(sess, dblog, cell_l, (count + 1).to_bytes(4, "little"))
        print(
            f"capgen: inject command #{count} -> {base_l + count * stride:#x} "
            f"({len(data)} B payload), count {count} -> {count + 1}",
            file=sys.stderr,
        )
        return
    if row.get("op") == "pad":
        slot = int(row["slot"], 0) if isinstance(row["slot"], str) else int(row["slot"])
        if not 0 <= slot <= 998:
            raise ValueError(f"pad op slot {slot} out of range 0..998 (999 .PAD slots)")
        target = row.get("target")
        if not isinstance(target, list) or len(target) != 3:
            raise ValueError("pad op needs target = [x, y, z] (3 SEG:EXPR addrs)")
        # The READ goes through the bank's own SEG form with the slot
        # offset pre-evaluated (MEMDUMPBIN off is a hex literal; the
        # plan grammar's expressions are capgen's to resolve).
        seg, off = row["bank"].split(":", 1)
        off_val = resolve_expr(off, symbols) if not _HEX_LIT.match(off) else int(off, 16)
        off_val += slot * 8
        if off_val + 8 > 0x1F38:
            raise ValueError(
                f"pad op record at bank+{off_val:#x} exceeds the 999*8 pad bank"
            )
        dest = os.path.join(dumps, "inject.pad.bin")
        rec = dump_watch(sess, dblog, args.workdir, dest, f"{seg}:{off_val:X}", 8)
        active, px, py, pz = (
            int.from_bytes(rec[i : i + 2], "little") for i in (0, 2, 4, 6)
        )
        # FAIL LOUD (D86): active==1 is the 7j.16 loader's parsed-slot
        # mark; x==0xFFFF is the file terminator. A slot the staged
        # mission never loaded must never emit a garbage order.
        if active != 1 or px == 0xFFFF:
            raise ValueError(
                f"pad op slot {slot}: record not a loaded pad "
                f"(active={active:#x}, x={px:#x}, y={py:#x}, z={pz:#x}) — "
                f"the loader marks parsed slots active=1 and stops at "
                f"x==0xFFFF; pick a slot the staged mission has "
                f"(the extraction-pad census, DESIGN §7)"
            )
        for cell, v in zip(target, (px, py, pz)):
            smv_bytes(
                sess, dblog, addr_to_linear(cell, symbols), int(v).to_bytes(4, "little")
            )
        print(
            f"capgen: inject pad slot {slot} ({px},{py},{pz}) -> order-target "
            f"triple {target}",
            file=sys.stderr,
        )
        return
    if not data:
        raise ValueError(f"inject row has no bytes: {row!r}")
    linear = addr_to_linear(row["addr"], symbols)
    smv_bytes(sess, dblog, linear, data)
    print(f"capgen: inject {linear:#x} <- {data.hex()}", file=sys.stderr)


# ---------------------------------------------------------------- plan v2

# The live plan's address form floor: both LE objects (0x10000-0x72800,
# 0x80000-0x12583e) must be readable through the flat CS (RUNTIME.md
# "S0 live channel mechanics" #1/#3).
MIN_FLAT_LIMIT = 0x12583E

V2_KEYS = ("boot_commands", "arm_commands", "resolve", "anchor_watches", "walk")


def resolve_expr(expr, symbols):
    """Evaluate a plan addr/len expression: int literals (0x.. ok), the
    four integer ops, parens, and $name refs into the resolve table.
    ast-whitelisted (never eval())."""
    if isinstance(expr, int):
        return expr
    s = str(expr)

    def sub(m):
        name = m.group(1)
        if name not in symbols:
            raise KeyError(f"plan expression references unknown $${name}")
        return str(symbols[name])

    s = re.sub(r"\$([A-Za-z_][A-Za-z0-9_]*)", sub, s)
    try:
        tree = ast.parse(s, mode="eval")
    except SyntaxError as e:
        raise ValueError(f"bad plan expression {expr!r}: {e}") from None

    def ev(node):
        if isinstance(node, ast.Expression):
            return ev(node.body)
        if isinstance(node, ast.Constant):
            if isinstance(node.value, bool) or not isinstance(node.value, int):
                raise ValueError(f"non-int constant in {expr!r}")
            return node.value
        if isinstance(node, ast.BinOp):
            a, b = ev(node.left), ev(node.right)
            if isinstance(node.op, ast.Add):
                return a + b
            if isinstance(node.op, ast.Sub):
                return a - b
            if isinstance(node.op, ast.Mult):
                return a * b
            if isinstance(node.op, ast.FloorDiv):
                return a // b
        if isinstance(node, ast.UnaryOp):
            if isinstance(node.op, ast.UAdd):
                return +ev(node.operand)
            if isinstance(node.op, ast.USub):
                return -ev(node.operand)
        raise ValueError(f"unsupported node in plan expression {expr!r}")

    return ev(tree)


_HEX_LIT = re.compile(r"[0-9A-Fa-f]*\Z")


def watch_target(w, symbols):
    """One plan watch row -> (addr "SEG:HHHHHHHH", length int). Literal
    hex offsets pass through untouched (legacy form); anything else is
    an expression over $symbols (v2 form)."""
    seg, off = w["addr"].split(":", 1)
    if not _HEX_LIT.match(off):
        off_val = resolve_expr(off, symbols)
        if not 0 <= off_val <= 0xFFFFFFFF:
            raise ValueError(f"watch offset out of range: {w['addr']!r}")
        off = f"{off_val:08X}"
    length = resolve_expr(w["len"], symbols)
    if not isinstance(length, int) or length <= 0:
        raise ValueError(f"watch length must be positive: {w.get('len')!r}")
    if length > 0x100000:
        raise ValueError(f"watch length too large ({length:#x}): {w['addr']!r}")
    return f"{seg}:{off}", length


def selinfo_cs(sess, dblog):
    """SELINFO CS through the logfile; returns (base, limit) ints-or-None.

    Patient by design: during the operator's menu walk the machine RUNS
    for minutes; a wait_hit false positive (stray redraw bytes) sends
    this early and the ack only lands at the REAL boot-trap stop — soak
    with wait_hit until the queued command executes instead of aborting
    the session."""
    send_cmd(sess, "SELINFO CS")
    got = False
    deadline = time.monotonic() + 600
    while time.monotonic() < deadline:
        try:
            dblog.expect(rb"SelectorInfo CS:", timeout=15)
            got = True
            break
        except TimeoutError:
            sess.wait_hit(timeout=60)  # soak until the real stop; the
            # queued command acks then (count growth). Re-send each
            # round: expect() re-bases its count on entry, so a fresh
            # ack is what the next expect matches.
            send_cmd(sess, "SELINFO CS")
    if not got:
        raise RuntimeError("capgen: SELINFO CS never acked (no stop within 10 min)")
    base = limit = None
    m = dblog.last_match(rb"CS: b:([0-9A-Fa-f]{8})")
    if m:
        base = int(m, 16)
    m = dblog.last_match(rb"l:([0-9A-Fa-f]{8})")
    if m:
        limit = int(m, 16)
    return base, limit


def run_boot_trap(sess, dblog, plan, args):
    """v2: RUNWATCH into the first post-boot stop and keep retrying until
    the stop is in a flat (base-0, full-image) CS context. Returns the
    (base, limit) of the accepted stop."""
    flat_guard = plan.get("flat_guard", True)
    boot_timeout = int(plan.get("boot_timeout", args.hit_timeout))
    boot_retries = int(plan.get("boot_retries", 24))

    def arm_boot():
        for pre in plan.get("boot_commands", []):
            send_cmd(sess, pre["cmd"])
            dblog.expect(pre["expect"].encode(), timeout=pre.get("timeout", 30))

    arm_boot()
    base = limit = None
    for attempt in range(1, boot_retries + 1):
        send_cmd(sess, "RUNWATCH")
        if not sess.wait_hit(timeout=boot_timeout):
            raise RuntimeError(
                f"capgen: boot trap never fired (attempt {attempt}/{boot_retries})"
            )
        if not flat_guard:
            return None, None
        base, limit = selinfo_cs(sess, dblog)
        print(
            f"capgen: boot stop {attempt}/{boot_retries}: CS base="
            f"{base if base is None else hex(base)} limit="
            f"{limit if limit is None else hex(limit)}",
            file=sys.stderr,
        )
        if base == 0 and limit is not None and limit >= MIN_FLAT_LIMIT:
            return base, limit
        # Non-flat stop (LeLoader stub / real mode): drop every trap,
        # re-arm the boot trap, run again.
        send_cmd(sess, "BPDEL *")
        dblog.expect(rb"Breakpoints deleted", timeout=10)
        arm_boot()
    raise RuntimeError(
        f"capgen: no flat-CS stop after {boot_retries} boot traps "
        f"(last base={base} limit={limit})"
    )


def run_arm(sess, dblog, plan):
    """v2: run arm_commands at the accepted stop; return the numeric flat
    selector echoed by the BP ack (the per-run selector pin)."""
    for pre in plan.get("arm_commands", []):
        send_cmd(sess, pre["cmd"])
        dblog.expect(pre["expect"].encode(), timeout=pre.get("timeout", 30))
    m = dblog.last_match(rb"Set breakpoint at ([0-9A-Fa-f]{4}):")
    return m.decode() if m else None


def run_resolve(sess, dblog, plan, args, dumps):
    """v2: read the plan's loader-static cells at the arm stop."""
    symbols = {}
    for r in plan.get("resolve", []):
        dest = os.path.join(dumps, f"resolve.{r['name']}.bin")
        data = dump_watch(
            sess, dblog, args.workdir, dest, r["addr"], int(r.get("len", 4))
        )
        symbols[r["name"]] = int.from_bytes(data, "little")
        print(f"capgen: resolved {r['name']} = {symbols[r['name']]:#x}", file=sys.stderr)
    return symbols


def run_walk(sess, dblog, plan, args, dumps, notes):
    """v3 WALK phase (D84): the BPLM boot trap stays armed — one stop
    per counter-writing screen frame. Stop i applies its `walk` rows
    via SMV (they become screen frame i+1's input); optional
    walk_watches are dumped per stop AFTER the writes (same
    write-then-dump ordering as the frame loop) into transcript
    comments. arm_commands run at the LAST walk stop (BPDEL * drops
    the BPLM; the anchor BP arms). Returns the selector pin.

    Walk rows are LITERAL addresses: $symbols do not exist yet (resolve
    runs at the anchor, after the walk)."""
    rows = plan.get("walk") or []
    if not rows:
        raise ValueError("capgen: walk plan has no rows")
    by_stop = {}
    for row in rows:
        stop = int(row["stop"])
        if stop < 1:
            raise ValueError(f"capgen: walk stop indices are 1-based (got {stop})")
        if row.get("op"):
            raise ValueError(
                "capgen: walk rows are plain writes only (command ops are "
                "mission-phase seams; a menu walk needs no ring appends)"
            )
        by_stop.setdefault(stop, []).append(row)
    last = max(by_stop)
    watch_defs = plan.get("walk_watches") or []
    for stop in range(1, last + 1):
        send_cmd(sess, "RUNWATCH")
        if not sess.wait_hit(timeout=args.hit_timeout):
            print(f"capgen: walk stop {stop}: no hit burst seen; relying on ack", file=sys.stderr)
        wrote = 0
        for row in by_stop.get(stop, []):
            apply_inject(sess, dblog, args, dumps, row, {})
            wrote += 1
        for w in watch_defs:
            addr, length = watch_target(w, {})
            dest = os.path.join(dumps, f"walk{stop:05d}.{w['id']}.bin")
            data = dump_watch(sess, dblog, args.workdir, dest, addr, length)
            notes.append(f"walk stop {stop} {w['id']} {data.hex()}")
        print(
            f"capgen: walk stop {stop}/{last} ({wrote} writes)", file=sys.stderr
        )
    # At the last walk stop: drop the BPLM, arm the anchor. The machine
    # then free-runs through the mission load to the anchor hit.
    return run_arm(sess, dblog, plan)


def dedupe_frame1_rows(anchor_watches, watches):
    """The frame-1 row list: the deduped union keep-first by id.

    D140(2): on every committed plan the per-frame rows are a SUBSET of
    anchor_watches (the anchor list IS the frame-1 row set), so a
    literal anchor_watches+watches concatenation emits DUPLICATE ids and
    `diff stitch` rejects the transcript (canonicalize_frame
    DuplicateWatchId, dump.rs). Mirrors the capgen-o2 semantics exactly.
    """
    rows, seen = [], set()
    for w in anchor_watches + watches:
        if w["id"] in seen:
            continue
        seen.add(w["id"])
        rows.append(w)
    return rows


def run_capture(args):
    with open(args.plan) as f:
        plan = json.load(f)
    v2 = any(k in plan for k in V2_KEYS)
    watches = plan.get("watches", [])
    anchor_watches = plan.get("anchor_watches", [])
    if not v2 and not watches:
        sys.exit("capgen: empty watch plan")
    if v2 and not watches and not anchor_watches:
        sys.exit("capgen: empty v2 watch plan")
    frames_total = args.frames if args.frames is not None else int(plan.get("frames", 3))
    time_limit = args.time_limit if args.time_limit is not None else int(
        plan.get("time_limit", 180)
    )
    os.makedirs(args.workdir, exist_ok=True)
    dumps = os.path.join(args.workdir, "dumps")
    shutil.rmtree(dumps, ignore_errors=True)
    os.makedirs(dumps)

    # the [log] logfile is process-CWD-relative: it lands in workdir.
    # (probes use dbgprobe.log; game confs use dosbox-harness.log — take
    # the plan's word if it names one, else auto-detect after start.
    # A plan-named file is resolved against the WORKDIR — capgen's own
    # CWD is wherever the harness was invoked from, NOT the workdir.)
    logfile = plan.get("logfile")
    if logfile:
        if not os.path.isabs(logfile):
            logfile = os.path.join(args.workdir, logfile)
    logfile_abs = logfile

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
    # plan env: a value REPLACES capgen's default override; "" REMOVES it
    # (live plans need the real display+keyboard for the menu walk).
    for key, val in (plan.get("env") or {}).items():
        if val:
            env[key] = val
        else:
            env.pop(key, None)

    argv = [args.dbx, "-conf", os.path.abspath(args.conf), "-break-start"]
    if time_limit:
        argv += ["-time-limit", str(time_limit)]
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
    assert logfile_abs is None or logfile_abs == logfile
    dblog = LogTail(logfile)

    selector_pin = None
    boot_facts = (None, None)
    symbols = {}
    frames = []
    walk_notes = []
    try:
        # Liveness: the one-time help banner goes through DEBUG_ShowMsg.
        if not dblog.wait_present(rb"TYPE HELP", timeout=90):
            raise RuntimeError(
                f"capgen: no debugger banner in the logfile {logfile} — "
                "is the debugger up (PTY gate) and is [log] logfile= set?"
            )

        if args.probe:
            # Plumbing probe (NO game): the 18.2 Hz timer interrupt is the
            # breakpoint-hit surrogate for the frame-tail bp. Ack: "Set
            # interrupt breakpoint at INT 08" (source: debug.cpp:2590).
            send_cmd(sess, "BPINT 8")
            dblog.expect(rb"Set interrupt breakpoint at INT 08", timeout=15)
        for pre in plan.get("pre_commands", []):
            send_cmd(sess, pre["cmd"])
            dblog.expect(pre["expect"].encode(), timeout=pre.get("timeout", 30))

        if v2:
            # The machine is parked pre-boot at -break-start: run into the
            # boot trap, accept only a flat-CS stop, then (D84 flow) boot
            # writes at that stop, the WALK phase on the still-armed BPLM
            # (arm lands at the LAST walk stop), the anchor stop = frame 1.
            if args.probe:
                sys.exit("capgen: --probe is the legacy path; a v2 plan (boot/arm/resolve/anchor keys) must not combine with it")
            boot_facts = run_boot_trap(sess, dblog, plan, args)
            # W5 BOOT writes (§5.5): applied at the accepted boot stop —
            # frame 0, before any walk stop (the SAME stop as the legacy
            # arm position when no walk phase exists, so behavior is
            # unchanged for walk-less plans). Literal addresses only:
            # $symbols do not exist yet in the resolve_at=anchor flow.
            for row in plan.get("boot_writes", []):
                data = bytes.fromhex(row.get("bytes", ""))
                if not data:
                    raise ValueError(f"boot_writes row has no bytes: {row!r}")
                smv_bytes(sess, dblog, addr_to_linear(row["addr"], symbols), data)
                print(
                    f"capgen: boot write {row['addr']} <- {data.hex()}",
                    file=sys.stderr,
                )
            # D84 walk phase (the scripted menu walk): stop-indexed SMV
            # writes, one stop per counter-writing screen frame; the arm
            # commands run at the LAST walk stop.
            if plan.get("walk"):
                selector_pin = run_walk(sess, dblog, plan, args, dumps, walk_notes)
            else:
                selector_pin = run_arm(sess, dblog, plan)
            print(f"capgen: selector pin CS={selector_pin}", file=sys.stderr)
            # resolve position (D84): "arm" (legacy default) reads at the
            # arm stop; "anchor" (what dbx-plan emits) defers to the
            # frame-1 stop — the loader statics (map w/h, TOT/DAT/claim
            # pointers) are MISSION-load values and read garbage at any
            # pre-mission stop.
            if plan.get("resolve_at", "arm") != "anchor":
                symbols.update(run_resolve(sess, dblog, plan, args, dumps))
                resolve_pending = False
            else:
                resolve_pending = True
        else:
            resolve_pending = False

        inject_by_frame = {}
        for row in plan.get("inject", []):
            inject_by_frame.setdefault(int(row["frame"]), []).append(row)

        def dump_rows(frame, rows_def):
            rows = []
            for n, w in enumerate(rows_def):
                addr, length = watch_target(w, symbols)
                dest = os.path.join(dumps, f"f{frame:06d}.w{n:03d}.bin")
                data = b""
                if "prefix" in w:
                    # D109 count-cell prefix: dump the 4-byte cell
                    # FIRST, then the span — one concatenated blob
                    # (the O1 bank-row grammar the differ pins: u32
                    # count + records; e.g. trt 0x11949c + the
                    # count*0x20 span, object 0x119554 + the full
                    # 2000*0x14 bank).
                    paddr, plen = watch_target(w["prefix"], symbols)
                    pdest = os.path.join(dumps, f"f{frame:06d}.w{n:03d}.pre.bin")
                    data += dump_watch(sess, dblog, args.workdir, pdest, paddr, plen)
                data += dump_watch(sess, dblog, args.workdir, dest, addr, length)
                rows.append((w["id"], data))
            return rows

        # D140(2): frame 1 dumps the deduped anchor union keep-first —
        # see dedupe_frame1_rows (a literal concatenation duplicates
        # ids; `diff stitch` rejects DuplicateWatchId).
        frame1_rows = dedupe_frame1_rows(anchor_watches, watches)

        for frame in range(1, frames_total + 1):
            # v1 keeps frame 1 at the parked pre-boot halt (the probe
            # shape); v2 RUNWATCHes into every frame incl. 1 — the first
            # hit is the anchor (mission start; with a walk phase, the
            # first hit of the BP armed at the last walk stop).
            if v2 or frame > 1:
                send_cmd(sess, "RUNWATCH")
                if not sess.wait_hit(timeout=args.hit_timeout):
                    # No burst seen; proceed anyway — the next ack check
                    # fails loudly if the machine never stopped.
                    print(f"capgen: frame {frame}: no hit burst seen; relying on ack", file=sys.stderr)
                if frame == 1 and resolve_pending:
                    # D84 resolve_at=anchor: the loader statics are read
                    # at the anchor stop, before any dump needs the
                    # $symbols (expr lens evaluate per dump).
                    symbols.update(run_resolve(sess, dblog, plan, args, dumps))
                    resolve_pending = False
            # W5 injection (§5): apply this frame boundary's rows BEFORE
            # the dumps (the record carries injection_applied=1).
            injected = False
            for row in inject_by_frame.get(frame, []):
                apply_inject(sess, dblog, args, dumps, row, symbols)
                injected = True
            rows = dump_rows(frame, frame1_rows if frame == 1 else watches)
            frames.append((frame, rows, injected))

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
        f.write(f"frames={frames_total} dbx={os.path.basename(args.dbx)}\n")
        if v2:
            base, limit = boot_facts
            f.write(
                f"# selector-pin CS={selector_pin} base="
                f"{hex(base) if base is not None else 'n/a'} limit="
                f"{hex(limit) if limit is not None else 'n/a'}\n"
            )
            for name, val in symbols.items():
                f.write(f"# resolved {name}={val:#x}\n")
        for note in walk_notes:
            f.write(f"# {note}\n")
        for frame, rows, injected in frames:
            f.write(f"frame {frame} 1\n" if injected else f"frame {frame}\n")
            for wid, data in rows:
                f.write(f"watch {wid} {data.hex()}\n")
    print(f"capgen: wrote {args.out} ({frames_total} frames, {len(frame1_rows)} watches/frame-1 (deduped anchor union), {len(watches)}/frame-n)")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--dbx", required=True, help="self-built dosbox-x binary")
    ap.add_argument("--conf", required=True, help="run conf (empty autoexec + debuggerrun=debugger for --probe)")
    ap.add_argument("--plan", required=True, help="watch plan JSON {watches:[{id,addr,len}]}")
    ap.add_argument("--workdir", required=True, help="CWD for the binary (MEMDUMP.BIN + logfile land here)")
    ap.add_argument("--out", required=True, help="DBXCAP transcript path")
    ap.add_argument("--frames", type=int, default=None,
                    help="frame records to capture (default: plan 'frames', else 3)")
    ap.add_argument("--time-limit", type=int, default=None,
                    help="-time-limit safety net (default: plan 'time_limit', else 180)")
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
