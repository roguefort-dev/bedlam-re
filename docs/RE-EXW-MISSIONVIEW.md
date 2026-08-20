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
  0x4ede1c before every FUN_00401471 call, 0x40693c/0x406a54/0x406b1f].
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
                      ? rec[0x18]                // static
                      : u32[0x456ca8 + (g_frame_count&0xf)*4]   // anim seq
              FUN_00401471(BIN, sprite, remap=u32[0x4dd444 + frame*4])
           cursor++
           // chase: consecutive seen levels above draw the SAME-side
           // column at dest-0x5000+bias while seen && word != 0:
           while cursor < 8 && seen[cursor] && word[cursor]:
              d2 = dest - 0x5000 + bias
              sprite2 = LNK[word[cursor]]; word[cursor] = sprite2
              if water-zone sprite range (0x454aac[zone]..+0x1e)
                 && water enabled (_DAT_004edbd4):
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

- Directory: `sprite = bank + u32[bank + 4 + id*4]` (u32[bank+0] not
  read here — likely count).
- Header (10 B): `u16 fmt; u16 dy; u16 dx; u16 gate(≠0 else return);
  u16 rows;` then the stream. Dest = EDI + dy*0x280 + dx (stride 640).
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

FUN_0040167a (water variant): same header, forces u8-RLE decode, each
literal byte written as TXPAL1-relative lookup (bank 0x4edbfc)
[secondary; ZONEA unaffected unless the water range hits].

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
  plain raw copy;
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

1. **shield** (state u16@+0x0c ∈ {5,6}): bank DAT_0046af38 at
   `sy - 0x48`, mode 0x12e, frame = clamp(10 − wobble/4, 0..9) with
   wobble = i32@+0x90;
2. **body** DANTE (unless hidden): bank `_DAT_004ede2c` =
   `GAMEGFX\DANTE.BIN` [LoadFile @0x41e02e, ArenaAlloc 85000], frame
   = u16@+0x12 (the walk anim phase), mode 300. Hidden when
   `state==2 && i32[0x4dcdd4 + i32@+0x84*0x24] > 0xf`,
   `state==5 && wobble > 0xf`, or `state==6`;
3. **variant sprite** (i32@+0x88 != 0): bank DAT_0046af44, frame =
   u16@+0x18, mode 300;
4. **animated overlay** (u16@+0x16 != 0xFFFF): DANTE, frame =
   u16@+0x14 * 3 + g_frame_count%3 + 0x40;
5. **always**: DANTE, frame = u16@+0x14 + 0x20, mode 300.

Spawn (FUN_0040cca0, decomp @0x41dc5a family) zero-fills the record
then sets facing = 0xFFFF, u16@+0x16 = 0xFFFF, i32@+0x88 = 0,
u16@+0x18 = RandA()&3, pos = tile*0x2000+0xF00, z = level*0x20−1 —
so **a spawned robot draws exactly two sprites: DANTE[anim] and
DANTE[0x20]** (u16@+0x14 stays 0). After the loop the low 2 bits of
u16@+0x18 cycle +1 &3 (only observable through the +0x88-gated
sprite). `_DAT_004edb88 != 0` additionally queues ROBNUMS
(DAT_0046cdb0) name-plate digits at `sx + i32[0x4e44c8 + c] + 6*i`
for name chars < 0x41 (multiplayer; not modeled). Platform
(0x4eb638) and effects (0x4cf638, the FUN_00401e39 draw_IMG codec
family) loops follow the same sx/sy form — out of scope for the P4
robot overlay.

## 6. Sprite banks staged by FUN_0041df10 [verified] (context)

DANTE/SCANNER/BLOWUP(/BLOWUPG)/WEAPONS/SHRIKE/REAPER/SMOKE/TELEPORT/
NUMBERS/FLAGS/VICERA/DEBRIS/SHIELD/ROBNUMS .BIN + TABLE.BIN +
DIGITS/SMOKER/HUMANS/IDIOTGFX + palettes TXPAL1/GAMEPAL/DARKPAL —
entity/overlay banks + game palette (GAMEPAL 0x4edbf8; 7c's 0x302-B
copy target). The 0x64000 tile buffer is NOT cleared by init_tiles
beyond the rep-stos in init_tiles itself (0x64000 bytes — full clear
each mission start; per-frame the terrain pass overwrites everything
the present window reads).

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
  camera 0/full zoom for the crop gate.

## 8. Open items

1. Producers of type-DB bytes +0x18/+0x1a/+0x1b/+0x1c (static frame,
   height bias, anim window). Zero-filled on ZONEA → no effect on the
   P4 corpus gate; find the writer (editor? BIN-side fixup?) later.
2. `u32[0x4dd444]` remap-table set + `u32[0x456ca8]` 16-entry anim
   sequence: producers unfound (likely BIN/TABLE.BIN parse);
   ZONEA/M1 LNK identity cells make frames irrelevant there.
3. ~~FUN_00403938's entity loops~~ **CLOSED 2026-08-21**: the robot
   entity loop + FUN_0040798e/0179b enqueue/flush are decoded (§5b–§5d)
   and wired into bedlam-render. Remaining out-of-scope tail: the
   platform loop (0x4eb638, bank DAT_0046af54), the effects loop
   (0x4cf638 — the separate draw_IMG/FUN_00401e39 codec family), and
   the ROBNUMS name plates (§5d).
4. BIN u32[bank+0] directory header word / sprite count sanity.
