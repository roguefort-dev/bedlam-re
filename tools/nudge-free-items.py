#!/usr/bin/env python3
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
for match in re.finditer(r"(?m)^\s*(\d+)\.\s+((?:\[[^]]+\]\s*)+)", now):
    item, raw_tags = match.groups()
    tags = {tag.strip().upper() for tag in re.findall(r"\[([^]]+)\]", raw_tags)}
    if tags & {"INTERACTIVE", "MANUAL", "BLOCKED"}:
        continue
    if item not in claimed:
        spawnable.append(item)
print(" ".join(spawnable))
