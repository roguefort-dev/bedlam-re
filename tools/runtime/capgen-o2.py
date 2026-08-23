#!/usr/bin/env python3
"""capgen-o2 — the W11 O2 transcript emitter skeleton (D140, DESIGN §10-W11).

The headless producer of the O2 DBXCAP capture transcript. Contract
split (why this exists): the W11 ptrace DRIVER (operator-gated Wine
work — trigger hits at the plan's trigger.site + process_vm_readv per
watch row) logs a DBXFEED v1 read/write log; THIS tool is the pure
plan interpreter + transcript emitter: it walks the D138 o2 capture
plan (resolve expressions, prefix sub-rows, $sym addrs, len exprs,
the three inject op forms), validates the feed against its own walk
1:1 — every read's addr+len, hit numbering, inject arithmetic
re-derived — and writes the DBXCAP v1 lines the D139 dbx-stitch
--channel o2 validates. Proving plan -> driver-log -> transcript ->
stitch -> differ headless needs NO Wine: --synthesize-feed is a
reference mini-driver whose arithmetic exercises every feed form.

DBXFEED v1 grammar (the driver's observable output):
    DBXFEED v1                  header (first non-comment line)
    kind synthetic|driver       REQUIRED; synthetic feeds mark the
                                emitted transcript SYNTHETIC (anti-
                                ghost: fabricated bytes are never live
                                captures — the s0-replay fixture
                                precedent)
    hit <n>                     starts a hit block. hit 0 = the
                                optional BOOT position (boot_writes
                                only, before any capture frame); hit 1
                                = the ANCHOR — where the feed starts
                                IS the driver's mission-load policy
                                (the plan never guesses it); hits
                                1..frames are the capture frames
                                (frame N == hit N, the same numbering
                                dbx-plan pins for inject rows:
                                "anchor-relative boundary numbering =
                                capture frame numbers").
    read <addr> <len> <hex>     one plan-ordered read at the current
                                hit (addr = resolved flat linear 0x
                                form, len decimal)
    write <addr> <len> <hex>    a driver write at the current hit
                                (inject entries precede the block's
                                watch reads — the O1 write-then-dump
                                ordering)

Block order is EXACTLY: [resolve reads (hit 1 only, plan order)]
[the frame's inject entries, plan order] [the frame's watch reads,
plan order; a prefix row = prefix-cell read then span read]. The
emitter consumes each block as a queue and fails loud on any mismatch
— the future driver's log must match the plan walk exactly.

Frame-counter alignment (trigger.frame_counter): the frame-counter
watch must advance +1 per hit from the anchor (the anchor value
itself is menu-timing dependent). Drift warns to stderr + records
one transcript comment — a missed trigger hit would otherwise
silently misalign every later frame.

Stdlib only — matches the tools/ charter; expression evaluation
reuses dbx-capgen.py's ast-whitelisted resolve_expr (ONE resolver,
never a second arithmetic implementation).
"""

import argparse
import importlib.util
import json
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))


def _load_capgen():
    """dbx-capgen.py (dash name, not importable) as a module for
    resolve_expr — importing it has no side effects (main is guarded)."""
    spec = importlib.util.spec_from_file_location(
        "dbx_capgen", os.path.join(_HERE, "dbx-capgen.py")
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_capgen = _load_capgen()
resolve_expr = _capgen.resolve_expr


# ---------------------------------------------------------------- plan form


class PlanError(Exception):
    pass


def load_plan(path):
    with open(path) as f:
        plan = json.load(f)
    if plan.get("channel") != "o2":
        raise PlanError(
            f"plan {path!r} is not an o2 channel plan (channel="
            f"{plan.get('channel')!r}) — O1/DOSBox plans are dbx-capgen.py's domain"
        )
    if plan.get("resolve_at", "anchor") != "anchor":
        raise PlanError(
            "o2 plans must carry resolve_at=anchor (there is no pre-mission arm "
            "stop under Wine; the D138 plan form pins the anchor read)"
        )
    if plan.get("walk"):
        raise PlanError(
            "o2 plans never carry a walk phase (the BPLM stop-indexed menu walk "
            "is DOSBox/O1 machinery — dbx-plan --channel o2 refuses it; D84/D138)"
        )
    trig = plan.get("trigger") or {}
    if not trig.get("site") or not trig.get("frame_counter"):
        raise PlanError("o2 plan is missing trigger.site / trigger.frame_counter")
    if not plan.get("watches"):
        raise PlanError("plan has no per-frame watches")
    return plan


def o2_addr(field, symbols):
    """O2 plan addr form -> int: a flat 0x literal or a bare $sym."""
    s = str(field).strip()
    if s.startswith("$"):
        return resolve_expr(s, symbols)
    if s.lower().startswith("0x"):
        return int(s, 16)
    raise ValueError(f"bad o2 addr form (want 0x.. or $sym): {field!r}")


def o2_len(field, symbols):
    n = resolve_expr(field, symbols)
    if not isinstance(n, int) or n <= 0:
        raise ValueError(f"watch length must be positive: {field!r}")
    if n > 0x100000:
        raise ValueError(f"watch length too large ({n:#x})")
    return n


def watch_reads(w, symbols):
    """One watch row -> the ordered (addr, len) reads it takes (a
    prefix row = prefix cell first, then the span — the O1 bank-row
    grammar; the transcript record is the CONCATENATION under one id)."""
    out = []
    if "prefix" in w:
        p = w["prefix"]
        out.append((o2_addr(p["addr"], symbols), o2_len(p["len"], symbols)))
    out.append((o2_addr(w["addr"], symbols), o2_len(w["len"], symbols)))
    return out


# ---------------------------------------------------------------- the walk


def plan_walk(plan, frames_total, symbols):
    """THE single ordered operation sequence both consumers drive (the
    validator and the synthesizer can never diverge — they share this).

    `symbols` is the RUNNING resolve table (mutated by the consumer as
    resolve ops are processed): the generator is lazy, so hit-1 watch
    reads see the resolve values that precede them.

    Yields (hit_no, op) where op is one of:
      ("resolve", row)          hit 1 only, plan order, before anything
      ("inject", row)           the frame's plan inject rows, plan order
      ("watch", w, reads)       one watch row -> its ordered
                                [(addr, len), ...] reads (a prefix row
                                = prefix cell first, then the span; the
                                transcript record is the CONCATENATION
                                of the read payloads under one id)
    """
    for r in plan.get("resolve", []):
        yield 1, ("resolve", r)
    inject_by_frame = {}
    for row in plan.get("inject", []):
        inject_by_frame.setdefault(int(row["frame"]), []).append(row)
    for frame in range(1, frames_total + 1):
        for row in inject_by_frame.get(frame, []):
            yield frame, ("inject", row)
        if frame == 1:
            # The anchor list IS the frame-1 row set: on every committed
            # plan the per-frame rows are a SUBSET of anchor_watches (the
            # TS statics ride the anchor frame; every T0 row also rides
            # it). Emit the deduped union keep-first — a literal
            # concatenation would duplicate ids and the stitcher's
            # canonicalize_frame rejects DuplicateWatchId (dump.rs; the
            # same landmine exists in the O1 dbx-capgen frame-1 path —
            # D140 finding, queued for the O1 tool).
            rows, seen = [], set()
            for w in plan.get("anchor_watches", []) + plan["watches"]:
                if w["id"] in seen:
                    continue
                seen.add(w["id"])
                rows.append(w)
        else:
            rows = plan["watches"]
        for w in rows:
            yield frame, ("watch", w, watch_reads(w, symbols))


# ---------------------------------------------------------------- feed parse


class FeedError(Exception):
    pass


def parse_feed(path):
    """DBXFEED v1 -> (kind, {hit_no: [entries]}); entries are
    (op, addr, len, bytes, line_no) in arrival order."""
    kind = None
    saw_header = False
    blocks = {}
    cur = None
    with open(path) as f:
        for line_no, raw in enumerate(f, 1):
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            d = parts[0]
            if d == "DBXFEED":
                if saw_header:
                    raise FeedError(f"{path}:{line_no}: duplicate DBXFEED header")
                if len(parts) < 2 or parts[1] != "v1":
                    raise FeedError(f"{path}:{line_no}: expected `DBXFEED v1`")
                saw_header = True
            elif not saw_header:
                raise FeedError(
                    f"{path}:{line_no}: feed must start with `DBXFEED v1` (line: {line!r})"
                )
            elif d == "kind":
                if kind is not None:
                    raise FeedError(f"{path}:{line_no}: duplicate kind")
                if len(parts) != 2 or parts[1] not in ("synthetic", "driver"):
                    raise FeedError(f"{path}:{line_no}: kind must be synthetic|driver")
                kind = parts[1]
            elif d == "hit":
                if len(parts) != 2:
                    raise FeedError(f"{path}:{line_no}: hit needs a number")
                try:
                    n = int(parts[1], 0)
                except ValueError:
                    raise FeedError(f"{path}:{line_no}: bad hit number") from None
                if n < 0:
                    raise FeedError(f"{path}:{line_no}: hit numbers are >= 0")
                if cur is not None and n <= cur:
                    raise FeedError(
                        f"{path}:{line_no}: hit numbers must strictly increase "
                        f"({n} after {cur})"
                    )
                cur = n
                blocks[cur] = []
            elif d in ("read", "write"):
                if cur is None:
                    raise FeedError(f"{path}:{line_no}: {d} before any hit block")
                try:
                    addr = int(parts[1], 16)
                    ln = int(parts[2], 0)
                    data = bytes.fromhex(parts[3]) if len(parts) > 3 else b""
                except (IndexError, ValueError) as e:
                    raise FeedError(f"{path}:{line_no}: bad {d} line: {e}") from None
                if len(data) != ln:
                    raise FeedError(
                        f"{path}:{line_no}: {d} payload is {len(data)} B, header says {ln}"
                    )
                blocks[cur].append((d, addr, ln, data, line_no))
            else:
                raise FeedError(f"{path}:{line_no}: unknown directive {d!r}")
    if not saw_header:
        raise FeedError(f"{path}: empty feed: missing `DBXFEED v1` header")
    if kind is None:
        raise FeedError(f"{path}: feed has no `kind synthetic|driver` line")
    return kind, blocks


# ---------------------------------------------------------------- validation


class ValidationError(Exception):
    pass


def _expect(queue, op_want, addr, ln, what):
    """Consume the next feed entry; it must be this exact read/write."""
    if not queue:
        raise ValidationError(
            f"feed ended while expecting a {op_want} for {what} @{addr:#x} len {ln}"
        )
    op, a, l, data, line_no = queue.pop(0)
    if op != op_want:
        raise ValidationError(
            f"feed line {line_no}: expected a {op_want.upper()} for {what} "
            f"(@{addr:#x} len {ln}), got a {op} (@{a:#x} len {l})"
        )
    if a != addr or l != ln:
        raise ValidationError(
            f"feed line {line_no}: {what} {op_want} mismatch — plan derives "
            f"@{addr:#x} len {ln}, feed logged @{a:#x} len {l}"
        )
    return data


def validate_inject(queue, row, symbols, frame):
    """Consume + check one inject row's entries from the block queue.

    The addr/len of DERIVED entries (a command ring's dest
    base+count*stride, a pad record at bank+slot*8) are re-derived
    from what the feed itself logged — the emitter re-derives the
    driver's arithmetic instead of trusting it."""
    tag = f"frame {frame} inject {row.get('op') or 'plain'}"
    if row.get("op") == "command":
        stride = (
            int(row["stride"], 0) if isinstance(row["stride"], str) else int(row["stride"])
        )
        data = bytes.fromhex(row.get("bytes", ""))
        if stride <= 0 or len(data) > stride:
            raise ValidationError(
                f"{tag}: payload {len(data)} does not fit stride {stride}"
            )
        cell = o2_addr(row["count_cell"], symbols)
        seen = _expect(queue, "read", cell, 4, f"{tag} count cell")
        count = int.from_bytes(seen, "little")
        dest = o2_addr(row["base"], symbols) + count * stride
        exp = data + b"\x00" * (stride - len(data))
        got = _expect(queue, "write", dest, stride, f"{tag} record #{count}")
        if got != exp:
            raise ValidationError(
                f"{tag}: record @ {dest:#x} is {got.hex()}, plan derives {exp.hex()}"
            )
        bump = (count + 1).to_bytes(4, "little")
        got = _expect(queue, "write", cell, 4, f"{tag} count bump")
        if got != bump:
            raise ValidationError(
                f"{tag}: count bump is {got.hex()}, plan derives {bump.hex()}"
            )
        return
    if row.get("op") == "pad":
        slot = int(row["slot"], 0) if isinstance(row["slot"], str) else int(row["slot"])
        if not 0 <= slot <= 998:
            raise ValidationError(f"{tag}: slot {slot} out of range 0..998")
        target = row.get("target")
        if not isinstance(target, list) or len(target) != 3:
            raise ValidationError(f"{tag}: needs target = [x, y, z] (3 addrs)")
        rec = _expect(
            queue, "read", o2_addr(row["bank"], symbols) + slot * 8, 8, f"{tag} slot record"
        )
        active = int.from_bytes(rec[0:2], "little")
        xyz = tuple(int.from_bytes(rec[i : i + 2], "little") for i in (2, 4, 6))
        if active != 1 or xyz[0] == 0xFFFF:
            raise ValidationError(
                f"{tag}: slot {slot} record is not a loaded pad (active="
                f"{active:#x}, x={xyz[0]:#x}, y={xyz[1]:#x}, z={xyz[2]:#x}) — "
                f"the loader marks parsed slots active=1 and stops at "
                f"x==0xFFFF (the D86 rule)"
            )
        for i, cell in enumerate(target):
            exp = int(xyz[i]).to_bytes(4, "little")
            got = _expect(queue, "write", o2_addr(cell, symbols), 4, f"{tag} target[{i}]")
            if got != exp:
                raise ValidationError(
                    f"{tag}: target[{i}] write is {got.hex()}, slot says {exp.hex()}"
                )
        return
    data = bytes.fromhex(row.get("bytes", ""))
    if not data:
        raise ValidationError(f"{tag}: inject row has no bytes: {row!r}")
    got = _expect(queue, "write", o2_addr(row["addr"], symbols), len(data), tag)
    if got != data:
        raise ValidationError(f"{tag}: write is {got.hex()}, plan wants {data.hex()}")


def emit(plan, kind, blocks, frames_total, out_path):
    """Validate the feed against the plan walk + write the DBXCAP."""
    symbols = {}
    # hit 0: the optional BOOT position (boot_writes only).
    hit0 = blocks.pop(0, None)
    boot_writes = plan.get("boot_writes", [])
    if boot_writes:
        if hit0 is None:
            raise ValidationError("plan has boot_writes but the feed has no hit 0 block")
        q = list(hit0)
        for row in boot_writes:
            data = bytes.fromhex(row.get("bytes", ""))
            if not data:
                raise ValidationError(f"boot_writes row has no bytes: {row!r}")
            got = _expect(q, "write", o2_addr(row["addr"], symbols), len(data), "boot write")
            if got != data:
                raise ValidationError(
                    f"boot write @ {row['addr']}: feed says {got.hex()}, "
                    f"plan wants {data.hex()}"
                )
        if q:
            op, a, l, _d, line_no = q[0]
            raise ValidationError(
                f"feed line {line_no}: hit 0 has an unexpected trailing {op} "
                f"@{a:#x} len {l} (boot blocks carry boot_writes only)"
            )
    elif hit0:
        raise ValidationError("feed has a hit 0 block but the plan has no boot_writes")

    # Frames: consume each hit block as a queue against the walk.
    frames_out = []  # (frame_no, [(id, bytes)], injected)
    cur = None
    queue = None
    rows = []
    injected = False

    def close():
        if queue:
            op, a, l, _d, line_no = queue[0]
            raise ValidationError(
                f"feed line {line_no}: hit {cur} block has an unexpected {op} "
                f"@{a:#x} len {l} after the plan walk ended"
            )
        frames_out.append((cur, rows, injected))

    for hit_no, op in plan_walk(plan, frames_total, symbols):
        if hit_no != cur:
            if cur is not None:
                close()
            cur = hit_no
            block = blocks.pop(hit_no, None)
            if block is None:
                raise ValidationError(
                    f"feed has no hit {hit_no} block "
                    f"(frames 1..{frames_total} are all required)"
                )
            queue = list(block)
            rows = []
            injected = False
        if op[0] == "resolve":
            r = op[1]
            data = _expect(
                queue,
                "read",
                o2_addr(r["addr"], symbols),
                o2_len(r.get("len", 4), symbols),
                f"resolve {r['name']}",
            )
            symbols[r["name"]] = int.from_bytes(data, "little")
        elif op[0] == "inject":
            validate_inject(queue, op[1], symbols, hit_no)
            injected = True
        else:
            _, w, reads = op
            data = b""
            for addr, ln in reads:
                data += _expect(queue, "read", addr, ln, f"watch {w['id']}")
            rows.append((w["id"], data))
    if cur is not None:
        close()
    if blocks:
        raise ValidationError(
            f"feed has extra hit blocks past frame {frames_total}: {sorted(blocks)}"
        )

    # Frame-counter alignment (trigger.frame_counter): +1 per hit from
    # the anchor; drift warns + records one comment (never silently).
    drift_comments = []
    anchor_fc = None
    for i, (no, frows, _inj) in enumerate(frames_out):
        fc = next((d for wid, d in frows if wid == "frame-counter"), None)
        if fc is None or len(fc) < 4:
            continue
        v = int.from_bytes(fc[:4], "little")
        if i == 0:
            anchor_fc = v
        elif anchor_fc is not None and v != anchor_fc + i:
            msg = f"frame-counter drift at frame {no}: {v} (anchor {anchor_fc} + {i})"
            print(f"capgen-o2: WARNING {msg}", file=sys.stderr)
            drift_comments.append(msg)

    with open(out_path, "w") as f:
        f.write("DBXCAP v1\n")
        trig = plan["trigger"]
        f.write(
            f"# capgen-o2 transcript channel=o2 frames={frames_total} "
            f"trigger.site={trig['site']} trigger.frame_counter={trig['frame_counter']} "
            f"feed-kind={kind}\n"
        )
        if kind == "synthetic":
            f.write(
                "# SYNTHETIC feed (capgen-o2 --synthesize-feed) — NOT game data.\n"
                "# Byte values are fabricated determinism vectors; they carry NO\n"
                "# claim about the original game's memory (anti-ghost: live O2\n"
                "# captures come only from the W11 ptrace driver under Wine).\n"
            )
        for name, val in symbols.items():
            f.write(f"# resolved {name}={val:#x}\n")
        for c in drift_comments:
            f.write(f"# WARNING {c}\n")
        for no, frows, inj in frames_out:
            f.write(f"frame {no} 1\n" if inj else f"frame {no}\n")
            for wid, data in frows:
                f.write(f"watch {wid} {data.hex()}\n")
    print(
        f"capgen-o2: wrote {out_path} ({len(frames_out)} frames, "
        f"{len(plan['watches'])} watches/frame-n, "
        f"{sum(1 for _f, r, _i in frames_out[:1] for _ in r)} frame-1 rows "
        f"(the deduped anchor union), kind={kind})",
        file=sys.stderr,
    )


# ---------------------------------------------------------------- synth feed


_LCG = 0x5851F42D4C957F2D
_MASK = (1 << 64) - 1


def synth_bytes(addr, ln, hit):
    """Deterministic per-(addr, hit) bytes for unpinned regions — the
    SYNTHETIC determinism vector (never a claim about game memory)."""
    out = bytearray(ln)
    st = (
        addr * 0x9E3779B97F4A7C15
        ^ ln * 0xBF58476D1CE4E5B9
        ^ hit * 0x94D049BB133111EB
    ) & _MASK
    for i in range(ln):
        st = (st * _LCG + 0x14057B7EF767814F) & _MASK
        out[i] = (st >> 33) & 0xFF
    return bytes(out)


class Synth:
    """The reference mini-driver: walks the plan exactly like the
    validator, producing the feed lines a CORRECT driver would log.

    Pinned cells keep the feed internally consistent (resolve cells ==
    the static-map-wh span reads, count cells == their bank prefix
    reads, the frame-counter advancing +1 per hit)."""

    MAP_W = 16
    MAP_H = 16
    ROBOT_COUNT = 3
    TRT_COUNT = 4
    OBJ_COUNT = 2000
    FC_ANCHOR = 0x1E0

    def __init__(self, plan):
        self.plan = plan
        self.pinned_cells = {}  # cell addr -> 4-B value (diagnostics)
        self.bytes_at = {}  # sparse pinned BYTE store (spans cover cells)
        self.store = {}  # written regions: addr -> bytes
        self.fc_cell = int(plan["trigger"]["frame_counter"], 16)
        n_ptr = 0
        for r in plan["resolve"]:
            a = int(r["addr"], 16)
            if r["name"] == "map_w":
                v = self.MAP_W
            elif r["name"] == "map_h":
                v = self.MAP_H
            elif r["name"] == "robot_count":
                v = self.ROBOT_COUNT
            elif r["name"] == "trt_count":
                v = self.TRT_COUNT
            else:  # obj/tot/dat/claim pointers: fabricated flat values
                v = 0x07000000 + 0x10000 * n_ptr
                n_ptr += 1
            self.pinned_cells[a] = v.to_bytes(4, "little")
            self.bytes_at.update(
                {a + i: b for i, b in enumerate(v.to_bytes(4, "little"))}
            )
        # bank prefix cells (D109): pin to the RESOLVE cell's value when
        # they coincide (trt-array's prefix IS the trt_count cell), else
        # the full-bank count (object-instances' own count cell).
        for w in plan.get("anchor_watches", []) + plan["watches"]:
            p = w.get("prefix")
            if p:
                a = int(p["addr"], 16)
                if a in self.pinned_cells:
                    continue  # already consistent with its resolve symbol
                val = self.OBJ_COUNT.to_bytes(4, "little")
                self.pinned_cells[a] = val
                self.bytes_at.update({a + i: b for i, b in enumerate(val)})

    def read(self, addr, ln, hit):
        if addr == self.fc_cell and ln == 4:
            return (self.FC_ANCHOR + hit - 1).to_bytes(4, "little")
        span = [self.bytes_at.get(addr + i) for i in range(ln)]
        if all(b is not None for b in span):
            return bytes(b for b in span)
        if any(b is not None for b in span):
            # partially pinned: a real cell mix — the generator refuses
            # rather than fabricate a half-consistent read.
            cells = [
                f"{c:#x}" for c in self.pinned_cells if addr <= c < addr + ln
            ]
            raise ValidationError(
                f"synth: read @{addr:#x} len {ln} is only PARTIALLY pinned "
                f"(cells {cells}) — the generator cannot satisfy this plan form"
            )
        return synth_bytes(addr, ln, hit)

    def write(self, addr, data):
        self.store[addr] = bytes(data)


def synthesize(plan, frames_total, out_path):
    """Produce the deterministic SYNTHETIC feed for the plan (the
    reference mini-driver: same walk, generating instead of checking)."""
    synth = Synth(plan)
    symbols = {}
    lines = [
        "DBXFEED v1",
        "# SYNTHETIC feed generated by capgen-o2 --synthesize-feed",
        "# (reference mini-driver; NOT game data — anti-ghost).",
        "kind synthetic",
    ]
    if plan.get("boot_writes"):
        lines.append("hit 0")
        for row in plan["boot_writes"]:
            data = bytes.fromhex(row["bytes"])
            a = o2_addr(row["addr"], symbols)
            synth.write(a, data)
            lines.append(f"write {a:#010x} {len(data)} {data.hex()}")
    cur = None
    for hit_no, op in plan_walk(plan, frames_total, symbols):
        if hit_no != cur:
            cur = hit_no
            lines.append(f"hit {hit_no}")
        if op[0] == "resolve":
            r = op[1]
            a = o2_addr(r["addr"], symbols)
            ln = o2_len(r.get("len", 4), symbols)
            data = synth.read(a, ln, hit_no)
            symbols[r["name"]] = int.from_bytes(data, "little")
            lines.append(f"read {a:#010x} {ln} {data.hex()}")
        elif op[0] == "inject":
            row = op[1]
            if row.get("op") == "command":
                stride = (
                    int(row["stride"], 0)
                    if isinstance(row["stride"], str)
                    else int(row["stride"])
                )
                data = bytes.fromhex(row.get("bytes", ""))
                cell = o2_addr(row["count_cell"], symbols)
                cur_b = synth.store.get(cell, (0).to_bytes(4, "little"))
                count = int.from_bytes(cur_b, "little")
                lines.append(f"read {cell:#010x} 4 {cur_b.hex()}")
                dest = o2_addr(row["base"], symbols) + count * stride
                rec = data + b"\x00" * (stride - len(data))
                synth.write(dest, rec)
                lines.append(f"write {dest:#010x} {stride} {rec.hex()}")
                bump = (count + 1).to_bytes(4, "little")
                synth.write(cell, bump)
                lines.append(f"write {cell:#010x} 4 {bump.hex()}")
            elif row.get("op") == "pad":
                slot = (
                    int(row["slot"], 0) if isinstance(row["slot"], str) else int(row["slot"])
                )
                rec_addr = o2_addr(row["bank"], symbols) + slot * 8
                xyz = (0x1234, 0x0456, 1)
                rec = (1).to_bytes(2, "little") + b"".join(
                    v.to_bytes(2, "little") for v in xyz
                )
                lines.append(f"read {rec_addr:#010x} 8 {rec.hex()}")
                for cell, v in zip(row["target"], xyz):
                    a = o2_addr(cell, symbols)
                    vb = int(v).to_bytes(4, "little")
                    synth.write(a, vb)
                    lines.append(f"write {a:#010x} 4 {vb.hex()}")
            else:
                data = bytes.fromhex(row.get("bytes", ""))
                a = o2_addr(row["addr"], symbols)
                synth.write(a, data)
                lines.append(f"write {a:#010x} {len(data)} {data.hex()}")
        else:
            _, w, reads = op
            for addr, ln in reads:
                data = synth.read(addr, ln, hit_no)
                lines.append(f"read {addr:#010x} {ln} {data.hex()}")
    with open(out_path, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(
        f"capgen-o2: synthesized {out_path} ({frames_total} hits, kind=synthetic)",
        file=sys.stderr,
    )


# ---------------------------------------------------------------- main


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument(
        "--plan", required=True, help="o2 capture plan JSON (dbx-plan --channel o2)"
    )
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--feed", help="DBXFEED v1 driver log to consume")
    g.add_argument(
        "--synthesize-feed", help="write a deterministic SYNTHETIC feed for the plan"
    )
    ap.add_argument("--out", help="DBXCAP transcript path (with --feed)")
    ap.add_argument(
        "--frames",
        type=int,
        default=None,
        help="frame records (default: plan 'frames')",
    )
    args = ap.parse_args()
    plan = load_plan(args.plan)
    frames_total = (
        args.frames if args.frames is not None else int(plan.get("frames", 3))
    )
    if args.synthesize_feed:
        synthesize(plan, frames_total, args.synthesize_feed)
        return
    if not args.out:
        ap.error("--out is required with --feed")
    kind, blocks = parse_feed(args.feed)
    try:
        emit(plan, kind, blocks, frames_total, args.out)
    except (ValidationError, ValueError, KeyError) as e:
        sys.exit(f"capgen-o2: error: {e}")


if __name__ == "__main__":
    main()
