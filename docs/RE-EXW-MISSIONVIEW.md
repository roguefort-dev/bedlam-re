# RE: BEDLAM.EXW — mission isometric viewport render chain (P4 render half)

Decoded 2026-08-21 (worker b9aaaa38, claim 1). All addresses EXW-anchored
from Ghidra dumps `ghidra-project/exw-missionrender{,2,3}.txt` (scripts
`tools/ghidra-scripts/ExwMissionRender{,2,3}.java`, `-process BEDLAM.EXW
-noanalysis`). 8street names in parentheses are navigation aids only.

## 1. Data flow overview [verified]

```
load_mission (7c)            init_tiles@00407e11         FUN_00403938 per frame
  TOT  -> 0x4ede20 (+4 skip)   viewport cache 0x4ede24     terrain loop (§5)
  BIN  -> 0x4ede1c             TOT words -> typeDB          LNK walk + blit
  LNK  -> 0x45cdda (in-image)  0x4796bc/cc                 FUN_00401471 into
  DAT  -> 0x4edd58 (+4 skip)   (§3)                        0x64000 buffer @0x4ede18
                                                            -> present 0x401107 (§7)
```

- **BIN** = `EDITOR\ZONE{A..G}\MISSION{A..G}.BIN` (path2 + `.BIN`,
  7c §1) — the zone TERRAIN SPRITE BANK (ZONEA: MISSIONA.BIN, 2.0 MB),
  read into `_DAT_004ede1c` [verified: FUN_00403938 loads ESI from
  0x4ede1c before every FUN_00401471 call, 0x40693c/0x406a54/0x406b1f;
  0x4069f5 is the 4th load site]. UPDATE 2026-08-22 (§7j.36 — the
  full content-consumer census): the bank has exactly THREE reader
  clusters + the loaders — (a) the terrain loop (above), (b) the
  BRIEF-minimap drawer (UPDATE §7j.49 2026-08-23: the sites
  0x440d1c/0x440d93 belong to FUN_00440c34, the draw half of
  FUN_00440dc2 = the BRIEF objective-minimap SNAPSHOTTER — type-DB
  word → FUN_00401471, EBX=0, dest bounds-checked into the
  backbuffer window; BRIEF screen only, its OWN [0x4ede18]
  allocation — never runs during the mission render pass, so the
  §1/§7 overwrite-ordering question between it and the terrain pass
  is closed by screen lifecycle: no shared buffer exists),
  and (c) FUN_00401010 = a 9-sprite RADAR STAMP that WRITES INTO
  the bank (5× downsample + 2:1 deshear of the 480×480 viewport at
  the camera, into scratch sprites u32[0x454b00+4·set]..+8 — a
  shipped-inert feature: the seven runtime-selected lettered banks each
  carry nine fmt-0 stubs physically encoded as the 6-B
  `{u16 fmt=0,u16 dy=64,u16 dx=64}` followed by 4096 raw bytes. The
  blitter reads the first four zero raw pixels as gate/rows and returns;
  LNK is identity on their ids, so the TOT references in zones A–D
  render nothing). Container grammar (§7j.36, tested seven runtime
  banks): u16[bank+0] = count (→ write-only 0x46cdb8); directory entry =
  bank+2+4·id, sprite = entry + u32[entry] SELF-relative. Those banks
  contain only the nine fmt-0 stubs and fmt-7 compressed records; this
  census makes no global claim about fmt 1..3 in other BIN banks.
- **LNK** = u16[8192] near-identity permutation (FORMATS-MISSION §5),
  loaded to the IN-IMAGE buffer 0x45cdda (0x8000) [7c §2]. Consumer
  FOUND (this pass): it is the **tile animation link table** — the
  render resolves `sprite = LNK[word]` and writes the result BACK into
  the type-DB word, so each rendered frame advances one step along the
  LNK cycle (identity cells = static tiles; 3–10-cell cycles = animated
  water/machinery) [verified 0x40686a..0x4068c9 + writeback].

## 2. init_tiles@00407e11 — viewport cache + type DB [verified, full disasm]

Two passes, zero-fill helper FUN_00402965 (rep-stos, args in ECX/EDI):

1. **36×36 ISO viewport tile cache** at `DAT_004ede24` (ArenaAlloc
   0x3cc0 in FUN_0041d954 = 1296 × 12 B entries), count in
   `_DAT_004ede28`:
   ```
   for gy in 0..36:
     for gx in 0..36: x = 0x130 + gx*0x20 - gy*0x20
                      y = -0x100 + gy*0x10 + gx*0x10
       if 0 <= x < 0x260 and 0 <= y < 0x320:      // 608×800 window
         first = (first==0) ? gx+5 : first         // sticky = 17
         entry = { s32 buf_off = y*0x280 + x;      // into the 0x64000 buffer
                   s32 dtile_x = gx - first;       // tile delta vs camera
                   s32 dtile_y = gy - first; }
   ```
   Screen geometry: classic 2:1 iso — x steps +32 along gx and −32
   along gy, y steps +16 along both; grid origin (0x130, −0x100). The
   sticky `first` is the gx anchor of the FIRST in-bounds cell of the
   whole gy-major scan — (gx,gy) = (12, 4) ⇒ first = 17, so tile
   deltas are (gx−17, gy−17) and the anchor tile (17, 17) sits at
   buffer (0x130, 0x120). The ZONEA cache holds 467 entries (dtile_y
   −13..=18; the anchored row dtile_y = 0 spans dtile_x −9..=9).
2. **TOT → type DB mirror**: zero 0x4ab50 bytes at 0x4796bc, then for
   every map tile (y,x), for z in 0..8:
   - if `TOT_plane_word != 0`: `wordDB[tile][z] = word`;
   - if additionally `DAT_plane_byte == 0`: `seen[tile][z] = 1` —
     marks stack levels with no DAT volume (deck vs solid fill).
   TOT word = `TOT[x*2 + y*w*2 + z*w*h*2]` (header-skipped base,
   STANDARD w*h u16 plane stride — the decompiler's `w*0x1e` TOT
   stride is an artifact; asm 0x407fd6 `ADD EBX,[w*h*2]` is
   authoritative). DAT byte = `DAT[x + y*w + z*w*h]` (header-skipped).
   The 0x1e-stride (30 B/tile) type-DB record layout, one record per
   tile: `+0x00..+0x0f` 8 u16 words (z 0..7; the TOT mirror, later
   overwritten with the resolved sprite id), `+0x10..+0x17` 8 seen
   bytes (0x4796cc = 0x4796bc+0x10 — the seen array is INSIDE the
   same records, byte record+0x10+z), `+0x18` static frame byte,
   `+0x1a` height-bias byte, `+0x1b/+0x1c` anim-window lo/hi
   [consumers verified in FUN_00403938; producers of +0x18/+0x1a/
   +0x1b/+0x1c NOT yet found — the zero-fill leaves them 0 on ZONEA,
   so shipped A-zone renders use static frame 0; open item §8.1].
   UPDATE 2026-08-22 (RE-EXW-SIM §7j.32): the full record grammar is
   unified there — `+0x18` = scorch (producer CLOSED 7j.8/7j.9
   FUN_00422287), `+0x19` = type-DB variant<<4 and `+0x1a` = door
   byte bit7 (producers = the 7j.12 stamper FUN_00422fd1 @0x4796d5/
   0x4796d6), and **`+0x1b/+0x1c` = the OBJECT-HEIGHT pair (z0,
   z0+D)** — the "anim-window lo/hi" producers ARE the
   objective-building stamps (FUN_0044889a @0x448963/0x448975 writes
   z0 and z0+D over each objective footprint; FUN_00448b80 CLEARS
   both on destroy) — the §2 renderer window test below is the
   intact-vs-rubble layer gate flipped by destruction; `+0x1d` has
   zero .text traffic. The §8.1 producer hunt for +0x1b/+0x1c is
   CLOSED (the +0x1a "height-bias" producer note stands as the
   door-stamper cross-ref).
   UPDATE 2026-08-22 (RE-EXW-SIM §7j.34): the tail semantics
   are now COMPLETE — `+0x19` = the door/scenery TARGET-TAG
   byte (the animator runs the frame counter until low7 of
   +0x1A equals it) and `+0x1a` = {bit7 door PHASE, bits0-6
   running FRAME COUNTER}: the 15-frame sliding-door machine
   FUN_00423081 (MissionShell epilogue tick @0x44808f) writes
   DAT-volume door-frame bytes 0x40..0x5E per tick and, on
   wrap, shifts the tile z-stack (DROP on open-complete /
   PUSH-UP on close-complete — FUN_004235fb/00423740); the
   renderer gives mid-anim door tiles a −nibble·0x500 Y-bias
   (0x406c5c). `+0x1d` zero traffic CONFIRMED (71-site
   absolute census). §8.1 fully CLOSED below.

## 3. FUN_00403938 — the per-frame viewport renderer [verified core loop]

Camera: `_DAT_004edde4/8` are Q5 pixel cams; camTileX/Y =
`>>5` stored to `_DAT_004ddb24/28` (§7 note: Q5 0x20 = 1 tile).
Shake `local_17c/180` (funnel from 0x4c71c4 table) adds `shake*0x280`
to every cached dest and `shake` to every sprite row. The terrain loop:

```
for i in 0..cache_count:                        // 0x4067a6..
  dest = cache[i].buf_off + shake*0x280
  tx = cache[i].dtile_x + camTileX; ty = cache[i].dtile_y + camTileY
  if tx/ty out of map:                           // off-map edge tiles
     sprite = FUN_00408030(zone): z1/2/4/7→rand9()+0x37,
             z3→rand9()+0x23e, z5→rand9()+0x65, z6→0x2ec, z0→1
     blit (BIN, sprite) at dest
  else:
     rec = tile*0x1e at 0x4796bc; bucket = (tx-camTileX+9)*4
             + (ty-camTileY+9)*0x90            // sprite-list cell
     bias = rec[0x1a]: &0x7f==0→0; bit7 clear→(v&0xf)*0x500; set→negative
     cursor = 0
     for layer in 0..8:                          // z levels bottom-up
        if cursor == layer:
           if word[layer] != 0 and 0x4ede18 <= dest < 0x4ede18+0x59b00:
              sprite = LNK[word[layer]]; word[layer] = sprite   // ANIM STEP
              frame = (layer < rec[0x1b] || layer >= rec[0x1c])
                      ? rec[0x18]                // static = scorch byte
                                                 // as ramp index
                      : u32[0x456ca8 + (g_frame_count&0xf)*4]
                                                 // anim seq = STATIC
                                                 // ping-pong const
                                                 // {0..7,7..0} §8.2
              FUN_00401471(BIN, sprite, remap=u32[0x4dd444 + frame*4])
           cursor++
           // chase: consecutive seen levels above draw the SAME-side
           // column at dest-0x5000+bias while seen && word != 0:
           while cursor < 8 && seen[cursor] && word[cursor]:
              d2 = dest - 0x5000 + bias
              sprite2 = LNK[word[cursor]]; word[cursor] = sprite2
              if water-zone sprite range (0x454aac[set@0x4edd8c]..+0x1e)
                 && water enabled (_DAT_004edbd4 — ≡ 1 in every
                 mission, §8.2):
                 FUN_0040167a(BIN, sprite2)     // u8-RLE + TXPAL1 remap
              else FUN_00401471(BIN, sprite2, ...)
              cursor++
        flush sprite-list bucket[layer]         // FUN_0040179b per node
        dest -= 0x5000                          // one z level = 32 rows
```

The dest bound `0x59b00` (=row 0x230) + up to 7×0x5000 keeps blits
inside the 0x64000 buffer (cache offsets reach 0x83000+ at low z; the
top screen rows only draw at high z).

Sprite-list flush FUN_0040179b(BIN, frame, mode) u16-RLE blit with mode
(= the queued record's byte at +0x18): 0x130 → paint 0xFF mask;
0x12d → remap sprite byte via DARKPAL(0x4edc00)-relative add;
0x12e → remap via TXPAL1(0x4edbfc); 0x12f → in-place DARKPAL[dest];
else (300/0x12c) plain copy. (Entity/overlay path; not needed for the
terrain crop but decoded while present.)

## 4. FUN_00401471 — terrain tile blit [verified, asm-authoritative]

`FUN_00401471(EAX=sprite_id, EDX=scratch, EBX=remap_table|0)`, ESI =
bank (BIN), EDI = dest ptr in the 0x64000 buffer.

- Directory (CORRECTED 2026-08-22, §7j.36 — asm 0x401477..0x401485):
  `entry = bank + 4·id + 2; sprite = entry + u32[entry]` (the offset
  is SELF-relative, exactly §5c's form; the old gloss
  "bank + u32[bank + 4 + id*4]" was wrong in both base and anchor;
  u16[bank+0] = the sprite count, never read by the blits).
- Record prefix: compressed/nonzero formats use the 10-B logical prefix
  `{u16 fmt,u16 dy,u16 dx,u16 gate,u16 rows}` before the stream; gate 0
  returns. The tested seven runtime-selected lettered BIN banks contain
  formats 0 and 7 only. Each has nine fmt-0 radar stubs physically encoded
  as the 6-B `{u16 fmt=0,u16 dy=64,u16 dx=64}` plus 4096 raw 64×64 bytes
  (4102 B total). The blitter's unconditional gate/rows reads therefore
  observe the first four zero raw pixels and return without drawing. This
  seven-bank census does not establish whether fmt 1..3 occur in other BIN
  banks. Dest = EDI + dy*0x280 + dx (stride 640).
- **fmt 0**: raw 64×64, transparency color 0, row stride 640
  (`ADD EDI,0x240` after 64 bytes = 640).
- **fmt 1..3**: u16 RLE. Control word (bit15 set): bit14 set = end of
  row (EDI = row_start+0x280), else skip (word & 0xFFF) px; literal
  word (bit15 clear): run of (word & 0xFFF) bytes, bit14 = end of row
  after the run. Row budget tracked in EDX (0x280 minus consumed).
- **fmt ≥ 4**: u8 RLE. Control byte (bit7 set): bit6 = end of row,
  else skip (b & 0x3F)+1 px; literal byte: copy (b & 0x3F)+1 bytes,
  bit6 = end of row after the run.
- **Remap path** (EBX≠0 && _DAT_004edbd4≠0): literal bytes go through
  XLAT (remap_table[byte]); skips unchanged. This is the water/dark
  palette effect; per-frame remap tables come from `u32[0x4dd444 +
  frame*4]`.

FUN_0040167a (water variant): same head but gate is READ AND IGNORED
(0x4016ad — no test; rows 0 still draws nothing, §7j.36), forces u8-RLE
decode, each literal byte written as TXPAL1-relative lookup (bank
0x4edbfc) [secondary; ZONEA unaffected — the water sprite family stages
ZERO cells in ZONEA/M1, §8.2].

## 5. FUN_0040798e — sprite-list enqueue (entities/overlays) [verified, asm-anchored]

Per-frame lifecycle [asm 0x403950..0x403962]: the bucket head array at
0x46cdbc is ZERO-CLEARED every frame (`FUN_00402965` with ECX=0xa200
= 36*36*8 ptrs, EDI=0x46cdbc) and the arena cursor resets
(`DAT_0046cc04 = _DAT_004edd50`) BEFORE the entity loops run — so a
fresh list per frame is the faithful model.

`FUN_0040798e(sx, sy, bank, wx, wy, frame, layer, mode)` (register
convention: EAX/EDX/EBX/ECX + 4 stack args):

- `dest = sx + sy*0x280` (byte offset rel. buffer 0x4ede18);
- `bx = (wx >> 5) - camTileX + 9`, `by = (wy >> 5) - camTileY + 9`
  (camTile from `_DAT_004ddb24/28`, set per frame); `bx/by < 0` →
  RETURN (not drawn). No upper clip — the caller's sx/sy clip bounds
  it (both clips ⇒ dx ≤ 0x1c8 ⇒ bx ≤ 23 < 36);
- `layer = clamp(layer, 0..7)`; `sort = wx + wy` (the +0xb-adjusted
  pixel coords the entity loop passes);
- bucket = head ptr at `0x46cdbc + bx*4 + by*0x90 + layer*0x1440`;
  48-B node `{+0 dest, +4 bank, +8 frame, +c layer, +10 wy, +14 wx,
  +18 mode, +1c sort, +20 next, +24 sx}` (arena cursor advances 40 B
  in the empty-bucket path — the +24 word only in the copy-insert
  path; node identity is not observable, list ORDER is);
- insertion keeps the list ASCENDING by sort, STABLE after equals
  (walk while `next != 0 && cur.sort <= new.sort`); head-insert when
  `new.sort < head.sort`; insert-before via duplicate-forward of the
  successor node. Entities enqueue BEFORE the terrain pass; the
  terrain loop (§3) flushes each visited bucket per layer, so sprites
  interleave with terrain in painter order (§5b).

### 5b. Flush site in the terrain loop [verified]

Per cache cell, per layer, AFTER the blit + seen-chase: gate
`0 <= bx < 0x24 && 0 <= by < 0x24` (bx/by = cell tile − camTile + 9,
same index form as the enqueue), then walk the bucket list via
`next` (+0x20) and call `FUN_0040179b(node.frame, node.frame,
node.mode)` with ESI = node.bank, EDI = 0x4ede18 + node.dest. Buckets
whose tile has no cache cell are never flushed (the EXW never draws
them either — same observable behavior).

### 5c. FUN_0040179b — the flush blit [verified, asm-authoritative]

Directory identical to §4 (`id & 0xFFF` → entry `2 + 4*id` → sprite
at `entry + u32[entry]`), but the header read starts +2 past the §4
sprite start (the fmt word is SKIPPED): dy, dx, gate (skipped), rows.
The decode is ALWAYS the u16-RLE family of §4 regardless of fmt, and
— unlike the terrain blit — **literal runs copy RAW bytes with NO
zero-skip** (REP MOVSB): transparency exists only as RLE skip words.

Mode dispatch (mode = node +0x18):
- `0x130` → paint every literal-run byte as 0xFF (STOSB 0xFF);
- water flag `_DAT_004edbd4 == 0` OR mode `0x12c` (= decimal 300) →
  plain raw copy [the flag==0 arm is DEAD CODE in shipped play —
  the flag is 1 for every mission, §8.2];
- water on: `0x12d` → `dest = TXPAL1[(dest<<8) | b]` and `0x12e` →
  `dest = TXPAL1[(b<<8) | dest]` — TXPAL1 at 0x4edbfc is a 64-KiB
  two-level composition table [asm: `MOV AH,[ESI]; MOV AL,[EDI]` /
  swapped]; `0x12f` → `if b != 0: dest = DARKPAL[dest]` (256-B XLAT
  at 0x4edc00 — the ONLY zero-gated mode); any other mode → RET
  (nothing drawn).

### 5d. The robot entity loop of FUN_00403938 [verified, decomp 0x4039fa..0x4067a0]

Runs over the robot array at 0x4c69e4 (stride 0xa8, count
`DAT_0046ccbc`) BEFORE the terrain pass, per frame. Field map below
uses record offsets from 0x4c69e4 (the SIM doc §3's row labels are
anchor-address-authoritative; its "+0x14" countdown row is actually
u16@+0x16 — see the correction note):

```
wx_px = pos_x(Q13) >> 8;  wy_px = pos_y >> 8     // 32 px per tile
dx = wx_px - camQ5X;      dy = wy_px - camQ5Y
sx = ((camQ5X&31) - (camQ5Y&31) + 0x20) & 0x3f + (dx - dy) + 0x110
sy = shakeY + ((dx + dy) >> 1) + 0x10c + ((camQ5X&31)+(camQ5Y&31))>>1 - z
clip: 0 <= sx < 0x23f && 0 <= sy < 0x23f && i32@+0x7c (alive) != 0
```

`z` = i32@+0x08 (Q5 px, raw in sy); enqueue layer = `z >> 5`. Per
visible+alive robot, in order (all enqueues pass `wx_px+0xb`,
`wy_px+0xb`):

1. **teleport beam** (state u16@+0x0c ∈ {5,6}): bank DAT_0046af38 =
   `GAMEGFX\TELEPORT.BIN` (10 imgs) at `sy - 0x48`, mode 0x12e, frame =
   clamp(10 − wobble/4, 0..9) with wobble = i32@+0x90. [CORRECTED
   2026-08-23, §7j.48/D120: this draw was mislabeled "shield" — cell
   0x46af38 is TELEPORT per the 7j.28/7j.30 corpus-string map, and the
   0..9 clamp matches its 10 images; states 5/6 are the beam phases];
2. **body** DANTE (unless hidden): bank `_DAT_004ede2c` =
   `GAMEGFX\DANTE.BIN` [LoadFile @0x41e02e, ArenaAlloc 85000], frame
   = u16@+0x12 (the walk anim phase), mode 300. Hidden when
   `state==2 && i32[0x4dcdd4 + i32@+0x84*0x24] > 0xf`,
   `state==5 && wobble > 0xf`, or `state==6`;
3. **shield sprite** (i32@+0x88 != 0): bank DAT_0046af44 =
   `GAMEGFX\SHIELD.BIN` (4 imgs), frame = u16@+0x18, mode 300.
   [CORRECTED 2026-08-23, §7j.48/D120: was mislabeled "variant
   sprite" — cell 0x46af44 is SHIELD; the 4 frames match spawn's
   `u16@+0x18 := RandA()&3` and the post-loop +1 &3 shimmer cycle];
4. **animated overlay** (u16@+0x16 != 0xFFFF): DANTE, frame =
   u16@+0x14 * 3 + g_frame_count%3 + 0x40;
5. **always**: DANTE, frame = u16@+0x14 + 0x20, mode 300.

Spawn (FUN_0040cca0, decomp @0x41dc5a family) zero-fills the record
then sets facing = 0xFFFF, u16@+0x16 = 0xFFFF, i32@+0x88 = 0,
u16@+0x18 = RandA()&3, pos = tile*0x2000+0xF00, z = level*0x20−1 —
so **a spawned robot draws exactly two sprites: DANTE[anim] and
DANTE[0x20]** (u16@+0x14 stays 0). After the loop the low 2 bits of
u16@+0x18 cycle +1 &3 (only observable through the +0x88-gated
sprite). `_DAT_004edb88 != 0` (any MP mode; SP never) additionally
queues TINYFONT (DAT_0046cdb0) name-plate glyphs at
`sx + u32[0x4e44c8 + id*4] + 6*i` for stored glyph bytes ≤ 0x40
(the id-indexed CENTERING table, NOT per-char — corrected 2026-08-23).
The full name-plate grammar + the SHIELD/TELEPORT/ROBNUMS staging
story is DECODED 2026-08-23 (§7j.48/D120): all three banks are
allocated + loaded at EVERY MissionShell head (FUN_0044771c →
FUN_0041d954 alloc @0x447860 + FUN_0041df10 LoadFile @0x447b3f,
straight-line, SP included — no mission/MP gate), ROBNUMS.BIN
(0x46af48, 9 imgs) is loaded but has ZERO game readers — dead data,
the plates draw TINYFONT glyphs — and the enqueue/flush pair has NO
unstaged-skip anywhere (the Backlog "flush skips while unstaged"
clause is retired). Platform
(0x4eb638) and effects (0x4cf638, the FUN_00401e39 draw_IMG codec
family) loops follow the same sx/sy form — DECODED 2026-08-21
(7j.26), see §5e/§5f.

### 5e. The render-tail direct-draw passes [verified 2026-08-21, 7j.26 — decomp+asm 0x4067a1..0x4071d4]

After the robot entity loop (§5d), FUN_00403938 runs FOUR more
entity passes that all draw through the DIRECT blit FUN_00401e39
(§5f) instead of the enqueue path — no z-layer nodes, no palette
modes, straight into the 640×640 backbuffer `_DAT_004ede18`.

**The effects loop — bank 0x4cf638, 80 × 0x1E [verified asm
0x406c86..0x406d60; mover FUN_00419f62; producer FUN_0041a225]**
(called from MissionShell tick @0x44813d):

```
for each record (stride 0x1E):
  if (u16@+0x18 != 0 && u16@+0x1A == 0):     // active ∧ not delayed
    dx = (i32@+0x00 >> 8) - camQ5X;  dy = (i32@+0x04 >> 8) - camQ5Y
    sy = shake2 + ((dx+dy) >> 1) + 0x100 + camRowAdj - (i32@+0x08 >> 8)
    sx = camColAdj + (dx - dy) + 0x110
    u16@+0x1C++                                // frame counter (in the DRAW)
    if (0 <= sx < 0x23f && 0 <= sy < 0x23e):
      FUN_00401e39(u16@+0x16 * 8 + (u16@+0x1C & 7), 1, sx, sy)
        with ESI = [0x4eddb4] = DEBRIS.BIN, EDI = [0x4ede18]
```

Two deltas vs the §5d robot form: the y base is **0x100** (−0xC =
12 px higher than robots/platforms) and the shake channel is the
SECOND table (`[0x454518 + quake_idx*4]` = local_17c; robots and
platforms use `[0x45450c + …]` = local_180 — two independent
quake ramps indexed by the same countdown `DAT_0046cce4`).
z@+0x08 is Q13 like x/y (px = z >> 8), unlike the Q5-raw robot/platform z.

Full record map (producer FUN_0041a225 + mover FUN_00419f62 +
allocator FUN_0041a4cc, all verified this unit):

```
+0x00/+0x04/+0x08  x/y/z Q13 (producer: ((tile<<5)+RandB&0x1F)<<8 - 0x1000,
                    z = level<<13 + 0xF00)
+0x0C/+0x10         vx/vy Q13/frame ((RandB&0x3F)<<7 - 0x1000)
+0x14 dword         vz = (RandB&0x7FF) + 0x1770  — RISING, 6000..12069;
                    its high word u16@+0x16 (0..2) IS the sprite group
+0x16 u16           DEBRIS.BIN image group (drawn img = group*8 + frame&7,
                    i.e. images 0..23; set implicitly by the vz write)
+0x18 u16           ACTIVE gate (0 = free slot; first-fit allocator scans
                    80 slots for ==0). Producer writes FUN_0041ec59(3)
                    ∈ {0,1,2} → ~8% of particles are STILLBORN (slot
                    occupied until the next alloc scan reuses it)
+0x1A u16           spawn DELAY countdown (producer copies its 4th
                    register arg ECX here; mover decrements while != 0;
                    no physics, no draw while delayed)
+0x1C u16           frame counter (producer seeds RandB&7; draw ++)
```

Mover FUN_00419f62 (per tick, only when +0x18≠0): if delayed,
`+0x1A--`; else `x += vx; y += vy; z += vz`, and the record is
KILLED (+0x18 := 0) when x<0 ∨ y<0 ∨ z<0 ∨ x>>13 ≥ [0x4eddec]
(map W) ∨ y>>13 ≥ [0x4eddf0] (map H) ∨ z>>13 > 0xB. With
vz ≥ 6000 the particles die at the z=12 ceiling in ~8..16 ticks —
a rising spark burst (the 7j.25 "ttl 6000+" gloss was this vz).
**FUN_0041ec59(3) identity pinned**: `RandB() / (0x8000/n − 1)`
clamped to n−1 — a bounded-uniform random helper; here it only
arms the active word (the 1-vs-2 value is never read).

**The platform loop — bank 0x4eb638, 32 × 0x14 [verified decomp
0x4067a1..0x406832; tick FUN_004238af; producer FUN_0042382c]**
(called from MissionShell tick @0x447fff):

```
for each record (stride 0x14):
  if (i32@+0x0C != 0):                        // claim/age gate
    dx = (i32@+0x00 >> 8) - camQ5X; dy = (i32@+0x04 >> 8) - camQ5Y
    sy = shake1 + ((dx+dy) >> 1) + 0x10c + camRowAdj - i32@+0x08   // z raw Q5
    sx = camColAdj + (dx - dy) + 0x110
    if (0 <= sx < 0x23f && 0 <= sy < 0x23f):
      layer = i32@+0x08 >> 5
      FUN_0040798e(sx, sy, DAT_0046af54, dxpx+0xb, dypy+0xb, 0, layer, 300)
      if (sy - 0x20 >= 0):
        FUN_0040798e(sx, sy-0x20, DAT_0046af54, dxpx+0xb, dypy+0xb,
                     i32@+0x10 + 1, layer, 0x12d)
```

Identical sx/sy form to the §5d robot loop (z@+0x08 raw in sy,
>>5 for the enqueue layer). **DAT_0046af54 = `GAMEGFX\SMOKER.BIN`**
(stager FUN_0041df10 @0x41dfb1 LoadFile) — the "platform" records
are the robot-death BLAST/smoke columns of 7j.24: base sprite =
SMOKER image 0 (mode 300), smoke column = SMOKER image
`i32@+0x10 + 1` (mode 0x12d = the DARKPAL flush) at sy−0x20.
The anim tick FUN_004238af cycles the frame word: `+0x10 = +0x10+1;
if (+0x10 == 0x10) +0x10 = 4` — from the producer's 0 the drawn
column runs 2,3,…,16 then loops 5…16 forever (record stays claimed;
slot reuse is the allocator's MIN-age pick). The enqueue args
`dxpx+0xb/dypy+0xb` mirror the robot loop's wx+0xb hotspot.

**Adjacent context (same asm block, decoded for geography): three
DROPSHIP ring passes.** The per-robot ring bank at 0x4e64c0 (12
slots implied by the 0x4e6610 boundary; loop bound = robot count
`DAT_0046ccbc`), the 6 standalone rings 0x4e6610 (drawn singly) +
0x4e662c..0x4e66b8 (5 × 0x1C), all 0x1C records
{active d@+0, x d@+8, y d@+0xC, z/alt d@+0x10, img-group d@+0x14}:
when active, a **7-column × 5-row grid of 0x40-stride tiles**
(448×320 px — 7j.27 correction of this pass's first "7×7" gloss;
0x23 = 35 = 7·5 images = exactly one DROPSHIP.BIN group) is drawn
via FUN_00401e39 with `img = group*0x23 + 7*row + col`, bank
**ESI = [0x4edd64] = `GAMEGFX\DROPSHIP.BIN`** (ArenaAlloc(0x25990)
loader @0x41c8xx family, exw-simtail 1752) — the dropship hull
during the 7j.20 mission-start pod descent. Robot-indexed sy also
subtracts the robot's own z (d@+0x08 of the 0x4c69e4 record); the
sx/sy bases are 0x90/0xd0 (not 0x110/0x10c) — the grid is 448 px
wide, centered differently. The trail-ring bank 0x4e66b8 (7j.22/23)
begins exactly at the end of the 6 standalone records. Producers
of the ring records = the pod-descent family — **CLOSED §7j.27**
(writer census complete: resets FUN_0040cca0 0x40cd3d +
MissionShell 0x447a7e/0x447a8d; spawners FUN_0041fa51/FUN_0041faf0/
FUN_0041fb4b; per-tick animator FUN_0041fbb1 with +0x14 = the
img-group selector toggling 0↔1 in phases 1-2 and ramping 2..5 in
departure; + the 0x412b60 POI-rescue exit-dwell reset).

**Bonus pin: the [0x4ede24/0x4ede28] "7×7 screen-address table"
of the backlog is NOT a 7×7 table — it is the terrain RESTAMP
list** [verified decomp 0x406a8c..0x406c73 region, readers
0x4067a6/0x406b32]: `_DAT_004ede28` = record count,
`_DAT_004ede24` = pointer to 3-dword records {dest row offset
(y·0x280 basis), tile-x, tile-y}; per record the pass blits via
the §4 terrain codec FUN_00401471 — tiles outside the map window
edges get the FUN_00408030 border tile, in-window tiles go
through the full LNK/type-DB terrain path (continuing into the
DAT_004796bc/LNK-image code). Writers per the 7j.16 census:
FUN_00440a2d (the TOT-mirror materializer), FUN_0043d00b,
FUN_0041d954 — so the materializer IS the scroll/camera
restamp stager, confirming the backlog hypothesis (UPDATE
§7j.49 2026-08-23: CORRECTED — the [0x4ede24] cell is a
PER-SCREEN reuse; FUN_00440a2d stages the BRIEF objective-
minimap list only; the in-game producer of the render-tail
list is FUN_0041d954's viewport-cache install). A separate
state-machine pass over 0x4c71f4 (states <0x13; the splash/
screen-effect sequences) sits between the platform and effects
loops — head-decoded §7j.27: it is the projectile mid-flight
draw dispatch (type word@+0 switch → shell/artillery/mortar/
damped/rocket/homing draw bodies) + the sibling 0x4cc654
50×0x22 bank draw (states 0x65..0x69, jump table 0x403908);
full per-type math still open (with the trail-ring draw
consumer @0x404464).

### 5f. FUN_00401e39 — the direct draw_IMG blit [verified, decomp+asm 0x401e39..0x401f83; 8street `draw_IMG_in_buffer` re-anchored]

Register args (EAX img, DX transp, EBX x, ECX y, ESI bank, EDI
dest). Same on-disk .BIN container as the enqueue path (§5/§6)
but with the layout now corpus-verified [7j.26]: **u16 image
count at word0, then count × int32 directory at `bank + 2 +
4*img`, each offset RELATIVE TO ITS OWN SLOT** (asm 0x401e40:
`add eax,eax; inc eax; add eax,eax` = 4·img+2; image data at
`&dirslot + *dirslot`; verified 24/24 DEBRIS + 160/160 DANTE
images parse + every RLE stream consumes exactly to the next
image). The DIFFERENCE vs the enqueue path is the consumer: no
layer buckets, no palette-flush modes, dest = EDI + y*0x280 + x
with row advance 0x280 (the 640-stride backbuffer), and the
second arg is a plain 0/≠0 flag:

```
img hdr: u16 flags; if (flags & 2) { y += s16 word1; x += s16 word2; }
         u16 w, u16 h
flags & 1 = RLE coded, else raw
RLE: per row, u16 control words until bit14 (EOL):
     bit15 set → run = word & 0xFFF:  transp≠0: skip run bytes
                                          transp==0: paint run ZERO bytes
     else      → literal: copy run raw bytes (NO per-byte zero test)
raw: transp==0 → copy w×h plain;  transp≠0 → per-byte zero-skip copy
```

So byte-granular transparency exists ONLY as RLE skip words in
coded images (same rule as the §5 flush codec) and ONLY as
per-byte zero-skip in uncoded ones; the opaque path paints
palette-0 in skip runs. Hotspot words are (yoff, xoff) in that
order. Callers: the four render-tail passes (0x406d56/0x406eee/
0x407077/0x4071ce), the map overlay FUN_004089b1 (TABLE.BIN —
7d/7e), the boot/attract + title/menu family (0x41c8xx, 0x43d5xx+),
FUN_00401ca2 etc. — the game's general-purpose UI/direct blitter.

## 6. Sprite banks staged by FUN_0041df10 [verified] (context)

DANTE/SCANNER/BLOWUP(/BLOWUPG)/WEAPONS/SHRIKE/REAPER/SMOKE/TELEPORT/
NUMBERS/FLAGS/VICERA/DEBRIS/SHIELD/ROBNUMS .BIN + TABLE.BIN +
DIGITS/SMOKER/HUMANS/IDIOTGFX + palettes TXPAL1/GAMEPAL/DARKPAL —
entity/overlay banks + game palette (GAMEPAL 0x4edbf8; 7c's 0x302-B
copy target). Bank pins refined 7j.26: **DAT_0046af54 = SMOKER.BIN**
(the §5e platform/blast smoke loop), **_DAT_004eddb4 = DEBRIS.BIN**
(the §5e effects-particle loop), and **[0x4edd64] = DROPSHIP.BIN**
(ArenaAlloc(0x25990), NOT staged here — own loader in the mission
init family; the §5e ring passes). TABLE.BIN identity pinned 2026-08-21 (RE-EXW-SIM 7d):
a draw_IMG-family bank whose image 0 is the strategic-map backdrop —
sole reader FUN_004089b1 (the map overlay); the per-tile map colors
are LNK-image words (0x45cdda + 2*type — the §7e correction of the
"0x45cdd8 table" gloss) feeding .MIN 4×4 masks through the MAPTRAN
ramps. The shared backbuffer [0x4ede18] = ArenaAlloc(0x64000)
(640×640); init_tiles clears all 0x64000 bytes each mission start,
the overlay draw clears the top 0x4b000 (640×480) per frame; the
terrain pass overwrites everything the present window reads.

## 7. Present: FUN_00401107 → FUN_00401010/FUN_004012f7 [verified]

- Buffer 0x64000 (stride 640). Viewport window: base
  `buf + 0xa040` (row 64, col 64) + fine-cam offset:
  `colAdj = ((camX&0x1f) - (camY&0x1f) + 0x20) & 0x3f`,
  `rowAdj = ((camX&0x1f) + (camY&0x1f))>>1` (0x40-multiple rows only
  affect the scaled path). Camera 0 → (col 96, row 64).
- 1:1 path (FUN_00401010 `0x4edba0==0`, full 0x1e0=480 height): copy
  0x1e0 rows × 0x78 dwords (480 px) from the window to the locked
  surface, source stride 640 (`ADD ESI,0xa0`), dest stride
  `_DAT_004edb40`. So the mission viewport presents a **480×480
  window** of the 640-wide buffer (letterboxed by the platform).
- Zoom path (`0x4ede34 != 0`, FUN_004012f7/FUN_004013e8):
  Bresenham-style row/column replication from 0x1e0-source-height
  scale factor `_DAT_004ede54` (heights 0x1e0/0x1df); not needed at
  camera 0/full zoom for the crop gate. [2026-08-23 precision,
  §7j.58/D130: this is the death-wipe IRIS path — FUN_004012f7
  fill-0 + centered v×v SHRINK (row routine 0x401430, scale
  (0x1E0<<16)/v) of the FROZEN frame, v := 480−min([0x4ede34],479);
  FUN_004013e8 is the NORMAL zoom path's STRETCH row routine
  (480←v bytes, scale (v<<16)/480) — the two row routines are
  inverse twins, not one path]

## 8. Open items

1. Producers of type-DB bytes +0x18/+0x1a/+0x1b/+0x1c (static frame,
   height bias, anim window). Zero-filled on ZONEA → no effect on the
   P4 corpus gate. **FULLY CLOSED 2026-08-22** (7j.8/7j.9/7j.10/
   7j.12/7j.32/7j.34 — the §7j.34 unit completed the census and
   unified the semantics; the complete traffic map, all 71 absolute
   sites): `+0x18` scorch — writers FUN_00422287 (absolute, clamp<8,
   the debris scorch rings) + FUN_0042223c (increment, clamp 7,
   platform damage/build) + the §7j.10 decay tick FUN_00424051
   (−1/frame; also reads 0x424088/96) + the scorch→damage reader
   0x40bc60 (FUN_0040b9f6: state-1 robot on scorched ground →
   FUN_004100b7(robot,0x14) fire damage, else pod countdown −= 10);
   `+0x19` TARGET-TAG byte — writers FUN_00422fd1 rect stamper at
   load (variant<<4, STATE@+0 ≥ 3 rects; the 7j.12 "word@+2" field
   citation corrected by §7j.34) + FUN_004235e4/004235bf (the door
   stepper's re-stamp); readers 0x406bd6/0x406bf9 (renderer
   door-strip adjacency incl. the north neighbor), 0x4110cb
   (FUN_00410823 fire anchor @robot anim word 0x4c720e), 0x418735
   (FUN_004186fc standing-on-scenery), 0x4237c5/0x4237da (the
   close-completion S/E-neighbor door test inside FUN_00423740);
   `+0x1a` {bit7 phase, low7 frame counter} — writers FUN_00422fd1
   (0/0x80 at load) + the animator FUN_00423081 low7++ (the
   MissionShell epilogue tick @0x44808f; state≥3 rects auto-cycle
   with countdown@+0xC/SFX@+0xE, states 1/2 are script-toggled via
   FUN_004223b8) + FUN_004235e4/004235bf (bit7 on toggle); the
   animator writes DAT door-frame bytes 0x40+2n (closing, even) /
   0x5F−2n (opening, odd) at the walk-down level and the finish
   pairs shift the tile z-stack (FUN_004235fb drop / FUN_00423740
   push-up); readers 0x422499/0x422529 (the stepper's anim-complete
   gate low7==+0x19), the animator's own 0x423165+ family, and
   0x406c3b/0x406c5c (renderer: mid-anim door tiles draw with
   −nibble·0x500 Y-bias — the door slide);
   `+0x1b/+0x1c` OBJECT-HEIGHT pair — writers FUN_0044889a ×2 walks
   (0x448963/75 + 0x448b4f/61) / FUN_00448b80 ×2 clears (0x448c25/2c
   + 0x448d65/6c); readers the intact-vs-rubble draw pick
   0x406891/0x4068ec/0x406907; `+0x1d` zero traffic CONFIRMED
   (padding). The 0x4dcae8 rect grammar is RESOLVED (§7j.34):
   {+0 state, +2 x0, +4 y0, +6 w, +8 h, +0xA variant, +0xC
   countdown, +0xE SFX-due} — the 7j.21 w/y/h permutation retired.
2. `u32[0x4dd444]` remap-table set + `u32[0x456ca8]` 16-entry anim
   sequence: **PARTIALLY CLOSED 2026-08-21 (RE-EXW-SIM §7e)** —
   u32[0x4dd444+4i] are the 8 PALTRAN ramp pointers (loader
   FUN_0042209b, slot 0 NULLed after load); u32[0x4dd464+4i] the 8
   MAPTRAN ramp pointers (FUN_00422171). **FULLY CLOSED 2026-08-22
   (RE-EXW-SIM §7j.35)**: the `u32[0x456ca8]` "producer" is the
   FILE IMAGE itself — a STATIC DGROUP const
   `{0,1,2,3,4,5,6,7, 7,6,5,4,3,2,1,0}` (16-phase ping-pong over
   the 8 ramp slots; zero .text writers, two readers 0x40691a/
   0x406a2c). The STATIC branch indexes the same ramps by the
   +0x18 SCORCH byte (scorch n → ramp n — the ramps double as
   scorch darkening); the anim-window branch (+0x1b/+0x1c, §8.1)
   is ZONEG-only. The WATER FLAG producer is closed the same
   unit: `_DAT_004edbd4` ≡ 1 for every mission (sole persistent
   writer = the campaign-boot defaults FUN_004252c0 @0x4252d8;
   one scoped save/restore around the SELECTOR screen
   FUN_0043e7d4 — no config/options/save/MP writer), so the
   0x12d/0x12e/0x12f "water-off" plain-copy arms are dead code
   in shipped play and E may hard-code water-ON. Corpus: water
   sprite words stage ONLY in ZONEB/M1 (12 cells), ZONEB/M6 (78),
   ZONEC/M4 (33), ZONED/M1 (1), ZONEF/M7 (4824) — ZONEA/M1 ZERO;
   the water-leg P4.2 hooks are in §7j.35 item 5.
3. ~~FUN_00403938's entity loops~~ **CLOSED 2026-08-21**: the robot
   entity loop + FUN_0040798e/0179b enqueue/flush are decoded (§5b–§5d)
   and wired into bedlam-render. **FULLY CLOSED 2026-08-21 (7j.26)**:
   the platform loop (0x4eb638 → SMOKER.BIN blast/smoke columns, §5e),
   the effects loop (0x4cf638 → DEBRIS.BIN particles via the
   FUN_00401e39 direct codec, §5e/§5f), and the codec itself (§5f).
   Remaining adjacent tail (context §5e): the DROPSHIP ring-record
   PRODUCERS (pod-descent family) — the name-plates half CLOSED
   2026-08-23 (§7j.48/D120: TINYFONT glyphs, MP-gated draw, the
   TELEPORT/SHIELD/ROBNUMS banks alloc+load at every MissionShell
   head — SP included, no gate; ROBNUMS itself dead data; no
   unstaged-skip in enqueue/flush); the DROPSHIP producers
   remain as their own backlog item (7j.27 covered the descent tick).
4. BIN u32[bank+0] directory header word / sprite count sanity.
    **RESOLVED 2026-08-21 (7j.26, corpus-verified)**: word0 is a
    **u16 image count** and the int32 directory starts IMMEDIATELY
    at bank+2 (no pad): DEBRIS.BIN count 24 (= exactly the effects
    loop's img range 0..23, ~12×14-px chunks with (y,x) hotspots
    ≈(40,26)), SMOKER.BIN count 17 (= frames 0..16, the blast
    base + 2..16/5..16 column), DROPSHIP.BIN count 210 (64×64
    tiles — matching the 7×7 0x40-stride ring grids; many entries
    are 0×0 empty stubs that the codec skips instantly).

## Off-map zone indexing correction (live original comparison)

[verified, EXW direct bounded disassembly] 0x408030 loads the one-based
zone at 0x4edd8c, then `dec eax` at 0x408035 before the unsigned
0..6 jump-table dispatch. The seven dwords at 0x408014 are
408043,408043,408053,408043,408063,408073,408043. Therefore for
engine zero-based zone indices A..G the bases are respectively
0x37,0x37,0x23e,0x37,0x65,0x2ec,0x37. All except F add RandB(9);
invalid zones fall back to sprite 1 without a random draw.

The previous table's one-based values were passed a zero-based engine
zone. This explains the live native paving outside Boot Camp, compared
to water at the same location in DOSBox. Zone A must consume edge
random draws; comments claiming fixed edges/no draws for zone zero are
wrong. The earlier renderer table is superseded by this correction.
The original full objdump listing was misaligned through the jump table;
starting disassembly explicitly at 0x408030 exposes the missed decrement.
Corpus checksums passed before and after the read-only probe.

## Mission sidebar base art

[verified, EXW + corpus header] MissionShell 0x447c82..0x447c96
draws GENERAL.BIN sprite 1 at x=480,y=0 with transparency parameter
zero. It then renders the scene and flips, looping twice to initialize
both original buffers (0x447ca5..0x447cae). Sprite 1 is RLE, no hotspot,
160x480 in the shipped bank. This is the missing static HUD frame seen
in the live original/native comparison. The native single retained
mission plane needs this base once at activation, before portraits, bars,
weapon rows and score digits. It must not be repainted over those
dynamic widgets every frame. Corpus checksums passed before/after.

## Radar scan lifecycle and compositing

[verified, EXW] MissionShell calls 0x41ec68 at 0x447b53: radius
0x4edd68 := 64 and clear the 128x128 backing image at 0x4eddb8
(0x402482 writes 4096 zero dwords). The per-frame caller is
0x448142 → 0x41ec81. This is a repeating reveal, not a one-time
opening animation as earlier shorthand suggested.

Normal state at radius 64: copy the full OLD backing image to the
screen (0x402504), clear the backing image, compute selected robot
center ((pos_x>>8)+16)>>4, ((pos_y>>8)+16)>>4, set radius zero,
then rebuild marker pixels through 0x41ee20. Other frames increment
radius by four and copy the central 2r by 2r square from the backing
image. For r<64 draw the expanding color-7 rectangular outline through
0x402492. At r=64 omit the outline; the next call copies that full
image again and starts a fresh marker capture. Thus refreshes are
separated by seventeen calls, with the new backing image revealed
starting on the call after capture.

Copy origin is screen (496+64-r,197+64-r), backing
(64-r,64-r), width/height 2r (0x402504..0x40256a). The outline
origin is (496+64-r,195+64-r), top/bottom width 2r, side pixels
x and x+2r for 2r rows; its last row is y+2r-1. Finally draw
SCANNER.BIN sprite 0 at (496,195), transparent (0x41edff..0x41ee16).
The +2 Y difference between copy and outline is in the executable.

Mouse-down inside inclusive x=494..625,y=195..326 latches the
pressed state. While held inside it draws sprite 17 at (494,195)
and skips scanning. Release or leaving the rectangle resets radius
64, clears the backing image, draws sprite 18 at (494,195), clears
the latch, and sets the existing scanner backdrop redraw count to 2
(0x41ec87..0x41ed2c). On the first press, normal scanning still runs
after the latch is set (0x41ed31..0x41ed74).

Marker space is axis-aligned, not the viewport isometric projection:
x=64+object_x-center_x, y=64+object_y-center_y; reject if either
absolute delta is >=128. Robot coordinates are Q5 rounded by +16
then shifted four; tile centers are (tile*32+16)>>4. Marker blitter
0x402572 reads raw SCANNER glyph bytes, applies unsigned stored
hotspot offsets then subtracts 2 from each coordinate, clips to
128x128, and skips source color zero (0x4026cc..0x4026d3). Do not
reuse the screen RLE/transparency path without matching those details.

Scanner equipment level >=1 gates TRT and destructible-strength
markers (0x41ee2b..0x41ee38,0x41f10c..0x41f119). Claim-bank tile
markers have a separate path at 0x41f191. Remaining marker-loop
ordering, icon gates and producer availability still need to be
connected before radar parity can be claimed. No runtime radar code
has been added by this specification pass.
