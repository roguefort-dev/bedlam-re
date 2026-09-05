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


## First implementation boundary

MissionRoom now loads the original room/region/mask assets, translates region
pixels with TXPAL3 while preserving RLE coverage, animates the two doors,
uses the mask to select missions in the current zone, and emits typed
Armoury/Briefing/Back actions. The viewport input cannot advance through a
locked door or an unrelated zone. SELDARK shades the description rectangle
at (1,1), 205x119, exactly as 0x43eb38..4f / FUN_00402a56 specify.

The room is not yet attached to GameHost. Description text, animated border,
entry sounds/music, and the armoury consumer remain in this queue item.
A standalone local raster preview was compared with the live DOSBox room:
map geography, region placement, door art and layout agree visually; this is
not a pixel-exact timing/color verdict. The next implementation pass must
finish the text/border and connect the room and shop through production
input, not turn the Armoury action into a shortcut directly to Mission.

Regression coverage: real-corpus Boot Camp selection and door gating,
completed/other-zone rejection, translation-table placement and RLE skips;
codec coverage test distinguishes a literal zero from a skipped pixel.

Validation for this module boundary: bedlam-assets library 100 passed;
bedlam-game library 156 passed; canonical_dump_gate, zone_mission_parity and
determinism passed with unchanged pins. Release clippy/all-targets and fmt
passed. Corpus manifest verified around every asset read. Queue remains open.

## Description and border follow-up

[verified] FUN_00447216 seeks OVERVIEW_<zone letter><mission digit> in
LANGUAGE, then copies preformatted rows until the closing bracket. No word
wrapping is performed here. Calls to FUN_0043e274 stage the first row at
(8,8) with delay 1; subsequent row n starts at (8,11+10*n), delay 3*n.
The text drawer 0x43f9dd..0x43fb69 uses TINYFONT (0x46cdb0), glyph
byte-0x21, glyph width+1 advance, space advance 3. High bytes use the
existing FUN_00410493 remap and accent glyph 0x71+accent. Its solid-color
blitter 0x4027b9 colors literal RLE spans, not skipped spans.

[verified] Color table at EXW 0x454b90 is three rows of eight dwords:
state 0: 129,130,130,130,130,130,130,130;
state 1: 129,130,131,132,132,132,132,132;
state 2: 1,130,131,132,133,134,135,136.
The drawer advances color phase toward 7 after each rendered frame.

[verified] FUN_00440888 builds the (1,1), 41-column, 17-row panel border
with 5x7 cells, staging TINYFONT glyphs 95..100 through 0x4406c4.
Horizontal top/bottom delays are abs(column-20). Interior side row r=1..15
has delay 8+min(r-1,15-r). Horizontal glyph 95, vertical 96, top corners
97/98 and bottom corners 99/100 preserve their file hotspot offsets.
The same three-state color table applies. State 2 is the selected unfinished
mission, state 0 is the initial/unavailable panel. The per-entry countdown
is decremented without drawing until zero (0x43f7b1 and 0x43fb40 tails).

Description/border implementation now uses the original TINYFONT and color
ramps, including delayed reveal, glyph widths, accents and border hotspot
placement. It preserves LANGUAGE-authored rows and validates every selectable
mission description in all six shipped languages. Corpus TINYFONT slots 105,
106 and 113 are legitimately empty; they must not reject the whole font.
A selected Boot Camp preview was compared with the live DOSBox selection:
text, panel shape and open armoury door agree visually. Exact frame timing
between EXW and EXD remains outside this visual check.

## Shop DONE correction for the next integration pass

[verified, correcting SIM 7j.45] DONE requires at least one **owned** weapon
row, not a free row. The scan at 0x442a16..58 advances over zero name words;
reaching seven means no owned weapon and jumps back without exiting.
The 0x4dc694 flag is an animation-ready gate: 0x44029d sets it, and each
owned row with its word@+4 below 9 clears it and increments that word
(0x4402c2..eb, owned-row gate 0x440390..3a3). Thus +4 is not unconsumed
inside the shop, contrary to the old prose. Both the click and highlight
paths test the ready flag (0x4429f4 and 0x44369d).
