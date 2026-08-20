# RE: BEDLAM.EXW - title menu screen (P2g slice)

Provenance: Ghidra headless `-process BEDLAM.EXW -noanalysis` + postScript
`tools/ghidra-scripts/ExwTitleMenu.java` (MENU_ITEMS table xref census,
full decompile + asm listing of NameEntryScreen@0043a5fc + depth-1 callee
closure, callers census). Raw dump: `ghidra-project/exw-titlemenu.txt`,
log `ghidra-project/process-exw-titlemenu.log`. Jump tables decoded from
the raw EXW image (python, file off = VA - 0x400C00; switch tables are
data blobs inside the text stream, so objdump windows were opened per
case target). Data-side anchor: the LANGUAGE.* `[MENU_ITEMS]` table,
96 entries x 0x30, loaded at boot into 0046af5c (D35); entry texts read
from `game-data/BEDLAM/LANGUAGE.ENG` (indices below are 0-based into
that list). Same tag discipline: [verified] = read in decompile + asm;
[inferred] = strong deduction; [hypothesis] = plausible, unconfirmed.
Addresses are EXW VAs.

## 1. Screen identity and lifecycle [verified]

`NameEntryScreen @0043a5fc` (size 0x220C; predecessor-named for its most
prominent sub-feature) IS the title/options menu screen. Called by
GameMain @0041c050 at the outer restart point LAB_0041c3d6 (0041c3ca),
after the boot attract, and once per outer restart (its only caller).

Entry sequence (asm 0043a5fc..0043a739):
1. Assets: `ArenaAlloc(0x64000)` backdrop buf; `ArenaAlloc(86000)`
   LANGUAGE text arena; FULLFONT.BIN bank (800) + FULLPAL.PAL (300000
   scratch); LOAD_{UK,US}.BIN via DAT_0046ae64 region flag + matching
   LOADPAL(U) (identical fetch set to the P4 shell step-1 fetch gate).
2. SFX: SfxLoad("SOUND\SFX\MENU1.RAW") -> 004edfc0 (hover),
   SfxLoad("SOUND\SFX\MENU2.RAW") -> 004edfc4 (click), click len
   0x50 -> 004edbc0.
3. Reset: g_input_seen(004edb50)=0, in-menu flag 0046ae78=0, attract
   counter 0046cbec=0, InputReset(0041fa3f).
4. Menu init: `FUN_00445b5c(1)` (main menu) @0043a720, sel=-1.
5. Audio: MusicStop(3) @0043a72f; `load_midi("SOUND\MIDI\OPTIONS")`
   @0043a739; MusicStart(3) fires after the attract replay / re-entry.

Exit: on `local_d8 != 0` (start-game or quit accepted) -> SetPaletteIndex
(0x90) + return @0043c7f1 tail; GameMain then proceeds into the episode
loop (or exits for quit). local_d4 selects: 1 = start game (draws
"Loading"(idx 66)@row 0xb4 + "Please wait..."(idx 67)@row 0xd2 base
0x82 first), 2/3 = multiplayer lobby ("Locating Players" idx 7 @row
0x172 base 0, "You are player N of M" built from idx 8/9 + literals
0x4590f5/0x4590f7, then FUN_00448ef1 (HEREIAM network lobby) when
_DAT_004edb88 != 0).

## 2. Menu data model [verified]

**Builder `FUN_00445b5c(menu_id)` @00445b5c** (EAX = id, stored to
**0046ae7c**); jump table @0x445b48 (5 entries):

| id | target | items (MENU_ITEMS idx) | count |
|---|---|---|---|
| 1 | 0x445b8b | New Single Player Game(3), Start Saved Game(30), \<difficulty via FUN_00446522(difficulty@0046cbf8)\>, "Name:"(31)+lit(0x459779)+player name(004e444c), View Hall of Fame(5), Credits(68), Quit to Windows(94) | 7 |
| 2 | 0x445ce3 | Start Cooperative Game(14), Start Head2Head Game(15), \<"Number of Players: N" via FUN_004464b8(0046cbe0)\>, Main Menu(16) | 4 |
| 3 | 0x445db0 | 5 save-game slots (names from SAVEGAME via FUN_00446f4f, stride 0xb4 at 004eae58, "EMPTY" 0x45980f if none) + Cancel(32) | 6 |
| 4 | 0x44611e | same construction as 3 (variant used by the multiplayer path) | 6 |
| 5 | 0x445d71 | Quit to Windows(94), Main Menu(16) | 2 |

Slot store: dword **004eabd0** = { lo: scratch, hi(word @004eabd2):
item count }; 7 slot strings @004eabd4, 004eac04, 004eac34, 004eac64,
004eac94, 004eacc4, 004eacf4 (stride 0x30). Invalid id -> fatal
"Error constructing Menu" (0x45977b, FUN_0044d2da(3)).

Formatters: FUN_00446522(d) -> difficulty string idx 0..2; FUN_004464b8
(n) -> "Number of Players: N" idx 17..27 (n-2), players clamped 2..12.
FUN_004473cd(int, buf) = int->decimal (used for save-game level text).

**Draw `FUN_0044653a(sel, bank)` @0044653a**: count = hiword(004eabd0);
row_base = **0x1d6 - count\*0x18** (bottom-anchored); item i drawn at
row row_base + i\*0x18, string slot i (stride 0x30), via font drawer
FUN_0043c87c with glyph base **0x82 for i == sel, 0 otherwise**
(corpus-pinned, see sec 2a: the two sets are the SAME shapes in two
FULLPAL ramp slices - selected = green set, unselected = blue set). Draw cycle everywhere: PresentCopy
(00425a1e) backdrop re-blit -> draw -> palette fiddles (004edbf8
[0x2a2..0x301] = 0x3f fill = the 96-entry fade window) -> FUN_004258d0
palette commit -> FadeSetup(10) -> PresentEnd (00425a03) -> lock
(0041e215) unlock (00425aa0) around draws.

## 3. Input dispatch [verified - asm 0043a79e..0043a996]

Main loop LAB_0043a77e: per iteration

- **Attract timeout**: counter 0046cbec increments while idle; at
  `>= 0x300` (768) AND movies enabled (DAT_0046cca4) AND in-menu flag
  0046ae78 == 0 (0043a7b1..0043a7cd) -> MusicStop(3), clear screen,
  FadeSetup, InputReset, arm skip gate 004edbc4=1, **FUN_004459f7 =
  TITLE.SMK replay** (letterboxed 0x50, skippable - the ONLY skippable
  movie), gate back to 0, SetPaletteIndex(0x5d), MusicStart(3), redraw
  menu. If audio HW absent (004ede5c/004ede58 zero) -> 0046ae78 forced
  1 -> attract disabled. Hover change or click resets 0046cbec = 0
  (0043a8b0, 0043a7f1-area).
- **Hit test** (hover; 0043a934..0043a996):
  - x: cursor must satisfy `0xdc < g_cursor_x(004eddc4) < 0x1a4`
    (i.e. x in [221, 419], a ~199px strip centered on 320);
  - y: `top < g_cursor_y(004eddc8) < 0x1d6` where
    `top = 0x1d6 - count*0x18`;
  - index = (y - top) / 0x18 (signed idiv), clamped [0, count];
    outside -> -1. No per-item rect table - the strip IS the hit model.
- **Hover change** (sel != prev): PresentCopy + FUN_0044653a(new sel)
  redraw + palette + PresentEnd, and SFX MENU1 (FUN_0043a48e(
  004edfc0, 0,-1,-1,2)) if debounce local_b8 == 0; debounce = 4 ticks.
- **Click** (g_scroll_flags@004eddcc != 0, i.e. any mouse button; sel
  may be -1): SFX MENU2 if debounce 0, then
  `switch (0046ae7c - 1)` @0043aa9b, table @0x43a5e8:
  case 0 menu 1 @0x43c060, case 1 menu 2 @0x43c258, case 2/3 menus 3/4
  @0x43c5a0/0x43c5b1 (adjacent stubs - shared body), case 4 menu 5
  @0x43c0bd.
- Sub-waits (credits/HOF pages): spin on ScrollUpdate until
  g_scroll_flags != 0 OR g_input_seen(ESC latch) OR counter 004edbcc
  timeout (2000 = 20 s @100Hz tick, PACER doc) - NOT a frame pacer.

## 4. Menu 1 item actions [verified - sub-switch on sel+1, table @0x43a5b8]

| sel | item | handler | action |
|---|---|---|---|
| -1 | (click outside strip) | 0x43aad5 | players 0046cbe0=2; FUN_00445b5c(2) -> MULTIPLAYER menu; sel=-1 |
| 0 | New Single Player Game | 0x43aaa3 | 004edb88=0, mode 0046cbe0=1, DAT_0046ae70 = 4000 - difficulty\*500, exit local_d4=1 |
| 1 | Start Saved Game | 0x43abe4 | FUN_00446f4f (SAVEGAME names) -> FUN_00445b5c(3) load menu |
| 2 | \<difficulty\> | 0x43ab7e | 0046cbf8 = (d+1) mod 3 (SIMPLE/STANDARD/BEDLAM), rebuild menu 1 |
| 3 | Name: X | 0x43ae5e | name-entry sub-loop (below) |
| 4 | View Hall of Fame | 0x43ac28 | FUN_00446ebc; HOF screen (below) |
| 5 | Credits | 0x43b097 | CREDIT pages (below) |
| 6 | Quit to Windows | 0x43b058 | FUN_00445b5c(5) quit-confirm menu |

**Name entry** (extends RE-EXW-INPUT sec 5): loop `while
g_keystore[0x1c] (ENTER; 004edc44+0x1c = 004edc60) == 0`: AnyKeyWait ->
scan; Backspace (0xe or 0xd3) shortens; len < 8 -> ScanToChar, append at
004e444c, wait release via keystore[scan]; rebuild menu 1 each key;
cursor blink glyph entry 0x8e at x = (width("Name: ")+width(name))/2 +
0x146, row 0x1d6-(count-3)\*0x18, shown while (g_frame_count & 0xc) != 0.
On exit: FUN_0044efb3(name, 0x459078) validation; empty -> 0046cd0c=1 +
FUN_0044ef51(7) default name; **FUN_0042540c persists the config**:
SOUND(004ede5c)/SPEECH(004eb93c)/CINEMATICS(0046cca4)/ACTIONPAN
(004edbd8)/LANGUAGE(004eba1c)/DEFAULTNAME(004e444c,8) via
FUN_0044ed98/FUN_0044edcc + FUN_0044ed84 commit (the CONFIG.BDL
writer family).

## 2a. Glyph base 0 vs 0x82 [corpus-pinned, this run]

FULLFONT.BIN entries k and 0x82+k (k = 0..) are the SAME glyph shapes
(identical w/h, hotspots, pixel counts - python probe mirroring
sprites.rs + codecs.rs on game-data FULLFONT.BIN). They differ ONLY in
palette indices: base 0 pixels span 244..255, base 0x82 pixels span
233..244 (overlap exactly at the shadow entry 244). Mapping through
the FULLPAL.PAL ramp (98-byte blob = lead-in `e0 20` + 32 triples =
DAC entries 224..=255, per engine bedlam-assets pal.rs and the D35
LAB_0041c69e tail-copy proof):

| entry | RGB (6-bit) | role |
|---|---|---|
| 233 | 3f 3f 3f | white outline (base 0x82 set) |
| 234..243 | (v,3f,v), v 0x38..0x00 | green body ramp (base 0x82 set) |
| 244 | 02 03 08 | dark shadow (shared) |
| 245 | 3f 3f 3f | white outline (base 0 set) |
| 246..255 | (0x37,0x3a,0x3d) .. (0,0x16,0x2f) | blue-grey body ramp (base 0 set) |

So the menu's SELECTED item renders GREEN, unselected BLUE (both with
white outline + dark shadow). The HOF own-row and credits line0 use
base 0 (blue) while their siblings use 0x82 (green) - i.e. on those
screens green is the body style and blue marks the special line; on
the menu strip green marks the selection. [facts pinned; aesthetic
reading tagged inferred]


**There is no separate Options screen in EXW**: MENU_ITEMS entries 47
"Options" and 48..58 (Double Buffer..No CD Audio) are UNREFERENCED
(zero xrefs into their table slots across the whole census) - the
options ARE the main-menu items (difficulty, name, players, volume via
the mission shell), and the "options entry path" = menu-1 items 2/3 +
FUN_0042540c config save. [The toggle strings are likely the DOS
build's option screen; EXW dropped it.]

## 5. Other menu dispatches [verified]

**Menu 2** (sub-switch @0x43c666, table @0x43a5d8, on sel):
- 0 Coop @0x43c0ce: 0046ae70=4000, 004edb88=1, exit local_d4=2.
- 1 Head2Head @0x43c108: 0046ae70=0x5dc(1500), 004edb88=2, exit
  local_d4=3.
- 2 players item @0x43c128: g_scroll_flags bit0 (left) -> 0046cbe0++
  wrap at 0xd -> 2; bit1 (right) -> --, wrap <2 -> 0xc; rebuild menu 2.
- 3 Main Menu @0x43c1e1: FUN_00445b5c(1), sel=-1.

**Menus 3/4** (load; shared body @0x43c5a0): sel 0..4 = save slot
@004eae58+sel*0xb4; requires exists-dword (+0xc); FUN_0044745e +
MemCopy restore; zone from slot (004edd8c), completion bits:
FUN_004474ef(zone, 1..5) per bitmask bits 0..4 of the stored word, plus
partial zone bits; 004dd40c/0046ae70 (level ctx + score), difficulty
0046cbf8, squad stat arrays 004de664 (7 shorts x 0x62 stride) /
004deafc (0x1c), misc dwords 0046cd48/5c/70; mode 0046cbe0=1,
004edb88=0, exit local_d4=1. sel 5 = Cancel(32) -> menu 1.

**Menu 5** (quit confirm @0x43c0bd): sel 0 "Quit to Windows" ->
FUN_0041c9f0(4) (22-byte post-quit helper; DAT_0046ae7c-1 passed);
sel 1 "Main Menu" -> menu 1.

## 6. Hall of Fame and Credits [verified]

**HOF** (menu-1 sel 4): FUN_00446ebc loads; title str 0046c12c row
0x14; 10 entries i=0..9: rank strings via BmpNameBuild(0044d1f2) into
004ee404; two columns - names via FUN_0043cd7b (left pen x
0x50 + i\*0x1e + (i?0x14:0), row 0x17c) and scores via FUN_0043ce3c;
own entry (i == DAT_0046ae90) glyph base 0, others 0x82. Wait: 3000
iterations of ScrollUpdate until click/any-key, then back to menu.

**Credits** (menu-1 sel 5): title 0046bc1c (idx 68) row 10 base 0x82;
LANGUAGE blocks **CREDIT_1..CREDIT_13** (FUN_00424679 heading seek ->
FUN_004245e6 -> word loop), rows from 0x3c step 0x19 (+0x14 between
blocks); line 0 of each block base 0, lines 1+ base 0x82; CREDIT_13
two-column layout switch (FUN_0043c87c / FUN_0043c9bc / FUN_0043cafd
variants; first 0x10 lines cap on column 1, rows 0x55+). Page waits:
2000-tick timeout OR click (ScrollUpdate/g_scroll_flags) OR any key
(g_input_seen) between pages.

## 7. Globals added to the map this pass

| VA | meaning | tag |
|---|---|---|
| 0046ae7c | current menu id 1..5 (set by FUN_00445b5c) | verified |
| 004eabd0 | dword {scratch, count(hiword @004eabd2)} | verified |
| 004eabd4..004eacf4 | 7 menu slot strings, stride 0x30 | verified |
| 0046cbf8 | difficulty 0..2 (SIMPLE/STANDARD/BEDLAM); also endgame variant flag in GameMain | verified |
| 0046cbe0 | player count 2..12 (multiplayer); 1 = single-player mode marker | verified |
| 004edb88 | game start variant: 0 single, 1 coop, 2 head2head (local_d4 pairing 1/2/3) | verified ops / inferred names |
| 0046ae70 | start score/cash seed: 4000 - difficulty\*500 (single), 4000 coop, 1500 h2h, or from save | verified |
| 0046cbec | attract idle counter; >= 0x300 triggers TITLE.SMK replay | verified |
| 004edfc0 / 004edfc4 | MENU1 (hover) / MENU2 (click) SFX handles | verified |
| 004edc60 | g_keystore[0x1c] = ENTER (name-entry exit) | verified arithmetic |
| 004e444c | 8-char player name buffer (DEFAULTNAME config key) | verified (input doc) |
| 0046ae90 | HOF own-entry index (highlight row) | verified use |
| 004eae58 | 5 save-game slot records, stride 0xb4 (exists dword +0xc) | verified |
| 004edbcc | shared 2000-tick page-wait counter (tick-incremented) | verified (pacer doc) |
| 0046cd0c | empty-name flag (set when name validation fails) | verified ops / hyp role |

## 8. Open questions

- Glyph base 0 vs 0x82: RESOLVED this run (sec 2a) - same shapes,
  green (0x82) vs blue (0) FULLPAL ramp slices.
- FUN_00448ef1 (HEREIAM lobby, 2953 bytes, skipped as large): the
  multiplayer session flow (uses "Locating Players" idx 7) - separate
  slice if P4 ever needs multiplayer.
- Menu id 4 vs 3: identical construction; who calls FUN_00445b5c(4)?
  (Not NameEntryScreen; probably the lobby.) [inferred]
