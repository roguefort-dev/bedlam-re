#!/usr/bin/env python3
"""Print space-separated spawnable Now-item numbers for the nudge controller.

An item is spawnable when it is not claimed and carries no INTERACTIVE,
MANUAL, or BLOCKED tag. A BLOCKED-prefixed tag ([BLOCKED-operator-desktop]
and friends) skips exactly like [BLOCKED]: workers naturally suffix the
blocker, and an exact-match-only check respawned the unprogressable item
forever while real work starved behind it (watchdog repair 2026-08-22).
Tagged and untagged items are both eligible; untagged items produce a
stderr warning so malformed queue lines are visible instead of silently
unschedulable.
"""
import re
import sys
from pathlib import Path

queue_path = Path(sys.argv[1])
claims_path = Path(sys.argv[2])
text = queue_path.read_text() if queue_path.exists() else ""
now = text.split("## Now", 1)[1].split("## Backlog", 1)[0] if "## Now" in text else ""
claimed = {
    path.name.split("-", 1)[0]
    for path in claims_path.glob("*.claim")
}
spawnable = []
for match in re.finditer(r"(?m)^\s*(\d+)\.\s+(.*\S)\s*$", now):
    item, rest = match.groups()
    tags = {tag.strip().upper() for tag in re.findall(r"\[([^]]+)\]", rest)}
    if any(tag.startswith("BLOCKED") for tag in tags) or tags & {"INTERACTIVE", "MANUAL"}:
        continue
    if not tags:
        print(f"warning: queue item {item} has no [tag]; scheduling it anyway", file=sys.stderr)
    if item not in claimed:
        spawnable.append(item)
print(" ".join(spawnable))
