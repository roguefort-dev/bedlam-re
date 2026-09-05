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

## Armoury catalog and manual transaction reference

[verified] The straight-line initializer at EXW 0x44395b..0x4440e5
writes the following catalog. Entries are (name id, price, pack amount);
all availability fields initially contain 1, before the overrides below.
This table was evaluated from MOV/XOR instructions in the committed EXW
objdump, including both absolute-store encodings. No reference-project code
was used. Geometry is (x, y, yoff, count, columns, rows).

| Category | Geometry | Items |
| --- | --- | --- |
| 0 | (237, 97, 37, 3, 26, 6) | (2, 100, 300); (3, 250, 400); (4, 400, 500) |
| 1 | (390, 97, 37, 3, 23, 6) | (9, 500, 1); (10, 700, 1); (11, 900, 1) |
| 2 | (603, 200, 56, 4, 23, 7) | (37, 200, 24); (38, 400, 36); (39, 600, 72); (40, 800, 144) |
| 3 | (397, 364, 59, 6, 26, 10) | (24, 250, 60); (25, 350, 30); (27, 50, 96); (28, 100, 144); (29, 100, 96); (30, 200, 144) |
| 4 | (280, 375, 62, 6, 26, 10) | (20, 100, 80); (21, 200, 120); (22, 350, 160); (16, 150, 60); (17, 250, 120); (18, 400, 180) |
| 5 | (165, 356, 50, 1, 20, 3) | (14, 500, 20) |
| 6 | (95, 326, 50, 3, 26, 6) | (6, 200, 300); (7, 500, 600); (8, 800, 900) |
| 7 | (46, 269, 46, 4, 23, 7) | (32, 200, 24); (33, 350, 36); (34, 700, 72); (35, 950, 108) |
| 8 | (68, 204, 46, 5, 25, 9) | (42, 500, 15); (43, 250, 5); (44, 300, 25); (45, 400, 1); (46, 800, 1) |

[verified, correcting SIM 7j.45] 0x46cd48..0x46cd80 is **15** dwords,
not 16. These are availability flags: zero hides an item as CLASSIFIED;
nonzero enables it. The copy at 0x444184..0x444215 maps successive flags
to (category,item): (3,0), (0,2), (7,1), (2,0), (1,2), (5,0), (4,4),
(7,2), (2,1), (2,2), (7,3), (2,3), (4,5), (6,2), (3,1).
Shop entry sets all 15 flags to 1 for multiplayer or zone 7
(0x4411e1..0x44124c). Zone 7 skips the campaign-copy overrides.
Multiplayer additionally disables category 2 and all equipment, category 8
(0x44421f..0x444251). Scanner level 3 (8,4) is disabled unless mode is
zero and the valid campaign zone is 2..4 (0x44414a..0x444164).
The fresh-campaign reset source for all 15 flags still needs tracing; the
live original confirms Needler #3 is CLASSIFIED in a new Boot Camp game.

[observed, original EXD in DOSBox] Starting STANDARD balance 3500, selecting
Needler Cannon #1 displays CASH:3400 AMT:300. Holding the plus control
repeats additions: the inspected state reached CASH:2800 AMT:2100.
Clicking BUY then shows BALANCE:2800 and one owned Needler row. Clicking
that owned row removes it and stages the same 2100-ammo purchase, showing
CASH:2800 AMT:2100 again. Clicking CANCEL clears the staged purchase and
reveals BALANCE:3500 with no weapon rows. Thus the sell refund is real and
the CASH display subtracts the pending spend; it is not the bank balance.
The plus observation verifies repeat behavior, not an exact click count
or frame cadence.

[verified] Weapon BUY at 0x4427d2..0x4428a2 writes the first free row after
duplicate-name rejection, records staged amount and spend, initializes the
row animation to zero, then subtracts spend from balance. CANCEL at
0x442927..0x442974 clears selection without touching balance or owned rows.
These observations are transaction evidence, not completion of the remaster
shop: rendering, production integration and equivalent live input remain.

The Rust armoury catalog now contains all 35 authored items and their category
geometry. Availability takes explicit campaign flags and mode/zone inputs;
it does not invent a campaign unlock progression. Tests cover the original
Boot Camp Needler popup, transient nonzero flags, final-zone overrides,
scanner restrictions and multiplayer exclusions. Catalog tests, release
clippy/all-targets and fmt pass; corpus manifest verified. The catalog is a
prerequisite module, not yet the production shop or a completed queue item.

[verified] Manual selection checks affordability before availability
(0x442c6e..c87). Insufficient balance leaves the existing cart alone; an
unavailable affordable item clears it. Duplicate/full-slot rejection by
0x4437ea/0x443870 clears selection via 0x442e70 or 0x443065. Selection
stages one pack. Minus requires resulting amount > 0 (0x442486..48a),
correcting the earlier zero-floor prose. Both quantity controls reject
scanner items (8,3)/(8,4); plus checks total spend <= balance first.
Equipment same-name selection reuses its slot, and BUY replaces its amount
and paid word rather than accumulating or refunding the previous purchase
(0x443899..8ac and 0x44270f..785). This original behavior is retained.

Manual transactions now have a Rust model separating the pending cart from
balance and the seven weapon/two equipment rows. The observed 3500-credit
Needler buy/sell/cancel sequence passes as a behavioral regression. Additional
checks cover one-pack minimum, affordability, scanner mutex/quantity, duplicate
weapons, same-equipment replacement and the entry money floor. Persisted amount
and paid fields retain the original word truncation. Four focused transaction
tests, release clippy/all-targets and fmt pass. This module does not yet own
rendering, input debounce, animation-ready DONE gating, Auto or mission transfer;
those remain necessary before the shop can satisfy the production journey.

## Armoury pointer geometry

[verified] Category field +0x10 is a click radius, not a vertical offset.
The click path computes octile distance max(abs(dx),abs(dy)) +
min(abs(dx),abs(dy))/2 through 0x41ebf8, selects the first strictly nearest
category below 100, then rejects distances above its +0x10 radius
(0x44125b..29a, 0x4430c6..3153). Popup left is clamp(anchor_x -
5*columns/2, 10, 630-5*columns); top is anchor_y
(0x4440e5..4148). Item hit rows use strict horizontal interior and
y in [top+4+9*i, top+13+9*i), independently of the rendered row count
(0x4412bc..0x441345). Popup names are at left+5, top+7+9*i; prices at
left+5+5*columns-44, same y (0x44326b..334c).

[verified] Immediate text helper 0x43fe8a uses glyph byte-0x21,
width+1 advance, space 3 and accent glyph 0x71+accent. It colors literal
RLE spans through 0x4027b9, matching the mission panel's glyph grammar.
The caller supplies either TINYFONT or SMLFONT; their palette indices and
roles must remain distinct when the screen renderer is connected.

Catalog geometry now exposes the original popup origin, item hit rows and
nearest artwork selection. The misleading `y_offset` field is renamed
`click_radius`. Five catalog tests pass, including popup edge clipping and
row boundaries; release clippy/all-targets and fmt pass. Visible rendering
and controller integration remain outstanding.

## First armoury raster integration

The raster module loads the ten full-screen SHOPLITE images, SHOPPAL,
TINYFONT and SMLFONT, and draws category names/prices plus the pending
purchase or balance. A standalone raster was compared with the live DOSBox
armoury. Artwork, category placement and right-hand label scale agree
visually. This is not a complete shop: panel darkening/borders, owned rows
and icons, reveal effects, controls and production scene wiring remain.

[verified] The right-hand font pointer 0x4ede7c is SMLFONT (allocation
0x41d648, documented boot asset mapping), not the locally loaded SHOPFONT.
The steady redraw 0x4433ef/0x44345e uses color 253, superseding the
click-path color 191; using 191 persistently produced incorrect white text
in the first raster. Category text's final ramp entry is color 5 from
0x454bf0. Both corrections were visually checked against DOSBox. The
renderer currently uses the no-intro SHOPPAL path; SHOP.SMK palette/intro
playback is still required for the animation-enabled path.

One real-corpus raster test passes across all category pages and the
pending purchase display; release clippy/all-targets and fmt pass. The
corpus manifest was verified around preview and test reads.

[verified] Owned rows in FUN_00440287 use TINYFONT at weapon
(538,342+10*slot), equipment (547,417+10*slot). Color table 0x454ca0
is [4,64,74,58,42,128,160,158,1,207], indexed by row animation word +4,
which advances toward 9. Weapon icon counter +12 advances from 1 to 12;
WEAPICON entry is category*12+counter-1. Its position is
x=[318,92,546,156,484,236,406][slot]-29,
y=[89,145,145,202,202,181,181][slot]-27 (tables 0x456c0c/0x456c28,
draw 0x440307..341). Equipment has text but no icon in this drawer.
These counters are presentation state; painting must not advance them.
