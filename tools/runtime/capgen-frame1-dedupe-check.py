#!/usr/bin/env python3
"""capgen-frame1-dedupe-check — the D140(2) headless verification.

Proves, WITHOUT a game session, that dbx-capgen's frame-1 row list is
safe to stitch for every committed O1 plan:

  (a) the patched list (the REAL dedupe_frame1_rows from
      dbx-capgen.py, imported — never a copy) has UNIQUE ids over all
      13 committed plans, and
  (b) it EQUALS the anchor list (keep-first over the subset property:
      every per-frame id rides anchor_watches, so the deduped union is
      the anchor list in anchor order) — the same property capgen-o2
      proved for the O2 channel, and
  (c) the pre-fix landmine is gone from the source: the literal
      `anchor_watches + watches if frame == 1` concatenation (the
      DuplicateWatchId rejection the stitcher would raise, dump.rs)
      no longer exists in dbx-capgen.py.

Unattended-safe: no DOSBox, no game, no corpus read (the committed
plans under tools/diffharness/capture-plans/ are repo files).
"""
import glob
import importlib.util
import json
import os
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CAPGEN = os.path.join(REPO_ROOT, "tools", "runtime", "dbx-capgen.py")
PLAN_GLOB = os.path.join(REPO_ROOT, "tools", "diffharness", "capture-plans", "*.json")

# import dbx-capgen.py (module name carries a dash — load by path)
spec = importlib.util.spec_from_file_location("dbx_capgen", CAPGEN)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

# (c) the landmine expression is gone from the shipped source
src = open(CAPGEN).read()
if "anchor_watches + watches if frame == 1" in src:
    sys.exit("capgen-frame1-dedupe-check: FAIL: the literal frame-1 "
             "concatenation is still in dbx-capgen.py (the D140(2) landmine)")

plans = sorted(glob.glob(PLAN_GLOB))
if len(plans) != 13:
    sys.exit(f"capgen-frame1-dedupe-check: FAIL: expected 13 committed "
             f"plans, found {len(plans)}")

for path in plans:
    with open(path) as f:
        plan = json.load(f)
    anchors = plan.get("anchor_watches", [])
    watches = plan.get("watches", [])
    name = os.path.basename(path)

    rows = mod.dedupe_frame1_rows(anchors, watches)
    ids = [r["id"] for r in rows]

    if len(ids) != len(set(ids)):
        sys.exit(f"{name}: FAIL: duplicate ids in the deduped frame-1 list")

    anchor_ids = [w["id"] for w in anchors]
    expected = list(dict.fromkeys(anchor_ids))  # keep-first over anchors alone
    if ids != expected:
        sys.exit(f"{name}: FAIL: frame-1 ids != anchor list "
                 f"({len(ids)} vs {len(expected)} rows)")

    missing = [w["id"] for w in watches if w["id"] not in set(anchor_ids)]
    if missing:
        sys.exit(f"{name}: FAIL: per-frame ids outside the anchor list: {missing}")

    raw_dupes = len(anchors) + len(watches) - len(set(anchor_ids) | set(w["id"] for w in watches))
    print(f"{name:14} frame1={len(ids):3} unique anchors={len(anchor_ids):3} "
          f"per-frame={len(watches):3} (subset; raw concat would have "
          f"duplicated {raw_dupes} ids)")

print("capgen-frame1-dedupe-check: ALL GREEN (13 plans, unique ids, "
      "frame-1 == anchor list, landmine expression absent)")
