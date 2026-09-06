# Delayed fence shutdown

## Boot Camp producer and timer (2026-09-06)

[verified, EXW] FUN_00439c20 receives the dying object type in EAX.
The jump table at 0x439c04 selects by terrain set minus one. Set 1
lands at 0x439c39: network mode 2 or mission other than 1 returns
zero; type 0x85 returns 0x847f (0x439c55/63), type 0x86 returns
0x83 (0x439c5c/70), all others return zero. Verified with targeted
objdump starting at 0x439c20; the older whole-section disassembly
misaligns the function prologue because it follows embedded tables.

[verified, corpus] ZONEA/MISSION1.POS has type 0x85 at slot 82,
origin (16,63,1). Its linked type 0x7f instances are at x=10,
y=60..68, z=1; type 0x84 instances at x=12..15,y=70,z=1.
Type 0x86 is slot 203 at (8,15,1), linked to type 0x83 at
(7,12,1) and (8,12,1). These locations identify the test targets;
no live shutdown has yet been observed.

[verified, EXW] FUN_00422e0a calls the mapping, schedules a nonzero
payload through 0x422c9b, then searches the first instance matching
the low byte and calls the chase-camera function 0x4245c9 with
its origin multiplied by 32. The destroy tail calls this producer
before the bridge-ring dispatcher. Native destroy_tail currently
leaves it as a no-op, so successful generator destruction cannot
perform the original linked shutdown.

[verified, EXW; correction to RE-EXW-SIM 7j.12 and ledger]
The timer bank is 32 records of **6 bytes**, not 0x18 bytes:
0x422c7f and 0x422dd6 increment by 6 up to 0xc0. Each record is
a dword payload and a word countdown. Allocation chooses the first
zero payload, or slot zero when full (0x422c78..9a); arm sets
countdown 8. Tick tests the old countdown: nonzero decrements only,
zero expires. Therefore an untouched newly armed timer expires on
its ninth tick, including any same-frame epilogue tick.

[verified, EXW] Expiry (0x422cd2..0x422dce) emits SFX 0x22 on
channel 3, scans instances in order matching payload & 0xff, then
payload arithmetic-shifted right by 8 (not masked again), skipping
a zero selector. Matching compares the full original id/flags
dword, sets its second byte bit 0x40, clears plane-A occupancy
at the origin, and writes the zone floor word through 0x41bd54
with seen=1. It does not run ordinary damage, scoring, chain
destruction, or full template restoration. Payload is then zeroed.

Implementation must preserve timer capacity/order/expiry semantics,
origin-only occupancy and mirror writes, and avoid substituting
ordinary building destruction. Other zone mapping branches, chase
camera behavior and sound remain to be traced/wired separately.

## Zone B mapping and radar consumers (2026-09-07)

[verified, EXW] Targeted disassembly and raw jump-table bytes establish
set 2's entry at 0x439c7b (table 0x439c04). Network mode 2 returns
zero; otherwise mission minus one indexes five entries at 0x439b40.
Mission 2 enters 0x439ce3 and maps types 0x70..0x75 through the
six entries at 0x439af4 to payloads 0x67, 0x68, 0x69, 0x6d,
0x64, 0x65 respectively. Other types return zero. This mapping is
currently absent from native fence shutdown as well as radar.

[verified, EXW; correction to the older radar summary] The object
radar pass does not classify arbitrary objects by alive/dead flags.
At 0x41f215..0x41f279 it scans POS records in order, skips full
id/flags dword -1, and requires origin strictly inside tile bounds.
It projects the origin center (2*x+1,2*y+1), calls 0x439c20,
and draws icon 9 for a nonzero linked-fence payload (0x41f2f6).
Otherwise 0x41f833 tests the complete id against a precomputed
list of linked target types and selects icon 10 on a match.
The list producer 0x41f867 scans initial objects, calls the same
mapping, inserts each nonzero low/high payload byte, and deduplicates
via 0x41f8ba. This is a mission-initial list, not a fresh list of
surviving generators on every radar refresh. Object marker rendering
must preserve that lifetime and compare complete flag-bearing ids.
The earlier description of icons 9/10 as generic alive flags is
superseded by these call-site and helper anchors.

[observed, live comparison] Both games reached B-2 from Boot Camp.
After allowing the native arrival fade and radar refresh to finish,
the original shows blue/white bars near the north fence while the
native radar remains almost empty. The missing generator/linked-type
pass is a concrete implementation gap; exact correspondence of each
colored marker still needs a post-fix live comparison. Neither the
remaining radar producers nor full Zone B gameplay is verified.
