# Mission room implementation notes

Status: implementation in progress; no mission or phase completion.

## Sources and scope

[verified] EXW FUN_0043e7d4, committed
`ghidra-project/exw-text-objdump.txt`; decompilation navigation in
`ghidra-project/exw-pacer-names.txt`. No reference-project code used.
The mission-ID mapping and completion bank are already specified in
RE-EXW-SIM section 7j.73; shop catalog and transactions in section 7j.45.
Live EXD observations are in PLAYTEST-2026-09-06. EXW remains logic canon.

## Room assets and drawing

[verified] Entry loads SELECTOR.BIN/SELECTOR.PAL, NORMAL.BIN and
SELMONT.BIN for single-player (strings 0x4592d8, 0x4592ed, 0x45945f,
0x459472). SELMONT is the pixel-to-ID grid: the entry loop stages its
base + 12 at 0x43ec0a..14; the click handler reads byte[y*640+x] at
0x43ed91..b4. The first 12 bytes are its one-image raw BIN header.
Use the validated image parser rather than assuming this offset in new code.

[corpus verified, manifest clean] SELECTOR has 15 images: entry 0 is
640x480; entries 5/7/9/11/13 are 65x106 armoury-door frames and
6/8/10/12/14 are 95x169 briefing-door frames. NORMAL has 35 images:
0..25 are mission-region overlays, 26..34 are the table reveal animation.
SELMONT has one raw 640x480 image. SELECTOR.PAL is 770 bytes,
SELDARK.PAL is a 256-byte translation table, TXPAL3.PAL is 65536 bytes.
These translation tables are not VGA palettes.

[verified] BIN hotspot word order is **y, x** for this blitter:
0x401e5d..6a adds the first word to ECX (y), the second to EBX (x).
The existing SpriteImage.hot tuple retains file order. NORMAL entry 0
therefore lands at x=229,y=299, not x=299,y=229.

[verified] FUN_0043f430 draws completed regions using blitter mode 0x12f,
and unfinished regions in the current zone using mode 0x12e. The latter
(0x401977..0x401a06) replaces each covered destination pixel with
TXPAL3[(source << 8) | destination]. RLE skip spans leave the destination
untouched. A decoded zero must not be assumed to mean an opaque lookup.
Completed-region mode 0x12f applies SELDARK to covered nonzero source
pixels (0x401939..42). The selected region alternates modes 0x12e and
0x12c by frame&7 after the table reveal (0x43f152..1c4); 0x12c is the
ordinary image-copy arm at 0x401862.

## Selection and doors

[verified] Entry clears selected mission to zero (0x43ea9b). A region
click is valid only for the campaign's current zone; the exact ID ranges
are in SIM 7j.73. Selection opens the doors only when the corresponding
completion record exists and is unfinished (0x43f018..0x43f0a0).

[verified] Door helper 0x43f4ee increments/decrements frame toward the
open flag and clamps to 0..4. It draws SELECTOR entry 5+2*frame at
(218,20). The briefing door uses entry 6+2*frame at (447,4) only in
single-player zones greater than 1; otherwise it stays at entry 6.

[verified] Armoury activation requires a selected mission, door frame 4,
a click, 227 < x < 284, and y < 128 (0x43ec46..ae). The extra lower
comparison is literally **x > 39**, not y > 39; both assembly and the
old decompiler agree. Do not silently repair it into a different hitbox.
The briefing door similarly uses 458 < x < 542, y < 176, an extra
x > 41, zone > 1, and non-MP mode (0x43ecb1..0x43ed20).
Armoury sets return flag 1; briefing sets 2. Escape aborts the room
through 0x43ed26..72. Neither ordinary ground clicks nor clicking a
locked region may advance the journey.

[verified] Mission description loader FUN_00447216 builds the LANGUAGE
heading from OVERVIEW_ (0x45983e), zone letter and mission digit;
MP uses DM_OVERVIEW_ (0x459831). It is called at (8,8) on selection.
Text layout/effect helper internals still need a bounded implementation pass.

## Armoury follow-through

[verified, existing SIM 7j.45] Auto sells existing equipment, makes 3..7
random catalog purchases with availability/affordability checks and bounded
retries, then tops up ammunition and sorts groups. The observed STANDARD
3500-credit loadout is **one result**, not a fixed preset. Do not hard-code
it as the Auto algorithm. DONE enable semantics must be rechecked at
0x440287 before wiring; the old prose's "requires a FREE weapon group"
is not enough evidence to infer that the player may leave empty-handed.
