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

The renderer now loads WEAPICON and displays owned weapon/equipment rows.
Its animated draw takes explicit per-row ages; repeated presentation does
not advance counters. The settled preview shows the purchased Needler icon
above the robot and the yellow inventory row in the positions observed in
the earlier DOSBox purchase. A real-corpus regression buys both a weapon
and equipment, checks animated versus settled output, checks repaint
stability and verifies selling restores the empty screen. Both raster tests,
release clippy/all-targets and fmt pass; corpus manifest verified. Scene-clock
ownership of the ages and DONE readiness remain integration work, alongside
popup borders, shading, controls, Auto and mission transfer.

[verified] Shop popup shading uses the 256-byte DARKPALS translation at
0x402a56 over (left,top), width=5*columns, height=7*rows. Border builder
0x440717 stages TINYFONT glyphs 95/96 and corners 97..100 on 5x7 cells.
Horizontal delay is abs(column-columns/2). Interior side index i=0..rows-3
has delay columns/2+min(i,rows-3-i). Drawer 0x43f71d uses color table
0x454cc8: [4,225,222,230,221,5,10,235,228,158,1,5], advancing to phase 11
only after its countdown expires. All coordinates preserve glyph hotspots.

The popup renderer now shades the exact catalog rectangle through DARKPALS
and draws the original delayed border using explicit panel age. The settled
Needler popup was compared with a fresh DOSBox screenshot: its outline,
background darkening and text placement agree visually. Raster tests verify
initial/settled differences, convergence by age 40 and stable repainting;
two renderer tests, release clippy/all-targets and fmt pass, with corpus
manifest checks around reads. Category text reveal, control highlights,
scene input/clock, Auto and mission transfer still remain.

[verified] Control highlights at 0x4435c6..0x4437b1 require the held
mouse-button cell, not hover. CONLITE entries/positions are Buy 0@(479,337),
Cancel 1@(479,361), Auto 2@(480,391), Increase 3@(623,314), Decrease
4@(479,314), Done 5@(568,446). DONE alone additionally tests animation
readiness. These draw gates do not establish that the associated transaction
is allowed: the click handler has separate affordability/selection gates.

A shared Control enum now owns the six original hit rectangles and highlight
placements. The renderer loads CONLITE and paints pressed feedback, including
DONE readiness. Thirteen armoury tests pass, including held-versus-hover and
blocked-versus-ready DONE; release clippy/all-targets and fmt pass; corpus
manifest verified around reads. This supplies visible input feedback but does
not yet dispatch actions from a production scene. That input/clock layer,
Auto, category text reveal and mission transfer remain necessary.

The armoury input model now consumes InputFrame pointer deltas and held
left input, dispatches manual transactions, owns presentation ages and emits
DONE only with an owned weapon and ready animation. Its draw method feeds
those ages to the raster layer. Tests move the pointer through category,
item, BUY and DONE, and verify held-plus repeat after the eight-frame
quantity debounce. Category selection uses ten frames, item selection three,
and BUY ten, following 0x441345..370/0x4431d5/0x442d32/0x442808.
Equipment replacement resets its reveal even when the row contents match.
Two input tests, release clippy/all-targets and fmt pass.

This remains an integration prerequisite: AutoRequested is an explicit
unhandled outcome awaiting the original Auto algorithm; the model is not
wired into GameHost or ShellController yet. It must not be exposed as the
completed shop until Auto, scene entry/exit, selected mission and equipment
transfer are connected and the real window journey passes. Entry/abort and
exact original frame ordering still need the integrated reference check.

## Auto audit before implementation

[verified] Auto's 3..7 outer attempts each try at most 50 random
category/item pairs (0x44161d..0x4416bf). Unavailable or duplicate/full-slot
candidates retry. Once a valid slot exists, an unaffordable candidate ends
that outer attempt, just like a successful purchase (0x4416dd..6eb,
0x44178b, 0x4417a7..7b5). Auto does not retry until it finds something
cheap enough. Its animation word starts at 7 minus the outer attempt index
(0x441641..64f/0x44173d/0x441808), not the robot type.

[verified] Top-up traverses the first n weapon slots, n being the outer
attempt count; equipment is not topped up. Each affordable weapon gets one
pack in a pass, updating amount and paid words. An unaffordable weapon marks
that slot; the pass completes, then ANY marked slot ends the top-up loop
(0x441866..0x44198b). If no weapon was purchased, top-up is skipped.
Thus remaining cash need not be exhausted. Sorting is stable descending
category rank; table 0x456c7c is [7,2,6,4,3,1,8,5,2], and swaps happen
only when the left rank is smaller (0x441a4c..5a).

[verified] Random calls use FUN_0041ec59 -> secondary generator
0x4029b6 (state 0x4ede4c/4ede4e), then the shared bounded transform at
0x41ec29. Do not consume mission RandA or substitute modulo sampling.
The pre-loop scanner purchase uses its separate original gate: balance
at least 2400 and scanner level 3 available. Auto algorithm and its
secondary RNG ownership remain to implement.

[verified] Manual sell has a no-pending-cart gate at 0x441b0f..17;
weapon row y=410..411 clamps to slot 6 (0x441b49..5a). The input model
now preserves owned items while another purchase is pending and clamps
that final hit strip. Three input tests and release clippy/all-targets pass.

Auto's transaction pass is now implemented with injected bounded secondary
random draws: refund and clear equipment, scanner pre-purchase, bounded
candidate attempts, weapon-only round-robin top-ups and stable descending
rank order. Regression draws prove that valid unaffordable candidates end
attempts, and that a failed top-up leaves 100 credits even though the
Needler could still buy ammunition. Six transaction tests, release
clippy/all-targets and fmt pass. The RNG source itself, Auto animation ages
and input dispatch remain to connect; no live Auto parity is claimed yet.

[verified] Secondary RNG 0x4029b6's byte shuffle and RCR/ADD/ADC sequence
is state = state*129 + 0x361962e9, wrapping at 32 bits; returned AX is
the new high word. Bounded helper 0x41ec29 uses
min((AX & 32767)/(32768/bound-1),bound-1), not modulo or rejection.
The shop now has an explicitly seeded isolated generator with this behavior.
Tests independently evaluate word/carry arithmetic at boundary seeds and
check bounded sequences. Two RNG tests, release clippy/all-targets and fmt
pass. Mission simulation RNG is untouched. Runtime initialization and the
shared presentation-stream ownership remain integration requirements; this
does not claim matching live random choices without matching stream state.

The input model now provides tick_with_random, which handles AutoRequested
on the caller-owned ShopRandom stream, clears the category/cart display and
leaves the generated equipment available for DONE. Its pointer regression
checks that Auto produces weapons, advances the stream, and that neutral
frames preserve the stream while another press continues it. Four input
tests and release clippy/all-targets pass.

Known integration gap: Auto currently restarts row reveal ages at zero.
Original label counters start at 7-attempt, while icon counters start at one;
the renderer/input currently share a weapon age, so these must be split and
the attempt metadata retained before claiming Auto animation parity. Runtime
seed/stream ownership, scene entry/exit and actual window validation remain.

Auto now returns label reveal phases keyed to purchased items, preserving
7-attempt through weapon sorting and equipment replacement. The input model
keeps separate weapon label and icon ages, so Auto icons begin at frame zero
while labels resume their authored phase. DONE readiness reads label ages.
The sorted three-weapon regression verifies phases [5,7,6] independently of
sorted slot order. Twenty-one armoury tests, release clippy/all-targets and
fmt pass; corpus manifest verified around raster tests.

Remaining timing caveat: the scanner pre-purchase references the previous
outer-index local before this pass initializes it; the current model starts
that special label at seven. Its exact entry/click lifecycle needs the live
integrated check. No Boot Camp impact, because scanner level 3 is unavailable
there. Scene wiring, initial secondary stream state and window journey remain.

Preparation now composes MissionRoom, ArmouryInput and ArmouryRenderer into
one owned flow. It retains the selected zone/mission while purchases occur
and emits a typed Launch only after the shop's DONE gate passes. The caller
can then transfer the retained Transactions into mission staging. A
real-corpus test uses only InputFrame deltas/buttons to select Boot Camp,
open Armoury, Auto-equip and request launch, checking visible frame change,
nonempty ammunition and the retained A1 slot. The targeted test, release
clippy/all-targets and fmt pass; corpus manifest verified.

This test terminates at the typed launch request, not a running mission.
GameHost/ShellController staging and live-window acceptance remain required.
The composed flow currently uses the no-intro shop raster; briefing action,
abort, sound and animation-enabled intro still require their host consumers.

GameHost now owns optional Preparation, routes Select/Shop input through it,
renders its plane above old loading/movie layers and switches to the selected
Mission on its typed launch result. Explicit preparation staging is atomic
on asset errors. Mission asset selection consults the retained one-based
single-player slot; load_mission transfers purchased weapon rows into robot
zero. A host-driven real-corpus input test selects Boot Camp, enters Shop,
Auto-equips and enters Mission with the correct slot. All 182 game-library
tests plus canonical_dump_gate, determinism and zone_mission_parity pass;
release clippy/all-targets and fmt pass; corpus manifest verified.

Shell staging has not enabled this host path yet. The host test reaches
Mission state before mission assets are loaded; actual transfer/activation
needs the ShellController test and window play. Current transfer covers
robot-zero weapons, not chassis equipment or other robot types. ShopRandom
is provisionally zero-initialized in GameHost; lifecycle seeding, options,
briefing and sound/intro consumers still need integration. Queue stays open.

ShellController now activates Preparation on the initial menu-start entry,
using the selected starting balance, and bypasses the legacy Shop movie
staging while Preparation owns Shop. Window absolute-pointer steering uses
the preparation cursor in Select/Shop. The production controller regression
loads Boot Camp and verifies weapon transfer; native mouse play reaches the
same route through DONE. See PLAYTEST-2026-09-06.md for evidence and limits.
This does not close the journey queue item: chassis, repeat-new-game lifecycle,
intro, options and other consumers remain outstanding.

## Equipment deployment boundary (2026-09-06)

[verified, EXW objdump] The shop's two equipment rows are not part of
the seven weapon groups. load_markers at 0x40cf77..0x40d031 walks the
0x4deafc + robot.type*0x1c table in two 0x0e increments. The jump-table
bytes at 0x40cc8c decode to 0x40cfb4, 0x40cff3, 0x40d004, 0x40d01b,
0x40d028 for names 0x2a..0x2e respectively.

| Name | Deployment write | Session row after copy |
| --- | --- | --- |
| 0x2a Auto Shielding | robot +0x8c = signed word quantity | +0, +2, +6 cleared |
| 0x2b Battery Pack | robot +0x94 = signed word quantity | +0, +2, +6 cleared |
| 0x2c Damper | robot +0x98 = signed word quantity * 200 | +0, +2, +6 cleared |
| 0x2d Scanner | scanner bank 0x46ae94[type] = 1 | retained |
| 0x2e Scanner | scanner bank 0x46ae94[type] = 2 | retained |

The first three read a dword at row +0 then arithmetic-shift by 16
(0x40cfb4, 0x40cff3, 0x40d004), so the quantity has i16 semantics.
Their shared tail 0x40cfc3..0x40cfe1 clears name, quantity and paid
price; it leaves the other shop words alone. Scanner branches skip
that consumption tail. Because rows are shared by type and consumed
inside the robot walk, cloning consumables to every robot of the same
type would be incorrect. Unknown names fall through without consumption.

[implementation finding] MissionScene::set_weapon_loadout currently
searches its seven weapon pairs for Battery Pack; the real shop puts
it in equipment, so production never reaches that compatibility path.
The next implementation must stage the separate chassis table, apply
its consumed rows in robot order, preserve scanner state, and update
the retained preparation inventory. Merely appending equipment to the
weapon list or setting every robot's battery would break this boundary.
The old synthetic battery-in-weapon test does not prove shop transfer.

The production host now deploys Preparation's weapons by robot type and
its separate equipment rows in robot order. Shield charges, battery/HP and
damper pool use signed quantities; consumed rows are removed from retained
shop inventory without refund. Scanner levels are retained by type and
scanner rows remain owned. Tests cover two same-type robots behind a
different-type robot, signed damper quantity, scanner retention, and the
real-corpus controller Auto/DONE route's resulting mission stats and inventory.
The old battery-in-weapon seam remains for historical synthetic fixtures;
production equipment uses the separate path. Scanner-driven map rendering,
return-to-shop lifecycle and post-mission inventory recapture remain open.

## Ground-input handoff audit (2026-09-06)

[verified, EXW objdump 0x410693..0x410809] The ground dispatcher
uses signed IDIV (truncate toward zero), then an arithmetic right shift:

```text
x_view = ((mouse_x - 240) * zoom) / 480
y_view = ((camera_z * 480 / zoom + mouse_y - 240) * zoom) / 480 + 21
world_x = camera_x + (x_view >> 1) + y_view
world_y = camera_y - (x_view >> 1) + y_view
world_z = 0
```

The order of divisions in y matters away from zoom 480. The old §7j.31
prose omitted the half-x mix in the final pair and flattened the nested
y expression; the instruction sequence above is authoritative. The
0x4edd54 camera-z cell is the signed average of the four selected-anchor
heights, computed in the renderer at 0x403a98..0x403b01. The x/y camera
anchors are averaged in the same block. Default zoom is 480 (0x447883).

[implementation finding] MissionScene currently routes viewport clicks
only through click_robot, an old approximate spread-order seam. Its own
32-pixel robot-box test is not the original single-player picker: original
SP writes no robot hot rectangles (0x403c87 network gate, §7j.31). Camera
is fixed on activation, another mismatch with the moving DOSBox reference.
The core already has CommandRecord processing in weapon.rs; its flag 1
sets a robot movement target, whereas flag 2 stages an order/fire triple.
Before wiring this conversion, trace the mouse-button/command-builder
selection so ordinary movement cannot accidentally become a firing order.
Existing robot-click synthetic controls cannot certify that production path.
