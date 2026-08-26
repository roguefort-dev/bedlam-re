#!/usr/bin/env python3
"""Fail closed until the P4 O2 operational trigger contract is exact."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
plan = json.loads((ROOT / "tools/diffharness/capture-plans/S1-o2.json").read_text())
watches = (ROOT / "tools/diffharness/watches.toml").read_text().lower()
source = (ROOT / "tools/diffharness/src/bin/dbx-plan.rs").read_text()

assert plan["trigger"]["site"] == "0x004486C9"
assert plan["trigger"]["frame_counter"] == "0x0046AE68"
assert 'exw_addr = "0x425a03"' in watches
assert 'exd_addr = "0x5a6eb"' in watches
assert '"site": "0x004486C9"' in source
assert '"site": "0x00425A03"' not in source
