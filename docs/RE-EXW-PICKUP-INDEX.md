# Pickup indexing: original Boot Camp gold structures

2026-09-06, interactive original/native playtest continuation.

[observed] Original Boot Camp, after the hidden scaffold section lowers:
shooting the nearest rotating gold structure produced impact effects but
did not remove it. Walking into it removed it and increased cash from
3000 to 3010 (score360 unchanged). Native remained blocked before the
same row. The actual map has DAT type3 at level1, x3, y35 onward; its TOT
animation word is 0x81..0x84. These are pickups, not static obstacles.

[verified, retained EXW disassembly] The consumer at 0x40bf3c loads raw
terrain set [0x4edd8c], shifts it left two, and loads A at 0x454a58 plus
that index (0x40bf48). No decrement or subtraction occurs. B repeats
the raw-cell lookup at 0x40bf59..0x40bf62. The floor replacement at
0x40bfac..0x40bfb8 likewise reads 0x454a90 plus raw set times four.
The effect dispatcher repeats this exact indexing at 0x40ebaa..0x40ebb3,
then tests A+12..15 for case4. Boot Camp uses raw set1, so A=0x75 and
the gold words 0x81..0x84 select the score/money award.

[verified, read-only PE bytes from BEDLAM.EXW] Eight-address windows,
including the word read for raw set7:

| Base | Dwords at offsets 0..7 |
| --- | --- |
| 0x454a58, A | 4e,75,75,358,75,a3,a3,75 |
| 0x454a74, B | 75,535,70b,656,535,4fe,31e,70b |
| 0x454a90, floor | 70b,48f,24c,368,48f,39,39,24c |

[correction] RE-EXW-SIM 7h.5/1's structural deduction that these reads
must subtract one is contradicted by the instructions. Its claim that
Boot Camp stages zero pickups was a consequence of the same wrong index,
not independent validation. Nominal adjacent seven-dword table labels
do not override the actual indexed addresses; raw set7 intentionally
reads the next labeled window's first word. Keep the exact eight-address
windows and pass raw set1..7. Retain zone0 as an unstaged no-op.

[verified] On a recognized probe, 0x40bfa8 clears the type3 DAT byte,
0x40bfc0 writes the floor word, 0x40bfc8 sets seen1, and 0x40bff8 applies
the original pickup word. Therefore correcting the range lookup also
removes the blocking collision volume and the gold image. The existing
world-write journal can propagate these changes; no new collision rule
is needed. Award RNG and amounts remain the existing case4 implementation.

[open] Implement and test the corrected raw-cell lookup, then replay the
rebuilt native to this location. Current native PID2151744 still has the
old index and is paused only by its blocked movement, not SIGSTOP.
Original PID2106527 is SIGSTOP-paused after collecting the first gold
pickup, 474 Plasma X2 rounds, cash3010, score360. Do not rebaseline replay
digests to conceal the newly active real-corpus pickup path.
